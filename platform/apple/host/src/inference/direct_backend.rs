//! Direct VLM backend using in-process mistral.rs.
//!
//! Loads the VLM model directly into the host process, bypassing
//! all event bus complexity. Uses Metal MPS for GPU acceleration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use image::DynamicImage;
use mistralrs::{Device, IsqType, Model, TextMessageRole, VisionMessages, VisionModelBuilder};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use super::backend::{InferenceBackend, update_job_progress, update_job_status};
use super::frame_extractor::{ExtractionConfig, FrameExtractor};
use super::{InferenceChunk, InferenceJob, JobStatus, VlmInferenceConfig};

/// Direct VLM backend configuration.
#[derive(Debug, Clone)]
pub struct DirectVlmConfig {
    /// HuggingFace model ID or local path.
    pub model_id: String,
    /// In-situ quantization level (Q4K, Q8_0, F16).
    pub isq: String,
    /// Device to run on (metal, cpu).
    pub device: String,
    /// Default temperature for generation.
    pub temperature: f32,
    /// Default max tokens for generation.
    pub max_tokens: u32,
}

impl Default for DirectVlmConfig {
    fn default() -> Self {
        Self {
            model_id: std::env::var("VLM_MODEL_ID")
                .unwrap_or_else(|_| "Qwen/Qwen2.5-VL-3B-Instruct".to_string()),
            isq: std::env::var("VLM_ISQ").unwrap_or_else(|_| "Q4K".to_string()),
            device: "metal".to_string(),
            temperature: 0.7,
            max_tokens: 512,
        }
    }
}

/// Direct in-process VLM backend.
///
/// Loads mistral.rs directly into the host process for lowest-latency
/// inference. Model is lazy-loaded on first inference request.
pub struct DirectVlmBackend {
    /// The loaded model (lazy initialization).
    model: Arc<RwLock<Option<Model>>>,
    /// Model configuration.
    vlm_config: DirectVlmConfig,
    /// Frame extraction configuration.
    extraction_config: ExtractionConfig,
    /// Whether model loading has been attempted.
    load_attempted: Arc<RwLock<bool>>,
}

impl DirectVlmBackend {
    /// Create a new direct VLM backend.
    pub fn new(config: &VlmInferenceConfig) -> Self {
        let extraction_config = ExtractionConfig {
            interval_ms: config.frame_interval_ms,
            max_frames: config.max_frames,
            target_width: None,
            target_height: None,
            jpeg_quality: 85,
        };

        Self {
            model: Arc::new(RwLock::new(None)),
            vlm_config: DirectVlmConfig::default(),
            extraction_config,
            load_attempted: Arc::new(RwLock::new(false)),
        }
    }

    /// Create with custom VLM configuration.
    pub fn with_vlm_config(config: &VlmInferenceConfig, vlm_config: DirectVlmConfig) -> Self {
        let extraction_config = ExtractionConfig {
            interval_ms: config.frame_interval_ms,
            max_frames: config.max_frames,
            target_width: None,
            target_height: None,
            jpeg_quality: 85,
        };

        Self {
            model: Arc::new(RwLock::new(None)),
            vlm_config,
            extraction_config,
            load_attempted: Arc::new(RwLock::new(false)),
        }
    }

    /// Ensure the model is loaded.
    #[instrument(skip(self))]
    async fn ensure_loaded(&self) -> Result<()> {
        // Check if already loaded
        {
            let model_guard = self.model.read().await;
            if model_guard.is_some() {
                return Ok(());
            }
        }

        // Check if we already tried and failed
        {
            let attempted = self.load_attempted.read().await;
            if *attempted {
                anyhow::bail!("Model loading previously failed");
            }
        }

        // Mark as attempted
        {
            let mut attempted = self.load_attempted.write().await;
            *attempted = true;
        }

        info!("Loading VLM model: {}", self.vlm_config.model_id);
        let start = Instant::now();

        // Parse ISQ type
        let isq_type = match self.vlm_config.isq.to_uppercase().as_str() {
            "Q4K" | "Q4_K" => Some(IsqType::Q4K),
            "Q8_0" | "Q8" => Some(IsqType::Q8_0),
            "F16" | "FP16" => None,
            "BF16" => None,
            _ => {
                warn!("Unknown ISQ type '{}', using Q4K", self.vlm_config.isq);
                Some(IsqType::Q4K)
            }
        };

        // Create Metal device
        let device = match self.vlm_config.device.to_lowercase().as_str() {
            "metal" | "mps" => Device::new_metal(0).context("Failed to create Metal device")?,
            "cpu" => Device::Cpu,
            _ => {
                warn!("Unknown device '{}', using Metal", self.vlm_config.device);
                Device::new_metal(0).context("Failed to create Metal device")?
            }
        };

        // Build the vision model
        let mut builder = VisionModelBuilder::new(&self.vlm_config.model_id)
            .with_logging()
            .with_device(device);

        if let Some(isq) = isq_type {
            builder = builder.with_isq(isq);
        }

        let model = builder.build().await.context("Failed to build VLM model")?;

        let load_time = start.elapsed();
        info!("VLM model loaded in {:.1}s", load_time.as_secs_f32());

        // Store the model
        {
            let mut model_guard = self.model.write().await;
            *model_guard = Some(model);
        }

        Ok(())
    }

    /// Analyze a single image.
    #[instrument(skip(self, image), fields(prompt_len = prompt.len()))]
    async fn analyze_image(&self, image: DynamicImage, prompt: &str) -> Result<String> {
        self.ensure_loaded().await?;

        let model_guard = self.model.read().await;
        let model = model_guard.as_ref().context("Model not loaded")?;

        let start = Instant::now();

        // Build vision messages
        let images = vec![image];
        debug!("Building VisionMessages with prompt: {}", prompt);
        let messages = VisionMessages::new().add_image_message(
            TextMessageRole::User,
            prompt.to_string(),
            images,
            model,
        )?;
        debug!("VisionMessages built successfully");

        // Run inference
        let response = model
            .send_chat_request(messages)
            .await
            .context("VLM inference failed")?;

        let inference_time = start.elapsed();

        // Extract response text
        let text = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens = response.usage.completion_tokens;

        debug!(
            "Inference complete: {} tokens in {:.2}ms ({:.1} tok/s)",
            tokens,
            inference_time.as_secs_f32() * 1000.0,
            tokens as f32 / inference_time.as_secs_f32()
        );

        Ok(text)
    }
}

#[async_trait]
impl InferenceBackend for DirectVlmBackend {
    async fn run_inference(
        &self,
        jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
        job_id: String,
        job: InferenceJob,
    ) -> Result<()> {
        info!("Starting direct VLM inference for job {}", job_id);

        // Get video path
        let video_path = job.video_path.as_ref().context("No video path set")?;

        // Extract frames
        update_job_status(&jobs, &job_id, JobStatus::Extracting).await;

        let extractor = FrameExtractor::new(self.extraction_config.clone());
        let frames = extractor
            .extract_frames(video_path)
            .await
            .context("Failed to extract frames")?;

        let total_frames = frames.len() as u64;
        {
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.total_frames = total_frames;
            }
        }

        info!("Extracted {} frames for job {}", total_frames, job_id);

        if total_frames == 0 {
            warn!("No frames extracted for job {}", job_id);
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.status = JobStatus::Completed;
                j.progress_percent = 100.0;
            }
            return Ok(());
        }

        // Ensure model is loaded before starting inference
        if let Err(e) = self.ensure_loaded().await {
            error!("Failed to load VLM model: {}", e);
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.status = JobStatus::Failed;
                j.error = Some(format!("Failed to load VLM model: {}", e));
            }
            return Err(e);
        }

        // Update to inferring status
        update_job_status(&jobs, &job_id, JobStatus::Inferring).await;

        // Process frames sequentially (VLM is already GPU-bound)
        let mut completed = 0u64;
        let mut failed = 0u64;

        for frame in frames {
            // Skip empty frames
            if frame.data.is_empty() {
                warn!("Frame {} has empty data, skipping", frame.number);
                failed += 1;
                continue;
            }

            // Decode JPEG to DynamicImage
            let image = match image::load_from_memory(&frame.data) {
                Ok(img) => {
                    info!(
                        "Frame {} decoded: {}x{} {:?}, JPEG size: {} bytes",
                        frame.number,
                        img.width(),
                        img.height(),
                        img.color(),
                        frame.data.len()
                    );
                    img
                }
                Err(e) => {
                    error!("Failed to decode frame {}: {}", frame.number, e);
                    failed += 1;
                    continue;
                }
            };

            // Run inference
            match self.analyze_image(image, &job.prompt).await {
                Ok(text) => {
                    let chunk = InferenceChunk {
                        frame_number: frame.number,
                        timestamp_ms: frame.timestamp_ms,
                        text,
                    };

                    completed += 1;
                    update_job_progress(&jobs, &job_id, completed, total_frames, Some(chunk)).await;

                    debug!(
                        "Job {} progress: {}/{} frames",
                        job_id, completed, total_frames
                    );
                }
                Err(e) => {
                    error!("Frame {} inference failed: {}", frame.number, e);
                    failed += 1;

                    // If too many failures, abort
                    if failed > total_frames / 2 {
                        let mut jobs_guard = jobs.write().await;
                        if let Some(j) = jobs_guard.get_mut(&job_id) {
                            j.status = JobStatus::Failed;
                            j.error = Some(format!(
                                "Too many frame failures ({}/{})",
                                failed, total_frames
                            ));
                        }
                        anyhow::bail!("Too many frame failures");
                    }
                }
            }
        }

        // Mark as completed
        {
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.status = JobStatus::Completed;
                j.progress_percent = 100.0;
            }
        }

        info!(
            "Direct VLM inference completed for job {}: {}/{} frames succeeded",
            job_id, completed, total_frames
        );

        Ok(())
    }

    fn name(&self) -> &'static str {
        "direct"
    }

    async fn is_ready(&self) -> bool {
        let model_guard = self.model.read().await;
        model_guard.is_some()
    }

    async fn warmup(&self) -> Result<()> {
        info!("Warming up direct VLM backend...");

        // Load the model
        self.ensure_loaded().await?;

        // Run a minimal inference to warm up GPU kernels
        let test_image = DynamicImage::new_rgb8(64, 64);

        if let Err(e) = self.analyze_image(test_image, "What is this?").await {
            warn!("Warmup inference failed (non-fatal): {}", e);
        }

        info!("Direct VLM backend warmup complete");
        Ok(())
    }
}

/// Create a direct VLM backend.
pub fn create_direct_backend(config: &VlmInferenceConfig) -> DirectVlmBackend {
    DirectVlmBackend::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DirectVlmConfig::default();
        assert!(config.model_id.contains("Qwen"));
        assert_eq!(config.isq, "Q4K");
        assert_eq!(config.device, "metal");
    }

    #[tokio::test]
    async fn test_backend_creation() {
        let config = VlmInferenceConfig::default();
        let backend = DirectVlmBackend::new(&config);
        assert_eq!(backend.name(), "direct");
        assert!(!backend.is_ready().await);
    }
}
