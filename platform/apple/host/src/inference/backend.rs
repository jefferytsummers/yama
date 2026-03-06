//! Inference backend abstraction.
//!
//! Provides a trait for swappable inference backends (mock, event bus, etc.).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{InferenceChunk, InferenceJob, JobStatus};

/// Backend for running VLM inference.
///
/// This trait abstracts the inference implementation, allowing:
/// - Mock backend for development/testing
/// - Event bus backend for real VLM container communication
/// - Direct backend for in-process inference (future)
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Run inference for a job.
    ///
    /// The backend should:
    /// 1. Extract frames from the video
    /// 2. Run inference on each frame
    /// 3. Update job progress via the provided jobs map
    /// 4. Set final status (Completed/Failed)
    async fn run_inference(
        &self,
        jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
        job_id: String,
        job: InferenceJob,
    ) -> Result<()>;

    /// Get backend name for logging/diagnostics.
    fn name(&self) -> &'static str;

    /// Check if the backend is ready to accept requests.
    async fn is_ready(&self) -> bool {
        true
    }

    /// Warm up the backend (e.g., load model).
    async fn warmup(&self) -> Result<()> {
        Ok(())
    }
}

/// Helper to update job status.
pub async fn update_job_status(
    jobs: &Arc<RwLock<HashMap<String, InferenceJob>>>,
    job_id: &str,
    status: JobStatus,
) {
    let mut jobs_guard = jobs.write().await;
    if let Some(job) = jobs_guard.get_mut(job_id) {
        job.status = status;
    }
}

/// Helper to update job progress.
pub async fn update_job_progress(
    jobs: &Arc<RwLock<HashMap<String, InferenceJob>>>,
    job_id: &str,
    current_frame: u64,
    total_frames: u64,
    chunk: Option<InferenceChunk>,
) {
    let mut jobs_guard = jobs.write().await;
    if let Some(job) = jobs_guard.get_mut(job_id) {
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

/// Helper to set job error.
#[allow(dead_code)] // Kept for use by backends that need error handling
pub async fn set_job_error(
    jobs: &Arc<RwLock<HashMap<String, InferenceJob>>>,
    job_id: &str,
    error: String,
) {
    let mut jobs_guard = jobs.write().await;
    if let Some(job) = jobs_guard.get_mut(job_id) {
        job.status = JobStatus::Failed;
        job.error = Some(error);
    }
}
