//! VLM (Vision Language Model) Backend Trait
//!
//! Provides a unified abstraction for VLM inference across platforms:
//! - Apple: mistral.rs + Metal MPS (DirectVlmBackend)
//! - Apple: Event bus to VLM container (EventBusBackend)
//! - Jetson: Triton Inference Server + TensorRT-LLM (TritonVlmBackend)
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                    VlmService (Unified)                          │
//! │  ┌────────────────────────────────────────────────────────────┐  │
//! │  │  VlmBackend Trait                                          │  │
//! │  │  - async analyze(VlmRequest) -> VlmResponse                │  │
//! │  │  - async analyze_batch(VlmBatchRequest) -> VlmBatchResult  │  │
//! │  │  - fn capabilities() -> VlmCapabilities                    │  │
//! │  └────────────────────────────────────────────────────────────┘  │
//! │                              │                                    │
//! │      ┌───────────────────────┼───────────────────────┐           │
//! │      ▼                       ▼                       ▼           │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐   │
//! │  │DirectBackend │    │EventBusBackend│   │TritonVlmBackend │   │
//! │  │(Apple Metal) │    │(Container IPC)│   │(Jetson gRPC)    │   │
//! │  └──────────────┘    └──────────────┘    └──────────────────┘   │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::PlatformResult;

/// Image source for VLM analysis.
#[derive(Debug, Clone)]
pub enum VlmImageSource {
    /// JPEG-encoded image data.
    Jpeg(Vec<u8>),
    /// PNG-encoded image data.
    Png(Vec<u8>),
    /// Raw RGB pixel data (width, height, data).
    RawRgb {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Raw RGB pixel data (3 bytes per pixel).
        data: Vec<u8>,
    },
    /// Raw RGBA pixel data (width, height, data).
    RawRgba {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Raw RGBA pixel data (4 bytes per pixel).
        data: Vec<u8>,
    },
    /// NV12 YUV data (common hardware decode format).
    Nv12 {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// NV12 plane data (Y plane followed by interleaved UV).
        data: Vec<u8>,
    },
    /// Reference to a shared memory segment (for zero-copy on Jetson).
    SharedMemory {
        /// Shared memory segment name.
        name: String,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Pixel format identifier.
        format: VlmPixelFormat,
        /// Offset into the shared memory segment.
        offset: u64,
        /// Size of the image data in bytes.
        size: u64,
    },
}

impl VlmImageSource {
    /// Get the dimensions of the image if known.
    #[must_use]
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match self {
            Self::Jpeg(_) | Self::Png(_) => None, // Need to decode to get dimensions
            Self::RawRgb { width, height, .. }
            | Self::RawRgba { width, height, .. }
            | Self::Nv12 { width, height, .. }
            | Self::SharedMemory { width, height, .. } => Some((*width, *height)),
        }
    }

    /// Get the size of the image data in bytes.
    #[must_use]
    pub fn data_size(&self) -> usize {
        match self {
            Self::Jpeg(data) | Self::Png(data) => data.len(),
            Self::RawRgb { data, .. }
            | Self::RawRgba { data, .. }
            | Self::Nv12 { data, .. } => data.len(),
            Self::SharedMemory { size, .. } => *size as usize,
        }
    }
}

/// Pixel format for raw image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VlmPixelFormat {
    /// RGB 8-bit per channel.
    Rgb8,
    /// RGBA 8-bit per channel.
    Rgba8,
    /// BGRA 8-bit per channel.
    Bgra8,
    /// NV12 YUV format (hardware decode native).
    Nv12,
    /// I420 YUV format.
    I420,
    /// JPEG compressed.
    Jpeg,
    /// PNG compressed.
    Png,
}

/// Generation parameters for VLM inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmGenerationParams {
    /// Maximum number of tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature (0.0 = deterministic, 1.0+ = creative).
    pub temperature: f32,
    /// Top-p (nucleus) sampling threshold.
    pub top_p: f32,
    /// Top-k sampling (0 = disabled).
    pub top_k: u32,
    /// Repetition penalty (1.0 = no penalty).
    pub repetition_penalty: f32,
}

impl Default for VlmGenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 0,
            repetition_penalty: 1.0,
        }
    }
}

/// Request for single-image VLM analysis.
#[derive(Debug, Clone)]
pub struct VlmRequest {
    /// Unique request identifier for correlation.
    pub request_id: String,
    /// Analysis prompt describing what to analyze.
    pub prompt: String,
    /// Image data to analyze.
    pub image: VlmImageSource,
    /// Generation parameters.
    pub params: VlmGenerationParams,
}

impl VlmRequest {
    /// Create a new VLM request with default parameters.
    #[must_use]
    pub fn new(request_id: impl Into<String>, prompt: impl Into<String>, image: VlmImageSource) -> Self {
        Self {
            request_id: request_id.into(),
            prompt: prompt.into(),
            image,
            params: VlmGenerationParams::default(),
        }
    }

    /// Set generation parameters.
    #[must_use]
    pub fn with_params(mut self, params: VlmGenerationParams) -> Self {
        self.params = params;
        self
    }

    /// Set maximum tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.params.max_tokens = max_tokens;
        self
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.params.temperature = temperature;
        self
    }
}

/// Response from VLM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmResponse {
    /// Request ID this response corresponds to.
    pub request_id: String,
    /// Analysis text generated by the VLM.
    pub analysis: String,
    /// Inference time in milliseconds.
    pub inference_time_ms: f32,
    /// Number of tokens generated.
    pub tokens_generated: u32,
    /// Model used for inference.
    pub model: String,
}

impl VlmResponse {
    /// Create a new VLM response.
    #[must_use]
    pub fn new(
        request_id: impl Into<String>,
        analysis: impl Into<String>,
        inference_time_ms: f32,
        tokens_generated: u32,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            analysis: analysis.into(),
            inference_time_ms,
            tokens_generated,
            model: String::new(),
        }
    }

    /// Set the model name.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// Request for batch VLM analysis (multiple frames/images).
#[derive(Debug, Clone)]
pub struct VlmBatchRequest {
    /// Unique batch request identifier.
    pub request_id: String,
    /// Individual image requests.
    pub requests: Vec<VlmRequest>,
    /// Whether to process in parallel (if backend supports).
    pub parallel: bool,
}

impl VlmBatchRequest {
    /// Create a new batch request.
    #[must_use]
    pub fn new(request_id: impl Into<String>, requests: Vec<VlmRequest>) -> Self {
        Self {
            request_id: request_id.into(),
            requests,
            parallel: true,
        }
    }

    /// Set whether to process in parallel.
    #[must_use]
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
}

/// Result from batch VLM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmBatchResult {
    /// Batch request ID.
    pub request_id: String,
    /// Individual responses (in order of requests).
    pub responses: Vec<VlmResponse>,
    /// Total processing time in milliseconds.
    pub total_time_ms: f32,
    /// Number of successful analyses.
    pub successful: u32,
    /// Number of failed analyses.
    pub failed: u32,
    /// Errors for failed requests (request_id -> error message).
    pub errors: std::collections::HashMap<String, String>,
}

impl VlmBatchResult {
    /// Create a new batch result.
    #[must_use]
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            responses: Vec::new(),
            total_time_ms: 0.0,
            successful: 0,
            failed: 0,
            errors: std::collections::HashMap::new(),
        }
    }

    /// Add a successful response.
    pub fn add_response(&mut self, response: VlmResponse) {
        self.responses.push(response);
        self.successful += 1;
    }

    /// Add a failed request.
    pub fn add_error(&mut self, request_id: impl Into<String>, error: impl Into<String>) {
        self.errors.insert(request_id.into(), error.into());
        self.failed += 1;
    }
}

/// VLM backend capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmCapabilities {
    /// Backend name (e.g., "direct", "event_bus", "triton").
    pub name: String,
    /// Model name or identifier.
    pub model: String,
    /// Maximum image width supported.
    pub max_image_width: u32,
    /// Maximum image height supported.
    pub max_image_height: u32,
    /// Maximum tokens that can be generated.
    pub max_tokens: u32,
    /// Supported pixel formats for input.
    pub supported_formats: Vec<VlmPixelFormat>,
    /// Whether batch processing is supported.
    pub supports_batch: bool,
    /// Whether streaming output is supported.
    pub supports_streaming: bool,
    /// Whether shared memory input is supported (for zero-copy).
    pub supports_shared_memory: bool,
    /// Backend-specific hardware accelerator.
    pub accelerator: String,
    /// Estimated tokens per second throughput.
    pub estimated_throughput: f32,
}

impl VlmCapabilities {
    /// Create capabilities for a direct Metal backend.
    #[must_use]
    pub fn metal(model: impl Into<String>) -> Self {
        Self {
            name: "direct".to_string(),
            model: model.into(),
            max_image_width: 4096,
            max_image_height: 4096,
            max_tokens: 4096,
            supported_formats: vec![
                VlmPixelFormat::Jpeg,
                VlmPixelFormat::Png,
                VlmPixelFormat::Rgb8,
                VlmPixelFormat::Rgba8,
            ],
            supports_batch: false,
            supports_streaming: false,
            supports_shared_memory: false,
            accelerator: "Metal MPS".to_string(),
            estimated_throughput: 30.0,
        }
    }

    /// Create capabilities for a Triton backend.
    #[must_use]
    pub fn triton(model: impl Into<String>) -> Self {
        Self {
            name: "triton".to_string(),
            model: model.into(),
            max_image_width: 4096,
            max_image_height: 4096,
            max_tokens: 4096,
            supported_formats: vec![
                VlmPixelFormat::Jpeg,
                VlmPixelFormat::Nv12,
                VlmPixelFormat::Rgb8,
            ],
            supports_batch: true,
            supports_streaming: true,
            supports_shared_memory: true,
            accelerator: "TensorRT-LLM".to_string(),
            estimated_throughput: 50.0,
        }
    }

    /// Create capabilities for an event bus backend.
    #[must_use]
    pub fn event_bus(model: impl Into<String>) -> Self {
        Self {
            name: "event_bus".to_string(),
            model: model.into(),
            max_image_width: 4096,
            max_image_height: 4096,
            max_tokens: 4096,
            supported_formats: vec![VlmPixelFormat::Jpeg],
            supports_batch: true,
            supports_streaming: false,
            supports_shared_memory: false,
            accelerator: "Container".to_string(),
            estimated_throughput: 20.0,
        }
    }
}

/// VLM backend trait for unified inference across platforms.
///
/// This trait abstracts the VLM inference implementation, allowing:
/// - Direct in-process inference (Apple Metal/MPS)
/// - Event bus communication with VLM container
/// - Triton Inference Server gRPC client (Jetson)
///
/// # Example
///
/// ```ignore
/// use yama_platform_traits::vlm::{VlmBackend, VlmRequest, VlmImageSource};
///
/// async fn analyze_frame(backend: &dyn VlmBackend, jpeg_data: Vec<u8>) {
///     let request = VlmRequest::new(
///         "req-1",
///         "Describe what you see in this image",
///         VlmImageSource::Jpeg(jpeg_data),
///     );
///
///     let response = backend.analyze(request).await.unwrap();
///     println!("Analysis: {}", response.analysis);
/// }
/// ```
#[async_trait]
pub trait VlmBackend: Send + Sync {
    /// Get backend capabilities.
    fn capabilities(&self) -> &VlmCapabilities;

    /// Analyze a single image.
    ///
    /// # Arguments
    ///
    /// * `request` - The analysis request containing image and prompt.
    ///
    /// # Returns
    ///
    /// The analysis response with generated text.
    async fn analyze(&self, request: VlmRequest) -> PlatformResult<VlmResponse>;

    /// Analyze multiple images in batch.
    ///
    /// Default implementation processes requests sequentially.
    /// Backends that support parallel processing should override this.
    ///
    /// # Arguments
    ///
    /// * `request` - The batch request containing multiple images.
    ///
    /// # Returns
    ///
    /// Batch result with all responses and error information.
    async fn analyze_batch(&self, request: VlmBatchRequest) -> PlatformResult<VlmBatchResult> {
        let start = std::time::Instant::now();
        let mut result = VlmBatchResult::new(&request.request_id);

        for req in request.requests {
            let req_id = req.request_id.clone();
            match self.analyze(req).await {
                Ok(response) => result.add_response(response),
                Err(e) => result.add_error(req_id, e.to_string()),
            }
        }

        result.total_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        Ok(result)
    }

    /// Check if the backend is ready to accept requests.
    ///
    /// This may check model loading status, connection health, etc.
    async fn is_ready(&self) -> bool;

    /// Warm up the backend (load model, establish connections, etc.).
    ///
    /// This should be called before first use to ensure low latency
    /// on the first real request.
    async fn warmup(&self) -> PlatformResult<()>;

    /// Get the backend name for logging/diagnostics.
    fn name(&self) -> &'static str;

    /// Shutdown the backend and release resources.
    ///
    /// Default implementation does nothing. Override if cleanup is needed.
    async fn shutdown(&self) -> PlatformResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlm_request_builder() {
        let request = VlmRequest::new(
            "test-1",
            "Describe this image",
            VlmImageSource::Jpeg(vec![0xFF, 0xD8, 0xFF]),
        )
        .with_max_tokens(256)
        .with_temperature(0.5);

        assert_eq!(request.request_id, "test-1");
        assert_eq!(request.params.max_tokens, 256);
        assert!((request.params.temperature - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vlm_response() {
        let response = VlmResponse::new("req-1", "A cat sitting on a mat", 150.0, 12)
            .with_model("qwen2.5-vl-3b");

        assert_eq!(response.request_id, "req-1");
        assert_eq!(response.tokens_generated, 12);
        assert_eq!(response.model, "qwen2.5-vl-3b");
    }

    #[test]
    fn test_vlm_batch_result() {
        let mut result = VlmBatchResult::new("batch-1");
        result.add_response(VlmResponse::new("req-1", "Analysis 1", 100.0, 10));
        result.add_response(VlmResponse::new("req-2", "Analysis 2", 120.0, 15));
        result.add_error("req-3", "Timeout");

        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 1);
        assert_eq!(result.responses.len(), 2);
        assert!(result.errors.contains_key("req-3"));
    }

    #[test]
    fn test_image_source_dimensions() {
        let jpeg = VlmImageSource::Jpeg(vec![]);
        assert!(jpeg.dimensions().is_none());

        let rgb = VlmImageSource::RawRgb {
            width: 640,
            height: 480,
            data: vec![0; 640 * 480 * 3],
        };
        assert_eq!(rgb.dimensions(), Some((640, 480)));
    }

    #[test]
    fn test_capabilities_metal() {
        let caps = VlmCapabilities::metal("qwen2.5-vl-3b");
        assert_eq!(caps.name, "direct");
        assert_eq!(caps.accelerator, "Metal MPS");
        assert!(caps.supported_formats.contains(&VlmPixelFormat::Jpeg));
    }

    #[test]
    fn test_capabilities_triton() {
        let caps = VlmCapabilities::triton("qwen2.5-vl-7b");
        assert_eq!(caps.name, "triton");
        assert!(caps.supports_batch);
        assert!(caps.supports_shared_memory);
    }
}
