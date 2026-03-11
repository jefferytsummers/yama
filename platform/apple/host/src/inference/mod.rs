//! VLM Video Inference Service
//!
//! Provides video analysis using Vision Language Models. Supports:
//! - Single video file upload and analysis
//! - Frame extraction and chunking
//! - Streaming results via event bus
//! - Swappable backends (mock, event bus, direct)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

mod backend;
#[cfg(feature = "direct-vlm")]
mod direct_backend;
mod event_bus_backend;
mod frame_extractor;
mod mock_backend;

pub use backend::InferenceBackend;
#[cfg(feature = "direct-vlm")]
pub use direct_backend::{create_direct_backend, DirectVlmBackend, DirectVlmConfig};
pub use event_bus_backend::{create_event_bus_backend, EventBusBackend};
pub use frame_extractor::{ExtractionConfig, ExtractedFrame, FrameExtractor, VideoInfo};
pub use mock_backend::MockBackend;

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
    #[default]
    Mock,
    /// Event bus backend for VLM container communication.
    EventBus,
    /// Direct in-process VLM backend (requires `direct-vlm` feature and Xcode).
    #[cfg(feature = "direct-vlm")]
    Direct,
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
    /// Enable mock inference (deprecated, use backend field).
    #[serde(default = "default_mock_enabled")]
    pub mock_enabled: bool,
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

fn default_mock_enabled() -> bool {
    true
}

impl Default for VlmInferenceConfig {
    fn default() -> Self {
        Self {
            upload_dir: default_upload_dir(),
            max_upload_size: default_max_upload_size(),
            frame_interval_ms: default_frame_interval_ms(),
            max_frames: default_max_frames(),
            backend: BackendType::default(),
            mock_enabled: default_mock_enabled(),
        }
    }
}

impl VlmInferenceConfig {
    /// Get the effective backend type.
    pub fn effective_backend(&self) -> BackendType {
        // Backend field takes precedence when explicitly set to non-default
        // mock_enabled is deprecated - only use as fallback
        self.backend
    }
}

/// VLM Inference Service.
///
/// Manages video uploads, inference jobs, and model interactions.
/// Uses a pluggable backend for actual inference execution.
pub struct VlmInferenceService {
    config: VlmInferenceConfig,
    jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
    uploads: Arc<RwLock<HashMap<String, UploadInfo>>>,
    models: Vec<VlmModelInfo>,
    backend: Arc<dyn InferenceBackend>,
}

impl VlmInferenceService {
    /// Create a new VLM inference service with the default backend.
    pub async fn new(config: VlmInferenceConfig) -> Result<Self> {
        let backend: Arc<dyn InferenceBackend> = match config.effective_backend() {
            BackendType::Mock => {
                info!("Using mock inference backend");
                Arc::new(MockBackend::new())
            }
            BackendType::EventBus => {
                info!("Using event bus inference backend");
                Arc::new(create_event_bus_backend(&config).await?)
            }
            #[cfg(feature = "direct-vlm")]
            BackendType::Direct => {
                info!("Using direct in-process VLM backend");
                Arc::new(create_direct_backend(&config))
            }
        };

        Self::with_backend(config, backend).await
    }

    /// Create a mock VLM inference service for testing.
    ///
    /// This creates a synchronous mock that doesn't require async or file system access.
    pub fn new_mock() -> Self {
        Self {
            config: VlmInferenceConfig::default(),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            uploads: Arc::new(RwLock::new(HashMap::new())),
            models: vec![VlmModelInfo {
                id: "mock".to_string(),
                name: "Mock Model".to_string(),
                description: "Mock model for testing".to_string(),
                max_frames: 100,
                supports_streaming: false,
            }],
            backend: Arc::new(MockBackend::new()),
        }
    }

    /// Create a new VLM inference service with a specific backend.
    pub async fn with_backend(
        config: VlmInferenceConfig,
        backend: Arc<dyn InferenceBackend>,
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
                id: "vlm-default".to_string(),
                name: "VLM Default".to_string(),
                description: "General-purpose video understanding model".to_string(),
                max_frames: 300,
                supports_streaming: true,
            },
            VlmModelInfo {
                id: "vlm-fast".to_string(),
                name: "VLM Fast".to_string(),
                description: "Optimized for speed with lower accuracy".to_string(),
                max_frames: 100,
                supports_streaming: true,
            },
            VlmModelInfo {
                id: "vlm-detailed".to_string(),
                name: "VLM Detailed".to_string(),
                description: "Maximum detail analysis, slower processing".to_string(),
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

    /// Start inference on a video.
    pub async fn start_inference(&self, job_id: String, upload_id: String) -> Result<()> {
        // Get upload info
        let upload = self
            .get_upload(&upload_id)
            .await
            .context("Upload not found")?;

        // Update job with video path
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.video_path = Some(upload.path.clone());
                job.status = JobStatus::Extracting;
            }
        }

        // Get job info for the async task
        let job = self.get_job(&job_id).await.context("Job not found")?;

        // Clone what we need for the async task
        let jobs = self.jobs.clone();
        let backend = self.backend.clone();

        // Spawn the inference task
        tokio::spawn(async move {
            if let Err(e) = backend.run_inference(jobs, job_id.clone(), job).await {
                error!("Inference failed for job {}: {}", job_id, e);
            }
        });

        Ok(())
    }

    /// Run inference synchronously (for headless mode).
    pub async fn run_inference_sync(&self, job_id: &str, upload_id: &str) -> Result<InferenceJob> {
        // Get upload info
        let upload = self
            .get_upload(upload_id)
            .await
            .context("Upload not found")?;

        // Update job with video path
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.video_path = Some(upload.path.clone());
                job.status = JobStatus::Extracting;
            }
        }

        // Get job info
        let job = self.get_job(job_id).await.context("Job not found")?;

        // Run inference synchronously
        self.backend
            .run_inference(self.jobs.clone(), job_id.to_string(), job)
            .await?;

        // Return the completed job
        self.get_job(job_id).await.context("Job disappeared")
    }

    /// Clean up old uploads and jobs.
    pub async fn cleanup(&self, max_age: Duration) {
        debug!("Cleanup triggered with max_age: {:?}", max_age);
        // TODO: Implement cleanup of old files and completed jobs
    }

    /// Delete a job.
    pub async fn delete_job(&self, job_id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        jobs.remove(job_id).is_some()
    }

    /// Check if the backend is ready.
    pub async fn is_ready(&self) -> bool {
        self.backend.is_ready().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_config() -> VlmInferenceConfig {
        VlmInferenceConfig {
            backend: BackendType::Mock,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_create_job() {
        let config = mock_config();
        let service = VlmInferenceService::new(config).await.unwrap();

        let job_id = service
            .create_job("vlm-default".to_string(), "Describe this video".to_string())
            .await;

        let job = service.get_job(&job_id).await;
        assert!(job.is_some());

        let job = job.unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.model, "vlm-default");
    }

    #[tokio::test]
    async fn test_list_models() {
        let config = mock_config();
        let service = VlmInferenceService::new(config).await.unwrap();

        let models = service.list_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "vlm-default"));
    }

    #[tokio::test]
    async fn test_backend_name() {
        let config = mock_config();
        let service = VlmInferenceService::new(config).await.unwrap();
        assert_eq!(service.backend_name(), "mock");
    }

    #[tokio::test]
    async fn test_with_custom_backend() {
        let config = mock_config();
        let backend: Arc<dyn InferenceBackend> = Arc::new(MockBackend::with_delay(1));
        let service = VlmInferenceService::with_backend(config, backend)
            .await
            .unwrap();
        assert_eq!(service.backend_name(), "mock");
    }

    #[test]
    fn test_default_backend_is_mock() {
        let config = VlmInferenceConfig::default();
        assert_eq!(config.effective_backend(), BackendType::Mock);
    }
}
