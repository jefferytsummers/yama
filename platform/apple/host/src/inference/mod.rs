//! VLM Video Inference Service
//!
//! Provides video analysis using Vision Language Models. Supports:
//! - Single video file upload and analysis
//! - Frame extraction and chunking
//! - Streaming results via event bus
//! - Mock inference for development

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

mod frame_extractor;
pub use frame_extractor::FrameExtractor;

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
    /// Enable mock inference (for development).
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
            mock_enabled: default_mock_enabled(),
        }
    }
}

/// VLM Inference Service.
///
/// Manages video uploads, inference jobs, and model interactions.
pub struct VlmInferenceService {
    config: VlmInferenceConfig,
    jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
    uploads: Arc<RwLock<HashMap<String, UploadInfo>>>,
    models: Vec<VlmModelInfo>,
}

impl VlmInferenceService {
    /// Create a new VLM inference service.
    pub async fn new(config: VlmInferenceConfig) -> Result<Self> {
        // Ensure upload directory exists
        tokio::fs::create_dir_all(&config.upload_dir)
            .await
            .with_context(|| format!("Failed to create upload dir: {:?}", config.upload_dir))?;

        info!(
            "VLM inference service initialized (mock={})",
            config.mock_enabled
        );

        // Available models (mock for now)
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

        Ok(Self {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            uploads: Arc::new(RwLock::new(HashMap::new())),
            models,
        })
    }

    /// Get service configuration.
    pub fn config(&self) -> &VlmInferenceConfig {
        &self.config
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
    pub async fn start_inference(
        &self,
        job_id: String,
        upload_id: String,
    ) -> Result<()> {
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
        let config = self.config.clone();

        // Spawn the inference task
        tokio::spawn(async move {
            if let Err(e) = run_mock_inference(jobs, job_id.clone(), job, config).await {
                error!("Inference failed for job {}: {}", job_id, e);
            }
        });

        Ok(())
    }

    /// Clean up old uploads and jobs.
    pub async fn cleanup(&self, max_age: Duration) {
        // TODO: Implement cleanup of old files and completed jobs
        debug!("Cleanup triggered with max_age: {:?}", max_age);
    }
}

/// Run mock inference (simulates VLM processing).
async fn run_mock_inference(
    jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
    job_id: String,
    job: InferenceJob,
    _config: VlmInferenceConfig,
) -> Result<()> {
    info!("Starting mock inference for job {}", job_id);

    // Simulate frame extraction
    let total_frames = 10u64; // Mock: 10 frames

    // Update to extracting status
    {
        let mut jobs_guard = jobs.write().await;
        if let Some(j) = jobs_guard.get_mut(&job_id) {
            j.status = JobStatus::Extracting;
            j.total_frames = total_frames;
        }
    }

    // Simulate extraction delay
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Update to inferring status
    {
        let mut jobs_guard = jobs.write().await;
        if let Some(j) = jobs_guard.get_mut(&job_id) {
            j.status = JobStatus::Inferring;
        }
    }

    // Mock inference responses based on the prompt
    let mock_responses = generate_mock_responses(&job.prompt);

    for (i, response) in mock_responses.iter().enumerate() {
        // Simulate processing time (100-300ms per frame)
        let delay = 150 + (i as u64 * 20);
        tokio::time::sleep(Duration::from_millis(delay)).await;

        let chunk = InferenceChunk {
            frame_number: i as u64 + 1,
            timestamp_ms: (i as u64 + 1) * 1000,
            text: response.clone(),
        };

        // Update job progress
        {
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.current_frame = i as u64 + 1;
                j.progress_percent = ((i + 1) as f32 / mock_responses.len() as f32) * 100.0;
                j.results.push(chunk);
            }
        }

        debug!("Job {} progress: {}/{}", job_id, i + 1, mock_responses.len());
    }

    // Mark as completed
    {
        let mut jobs_guard = jobs.write().await;
        if let Some(j) = jobs_guard.get_mut(&job_id) {
            j.status = JobStatus::Completed;
            j.progress_percent = 100.0;
        }
    }

    info!("Mock inference completed for job {}", job_id);
    Ok(())
}

/// Generate mock responses based on the prompt.
fn generate_mock_responses(prompt: &str) -> Vec<String> {
    let prompt_lower = prompt.to_lowercase();

    if prompt_lower.contains("person") || prompt_lower.contains("people") {
        vec![
            "Frame begins with an empty scene. Natural lighting suggests daytime indoor setting.".to_string(),
            "A person enters the frame from the left side, walking at a normal pace.".to_string(),
            "The individual appears to be an adult, wearing casual attire.".to_string(),
            "They pause momentarily, appearing to look at something off-camera.".to_string(),
            "Movement continues toward the center of the frame.".to_string(),
            "The person reaches for an object on a nearby surface.".to_string(),
            "They pick up what appears to be a document or tablet device.".to_string(),
            "Brief examination of the item, turning it in their hands.".to_string(),
            "The person turns and begins walking toward the right side of frame.".to_string(),
            "Scene ends as the subject exits the visible area.".to_string(),
        ]
    } else if prompt_lower.contains("motion") || prompt_lower.contains("movement") {
        vec![
            "Initial frame shows static background with no significant motion.".to_string(),
            "Subtle movement detected in the upper-left quadrant.".to_string(),
            "Motion increases - appears to be an object entering the scene.".to_string(),
            "Primary motion vector: left to right, moderate velocity.".to_string(),
            "Secondary motion detected: slight camera shake or vibration.".to_string(),
            "Movement continues along predicted trajectory.".to_string(),
            "Motion velocity decreases, subject appears to be stopping.".to_string(),
            "Brief pause in primary motion, ambient movement continues.".to_string(),
            "New motion vector detected: stationary to rightward movement.".to_string(),
            "Motion exits frame boundary, scene returns to baseline.".to_string(),
        ]
    } else if prompt_lower.contains("describe") || prompt_lower.contains("summary") {
        vec![
            "Opening: Indoor environment with modern furnishings visible.".to_string(),
            "Lighting analysis: Even artificial lighting, approximately 5000K color temperature.".to_string(),
            "Background elements: Wall-mounted display, potted plant, minimal decor.".to_string(),
            "Floor surface appears to be light-colored hardwood or laminate.".to_string(),
            "Mid-frame: Activity begins with subject entry from off-screen.".to_string(),
            "Subject interaction with environment - reaching and grasping motions.".to_string(),
            "Spatial relationships: Subject maintains central frame position.".to_string(),
            "Temporal progression: Sequence spans approximately 10 seconds.".to_string(),
            "Notable events: Object manipulation, directional changes.".to_string(),
            "Conclusion: Subject exits, scene returns to initial state.".to_string(),
        ]
    } else {
        // Default generic responses
        vec![
            "Video analysis initiated. Processing first segment.".to_string(),
            "Scene establishes with clear visual elements.".to_string(),
            "Activity detected within the frame boundaries.".to_string(),
            "Continuing analysis of visual content.".to_string(),
            "Notable elements identified in current segment.".to_string(),
            "Temporal progression indicates structured sequence.".to_string(),
            "Visual patterns consistent with previous observations.".to_string(),
            "Analyzing contextual relationships between elements.".to_string(),
            "Processing final segments of video content.".to_string(),
            format!("Analysis complete. Prompt context: '{}'", prompt),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_job() {
        let config = VlmInferenceConfig::default();
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
        let config = VlmInferenceConfig::default();
        let service = VlmInferenceService::new(config).await.unwrap();

        let models = service.list_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "vlm-default"));
    }
}
