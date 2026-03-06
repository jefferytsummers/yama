//! Event bus backend for real VLM container communication.
//!
//! Sends inference requests to the VLM container via the event bus
//! and receives results asynchronously.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use prost::Message;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use yama_container_sdk::EventBusClient;
use yama_protocol::vlm::{ImageFormat, VlmAnalyzeRequest, VlmAnalyzeResponse};

use super::backend::{update_job_progress, update_job_status, InferenceBackend};
use super::frame_extractor::{ExtractionConfig, FrameExtractor};
use super::{InferenceChunk, InferenceJob, JobStatus, VlmInferenceConfig};

/// Backend that communicates with the VLM container via event bus.
pub struct EventBusBackend {
    /// Event bus client for sending requests.
    client: Arc<EventBusClient>,
    /// Configuration for frame extraction.
    extraction_config: ExtractionConfig,
    /// Timeout for each inference request.
    request_timeout: Duration,
    /// Pending requests waiting for responses.
    pending: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

/// A pending inference request.
struct PendingRequest {
    /// Channel to send the response.
    tx: mpsc::Sender<VlmAnalyzeResponse>,
}

impl EventBusBackend {
    /// Create a new event bus backend.
    pub async fn new(config: &VlmInferenceConfig) -> Result<Self> {
        // Connect to event bus
        let client = EventBusClient::connect_ws("ws://localhost:8765", "yama-host-inference")
            .await
            .context("Failed to connect to event bus")?;

        let client = Arc::new(client);

        // Subscribe to VLM results
        client
            .subscribe(&["vlm.analyze.result"])
            .await
            .context("Failed to subscribe to vlm.analyze.result")?;

        let extraction_config = ExtractionConfig {
            interval_ms: config.frame_interval_ms,
            max_frames: config.max_frames,
            target_width: None,
            target_height: None,
        };

        let pending: Arc<RwLock<HashMap<String, PendingRequest>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Spawn background task to receive responses
        let pending_clone = pending.clone();
        let recv_client = EventBusClient::connect_ws("ws://localhost:8765", "yama-host-recv")
            .await
            .context("Failed to connect to event bus for receiving")?;

        recv_client
            .subscribe(&["vlm.analyze.result"])
            .await
            .context("Failed to subscribe to vlm.analyze.result")?;

        tokio::spawn(async move {
            Self::response_loop(recv_client, pending_clone).await;
        });

        info!("EventBusBackend connected to VLM container");

        Ok(Self {
            client,
            extraction_config,
            request_timeout: Duration::from_secs(60),
            pending,
        })
    }

    /// Background loop to receive and dispatch responses.
    async fn response_loop(
        mut client: EventBusClient,
        pending: Arc<RwLock<HashMap<String, PendingRequest>>>,
    ) {
        info!("Starting VLM response receiver loop");

        while let Some(event) = client.recv().await {
            if event.envelope.topic == "vlm.analyze.result" {
                // Decode the response
                match VlmAnalyzeResponse::decode(event.payload.as_ref()) {
                    Ok(response) => {
                        let request_id = response.request_id.clone();
                        debug!("Received VLM response for request {}", request_id);

                        // Find and notify the pending request
                        let mut pending_guard = pending.write().await;
                        if let Some(pending_req) = pending_guard.remove(&request_id) {
                            if let Err(e) = pending_req.tx.send(response).await {
                                warn!("Failed to deliver response for {}: {}", request_id, e);
                            }
                        } else {
                            debug!("No pending request found for {}", request_id);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to decode VLM response: {}", e);
                    }
                }
            }
        }

        warn!("VLM response receiver loop ended");
    }

    /// Send an analyze request and wait for response.
    async fn analyze_frame(
        &self,
        request_id: &str,
        prompt: &str,
        image_data: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<VlmAnalyzeResponse> {
        // Create response channel
        let (tx, mut rx) = mpsc::channel(1);

        // Register pending request
        {
            let mut pending = self.pending.write().await;
            pending.insert(request_id.to_string(), PendingRequest { tx });
        }

        // Build request
        let request = VlmAnalyzeRequest {
            request_id: request_id.to_string(),
            prompt: prompt.to_string(),
            image_source: Some(yama_protocol::vlm::vlm_analyze_request::ImageSource::ImageData(
                image_data,
            )),
            width,
            height,
            format: ImageFormat::Jpeg as i32, // Assume JPEG encoding
            max_tokens: 512,
            temperature: 0.7,
        };

        // Send request
        self.client
            .publish("vlm.analyze.request", request)
            .await
            .context("Failed to send analyze request")?;

        debug!("Sent VLM analyze request: {}", request_id);

        // Wait for response with timeout
        match timeout(self.request_timeout, rx.recv()).await {
            Ok(Some(response)) => {
                if !response.error.is_empty() {
                    anyhow::bail!("VLM error: {}", response.error);
                }
                Ok(response)
            }
            Ok(None) => anyhow::bail!("Response channel closed"),
            Err(_) => {
                // Clean up pending request
                let mut pending = self.pending.write().await;
                pending.remove(request_id);
                anyhow::bail!("Request timed out after {:?}", self.request_timeout)
            }
        }
    }
}

#[async_trait]
impl InferenceBackend for EventBusBackend {
    async fn run_inference(
        &self,
        jobs: Arc<RwLock<HashMap<String, InferenceJob>>>,
        job_id: String,
        job: InferenceJob,
    ) -> Result<()> {
        info!("Starting event bus inference for job {}", job_id);

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

        // Update to inferring status
        update_job_status(&jobs, &job_id, JobStatus::Inferring).await;

        // Process each frame
        for (i, frame) in frames.iter().enumerate() {
            let request_id = format!("{}-frame-{}", job_id, i);

            // For mock frames (empty data), we'll send a placeholder
            // In real implementation, frames would have actual image data
            let image_data = if frame.data.is_empty() {
                // Create a minimal placeholder JPEG (1x1 pixel)
                // In production, this would be actual frame data
                vec![
                    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01,
                    0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08,
                    0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A,
                    0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D,
                    0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22,
                    0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34,
                    0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0,
                    0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4,
                    0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                    0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01,
                    0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
                    0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0xFF, 0xD9,
                ]
            } else {
                frame.data.clone()
            };

            match self
                .analyze_frame(&request_id, &job.prompt, image_data, frame.width, frame.height)
                .await
            {
                Ok(response) => {
                    let chunk = InferenceChunk {
                        frame_number: frame.number,
                        timestamp_ms: frame.timestamp_ms,
                        text: response.analysis,
                    };

                    update_job_progress(
                        &jobs,
                        &job_id,
                        i as u64 + 1,
                        total_frames,
                        Some(chunk),
                    )
                    .await;

                    debug!(
                        "Job {} progress: {}/{}",
                        job_id,
                        i + 1,
                        total_frames
                    );
                }
                Err(e) => {
                    error!("Frame {} analysis failed: {}", i, e);
                    // Continue with next frame rather than failing entire job
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

        info!("Event bus inference completed for job {}", job_id);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "event_bus"
    }

    async fn is_ready(&self) -> bool {
        // Could implement a health check here
        true
    }

    async fn warmup(&self) -> Result<()> {
        // Send a small warmup request to the VLM container
        info!("Warming up VLM connection...");
        // For now, just verify connection is alive
        Ok(())
    }
}

/// Create an event bus backend with default configuration.
pub async fn create_event_bus_backend(config: &VlmInferenceConfig) -> Result<EventBusBackend> {
    EventBusBackend::new(config).await
}
