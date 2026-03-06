//! Mock inference backend for development and testing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::backend::{update_job_progress, update_job_status, InferenceBackend};
use super::{InferenceChunk, InferenceJob, JobStatus};

/// Mock backend that simulates VLM inference.
///
/// Generates realistic-looking responses based on the prompt
/// without actually running any ML model.
#[derive(Debug, Default, Clone)]
pub struct MockBackend {
    /// Simulated delay per frame in milliseconds.
    delay_per_frame_ms: u64,
}

impl MockBackend {
    /// Create a new mock backend.
    pub fn new() -> Self {
        Self {
            delay_per_frame_ms: 150,
        }
    }

    /// Create a mock backend with custom delay.
    pub fn with_delay(delay_ms: u64) -> Self {
        Self {
            delay_per_frame_ms: delay_ms,
        }
    }
}

#[async_trait]
impl InferenceBackend for MockBackend {
    async fn run_inference(
        &self,
        jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
        job_id: String,
        job: InferenceJob,
    ) -> Result<()> {
        info!("Starting mock inference for job {}", job_id);

        // Simulate frame extraction
        let total_frames = 10u64;

        // Update to extracting status
        update_job_status(&jobs, &job_id, JobStatus::Extracting).await;
        {
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.total_frames = total_frames;
            }
        }

        // Simulate extraction delay
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Update to inferring status
        update_job_status(&jobs, &job_id, JobStatus::Inferring).await;

        // Generate mock responses based on the prompt
        let mock_responses = generate_mock_responses(&job.prompt);

        for (i, response) in mock_responses.iter().enumerate() {
            // Simulate processing time
            let delay = self.delay_per_frame_ms + (i as u64 * 20);
            tokio::time::sleep(Duration::from_millis(delay)).await;

            let chunk = InferenceChunk {
                frame_number: i as u64 + 1,
                timestamp_ms: (i as u64 + 1) * 1000,
                text: response.clone(),
            };

            // Update job progress
            update_job_progress(
                &jobs,
                &job_id,
                i as u64 + 1,
                mock_responses.len() as u64,
                Some(chunk),
            )
            .await;

            debug!(
                "Job {} progress: {}/{}",
                job_id,
                i + 1,
                mock_responses.len()
            );
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

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Generate mock responses based on the prompt.
fn generate_mock_responses(prompt: &str) -> Vec<String> {
    let prompt_lower = prompt.to_lowercase();

    if prompt_lower.contains("person") || prompt_lower.contains("people") {
        vec![
            "Frame begins with an empty scene. Natural lighting suggests daytime indoor setting."
                .to_string(),
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
            "Lighting analysis: Even artificial lighting, approximately 5000K color temperature."
                .to_string(),
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
    async fn test_mock_backend_completes() {
        let backend = MockBackend::with_delay(10); // Fast for testing
        let jobs = Arc::new(RwLock::new(HashMap::new()));

        let job = InferenceJob::new(
            "test-job".to_string(),
            "vlm-default".to_string(),
            "Describe this video".to_string(),
        );
        jobs.write().await.insert("test-job".to_string(), job.clone());

        backend
            .run_inference(jobs.clone(), "test-job".to_string(), job)
            .await
            .unwrap();

        let jobs_guard = jobs.read().await;
        let completed_job = jobs_guard.get("test-job").unwrap();
        assert_eq!(completed_job.status, JobStatus::Completed);
        assert!(!completed_job.results.is_empty());
    }

    #[test]
    fn test_prompt_based_responses() {
        let person_responses = generate_mock_responses("Find people in the video");
        assert!(person_responses[0].contains("scene"));

        let motion_responses = generate_mock_responses("Detect motion");
        assert!(motion_responses[0].contains("motion"));

        let describe_responses = generate_mock_responses("Describe everything");
        assert!(describe_responses[0].contains("Indoor"));
    }
}
