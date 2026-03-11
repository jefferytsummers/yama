//! VLM Video Inference Service for NVIDIA Jetson
//!
//! Provides video analysis using Vision Language Models on Jetson hardware.
//! Uses Triton Inference Server with TensorRT-LLM for optimized inference.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                    VlmInferenceService                           │
//! │  ┌────────────────────────────────────────────────────────────┐  │
//! │  │  TritonVlmBackend                                          │  │
//! │  │  - gRPC client to Triton Server                            │  │
//! │  │  - System shared memory for zero-copy                      │  │
//! │  │  - TensorRT-LLM request formatting                         │  │
//! │  └────────────────────────────────────────────────────────────┘  │
//! │                              │                                    │
//! │                              ▼                                    │
//! │  ┌────────────────────────────────────────────────────────────┐  │
//! │  │  Triton Inference Server (Docker)                          │  │
//! │  │  - TensorRT-LLM engine                                     │  │
//! │  │  - Qwen2.5-VL model                                        │  │
//! │  └────────────────────────────────────────────────────────────┘  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use yama_platform_traits::VlmBackend;

mod frame_extractor;
mod triton_backend;
mod triton_client;

pub use frame_extractor::{ExtractionConfig, ExtractedFrame, FrameExtractor, VideoInfo};
pub use triton_backend::{TritonVlmBackend, TritonVlmConfig};
pub use triton_client::{TritonClient, TritonClientConfig};

/// Inference job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Extracting,
    Inferring,
    Completed,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Extracting => write!(f, "extracting"),
            Self::Inferring => write!(f, "inferring"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// A chunk of inference results with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceChunk {
    pub frame_number: u64,
    pub timestamp_ms: u64,
    pub text: String,
}

/// Inference job state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceJob {
    pub job_id: String,
    pub status: JobStatus,
    pub progress_percent: f32,
    pub current_frame: u64,
    pub total_frames: u64,
    pub results: Vec<InferenceChunk>,
    pub error: Option<String>,
    pub model: String,
    pub prompt: String,
    #[serde(skip)]
    pub video_path: Option<PathBuf>,
}

impl InferenceJob {
    pub fn new(job_id: String, model: String, prompt: String) -> Self {
        Self {
            job_id,
            status: JobStatus::Queued,
            progress_percent: 0.0,
            current_frame: 0,
            total_frames: 0,
            results: Vec::new(),
            error: None,
            model,
            prompt,
            video_path: None,
        }
    }
}

/// Upload metadata for tracking uploaded files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInfo {
    pub upload_id: String,
    pub filename: String,
    pub size: u64,
    pub path: PathBuf,
    pub content_type: String,
}

/// VLM model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub max_frames: u32,
    pub supports_streaming: bool,
}

/// Backend type for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Mock backend for development/testing.
    Mock,
    /// Triton Inference Server backend (gRPC).
    #[default]
    Triton,
}

/// Configuration for the VLM inference service.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VlmInferenceConfig {
    /// Directory for temporary uploads.
    #[serde(default = "default_upload_dir")]
    pub upload_dir: PathBuf,
    /// Maximum upload size in bytes (default: 500MB).
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: u64,
    /// Frame extraction interval in milliseconds.
    #[serde(default = "default_frame_interval_ms")]
    pub frame_interval_ms: u64,
    /// Maximum frames to extract per video.
    #[serde(default = "default_max_frames")]
    pub max_frames: u32,
    /// Backend type to use.
    #[serde(default)]
    pub backend: BackendType,
    /// Triton server configuration.
    #[serde(default)]
    pub triton: TritonVlmConfig,
}

fn default_upload_dir() -> PathBuf {
    std::env::temp_dir().join("yama-inference")
}

fn default_max_upload_size() -> u64 {
    500 * 1024 * 1024 // 500MB
}

fn default_frame_interval_ms() -> u64 {
    1000 // 1 frame per second
}

fn default_max_frames() -> u32 {
    300 // 5 minutes at 1fps
}

impl Default for VlmInferenceConfig {
    fn default() -> Self {
        Self {
            upload_dir: default_upload_dir(),
            max_upload_size: default_max_upload_size(),
            frame_interval_ms: default_frame_interval_ms(),
            max_frames: default_max_frames(),
            backend: BackendType::default(),
            triton: TritonVlmConfig::default(),
        }
    }
}

/// VLM Inference Service for Jetson.
///
/// Manages video uploads, inference jobs, and model interactions.
/// Uses Triton Inference Server for VLM inference.
pub struct VlmInferenceService {
    config: VlmInferenceConfig,
    jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
    uploads: Arc<RwLock<HashMap<String, UploadInfo>>>,
    models: Vec<VlmModelInfo>,
    backend: Arc<dyn VlmBackend>,
}

impl VlmInferenceService {
    /// Create a new VLM inference service.
    pub async fn new(config: VlmInferenceConfig) -> Result<Self> {
        let backend: Arc<dyn VlmBackend> = match config.backend {
            BackendType::Mock => {
                info!("Using mock inference backend");
                Arc::new(MockBackend::new())
            }
            BackendType::Triton => {
                info!("Using Triton inference backend");
                Arc::new(
                    TritonVlmBackend::new(config.triton.clone())
                        .await
                        .context("Failed to create Triton backend")?,
                )
            }
        };

        Self::with_backend(config, backend).await
    }

    /// Create a new VLM inference service with a specific backend.
    pub async fn with_backend(
        config: VlmInferenceConfig,
        backend: Arc<dyn VlmBackend>,
    ) -> Result<Self> {
        // Ensure upload directory exists
        tokio::fs::create_dir_all(&config.upload_dir)
            .await
            .with_context(|| format!("Failed to create upload dir: {:?}", config.upload_dir))?;

        info!(
            "VLM inference service initialized (backend={})",
            backend.name()
        );

        // Available models
        let models = vec![
            VlmModelInfo {
                id: "qwen2.5-vl-7b".to_string(),
                name: "Qwen2.5-VL 7B".to_string(),
                description: "High-quality video understanding model".to_string(),
                max_frames: 300,
                supports_streaming: true,
            },
            VlmModelInfo {
                id: "qwen2.5-vl-3b".to_string(),
                name: "Qwen2.5-VL 3B".to_string(),
                description: "Fast video understanding model".to_string(),
                max_frames: 500,
                supports_streaming: true,
            },
        ];

        // Warm up the backend
        if let Err(e) = backend.warmup().await {
            warn!("Backend warmup failed: {}", e);
        }

        Ok(Self {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            uploads: Arc::new(RwLock::new(HashMap::new())),
            models,
            backend,
        })
    }

    /// Get service configuration.
    pub fn config(&self) -> &VlmInferenceConfig {
        &self.config
    }

    /// Get the backend name.
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Get the VLM backend for direct access.
    pub fn backend(&self) -> &Arc<dyn VlmBackend> {
        &self.backend
    }

    /// List available VLM models.
    pub fn list_models(&self) -> &[VlmModelInfo] {
        &self.models
    }

    /// Register an upload.
    pub async fn register_upload(&self, info: UploadInfo) {
        let mut uploads = self.uploads.write().await;
        uploads.insert(info.upload_id.clone(), info);
    }

    /// Get upload info.
    pub async fn get_upload(&self, upload_id: &str) -> Option<UploadInfo> {
        let uploads = self.uploads.read().await;
        uploads.get(upload_id).cloned()
    }

    /// Create a new inference job.
    pub async fn create_job(&self, model: String, prompt: String) -> String {
        let job_id = Uuid::new_v4().to_string();
        let job = InferenceJob::new(job_id.clone(), model, prompt);

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), job);

        job_id
    }

    /// Get a job by ID.
    pub async fn get_job(&self, job_id: &str) -> Option<InferenceJob> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// List all jobs.
    pub async fn list_jobs(&self) -> Vec<InferenceJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Update job status.
    pub async fn update_job_status(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status;
        }
    }

    /// Update job progress.
    pub async fn update_job_progress(
        &self,
        job_id: &str,
        current_frame: u64,
        total_frames: u64,
        chunk: Option<InferenceChunk>,
    ) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.current_frame = current_frame;
            job.total_frames = total_frames;
            job.progress_percent = if total_frames > 0 {
                (current_frame as f32 / total_frames as f32) * 100.0
            } else {
                0.0
            };
            if let Some(c) = chunk {
                job.results.push(c);
            }
        }
    }

    /// Set job error.
    pub async fn set_job_error(&self, job_id: &str, error: String) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.error = Some(error);
        }
    }

    /// Check if the backend is ready.
    pub async fn is_ready(&self) -> bool {
        self.backend.is_ready().await
    }

    /// Delete a job.
    pub async fn delete_job(&self, job_id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        jobs.remove(job_id).is_some()
    }

    /// Clean up old uploads and jobs.
    pub async fn cleanup(&self, max_age: Duration) {
        debug!("Cleanup triggered with max_age: {:?}", max_age);
        // TODO: Implement cleanup of old files and completed jobs
    }
}

/// Mock backend for testing.
pub struct MockBackend {
    capabilities: yama_platform_traits::VlmCapabilities,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            capabilities: yama_platform_traits::VlmCapabilities {
                name: "mock".to_string(),
                model: "mock-model".to_string(),
                max_image_width: 4096,
                max_image_height: 4096,
                max_tokens: 4096,
                supported_formats: vec![yama_platform_traits::VlmPixelFormat::Jpeg],
                supports_batch: false,
                supports_streaming: false,
                supports_shared_memory: false,
                accelerator: "CPU".to_string(),
                estimated_throughput: 1.0,
            },
        }
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl VlmBackend for MockBackend {
    fn capabilities(&self) -> &yama_platform_traits::VlmCapabilities {
        &self.capabilities
    }

    async fn analyze(
        &self,
        request: yama_platform_traits::VlmRequest,
    ) -> yama_platform_traits::PlatformResult<yama_platform_traits::VlmResponse> {
        // Simulate some processing time
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(yama_platform_traits::VlmResponse {
            request_id: request.request_id,
            analysis: "Mock analysis: This is a test image showing typical video content."
                .to_string(),
            inference_time_ms: 100.0,
            tokens_generated: 15,
            model: "mock-model".to_string(),
        })
    }

    async fn is_ready(&self) -> bool {
        true
    }

    async fn warmup(&self) -> yama_platform_traits::PlatformResult<()> {
        info!("Mock backend warmup (no-op)");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend() {
        let backend = MockBackend::new();
        assert_eq!(backend.name(), "mock");
        assert!(backend.is_ready().await);
    }

    #[tokio::test]
    async fn test_create_job() {
        let config = VlmInferenceConfig {
            backend: BackendType::Mock,
            ..Default::default()
        };

        let service = VlmInferenceService::new(config).await.unwrap();
        let job_id = service
            .create_job("qwen2.5-vl-7b".to_string(), "Describe this video".to_string())
            .await;

        let job = service.get_job(&job_id).await;
        assert!(job.is_some());
        assert_eq!(job.unwrap().status, JobStatus::Queued);
    }
}
