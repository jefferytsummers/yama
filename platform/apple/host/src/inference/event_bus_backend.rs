//! Event bus backend for real VLM container communication.
//!
//! Sends inference requests to the VLM container via the event bus
//! and receives results asynchronously. Uses frame pipelining for
//! improved throughput - multiple frames are sent concurrently while
//! maintaining bounded in-flight requests to prevent overload.
//!
//! Implements both the legacy `InferenceBackend` trait (for job-based processing)
//! and the new unified `VlmBackend` trait (for single-image analysis).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::stream::{self, StreamExt};
use prost::Message;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use yama_platform_traits::{
    PlatformError, PlatformResult, VlmBackend, VlmCapabilities, VlmImageSource, VlmRequest,
    VlmResponse,
};

use yama_container_sdk::EventBusClient;
use yama_protocol::vlm::{ImageFormat, VlmAnalyzeProgress, VlmAnalyzeRequest, VlmAnalyzeResponse};

use super::backend::{update_job_progress, update_job_status, InferenceBackend};
use super::frame_extractor::{ExtractionConfig, ExtractedFrame, FrameExtractor};
use super::{InferenceChunk, InferenceJob, JobStatus, VlmInferenceConfig};

/// Maximum number of connection retry attempts.
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Initial backoff duration for retries.
const INITIAL_BACKOFF_MS: u64 = 500;

/// Maximum number of concurrent frame analysis requests (pipeline depth).
const MAX_IN_FLIGHT_FRAMES: usize = 5;

/// Interval between connection health checks.
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Number of consecutive missed health checks before reconnecting.
const MAX_MISSED_HEALTH_CHECKS: u32 = 2;

/// Backend that communicates with the VLM container via event bus.
///
/// Implements both:
/// - `InferenceBackend` - Legacy trait for job-based video processing
/// - `VlmBackend` - New unified trait for single-image analysis
pub struct EventBusBackend {
    /// Event bus client for sending requests.
    client: Arc<RwLock<Option<EventBusClient>>>,
    /// Configuration for frame extraction.
    extraction_config: ExtractionConfig,
    /// Timeout for each inference request.
    request_timeout: Duration,
    /// Pending requests waiting for responses.
    pending: Arc<RwLock<HashMap<String, PendingRequest>>>,
    /// Whether the backend is connected and ready.
    is_connected: Arc<AtomicBool>,
    /// Number of consecutive connection failures.
    consecutive_failures: Arc<AtomicU32>,
    /// Configuration for reconnection.
    config: VlmInferenceConfig,
    /// Semaphore to limit concurrent in-flight requests.
    in_flight_semaphore: Arc<Semaphore>,
    /// Timestamp of last successful communication.
    last_activity: Arc<RwLock<std::time::Instant>>,
    /// Handle to the health check task.
    health_check_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Backend capabilities (for VlmBackend trait).
    capabilities: VlmCapabilities,
}

/// A pending inference request.
struct PendingRequest {
    /// Channel to send the response.
    tx: mpsc::Sender<VlmAnalyzeResponse>,
}

/// Progress callback for frame analysis.
pub type ProgressCallback = Arc<dyn Fn(&str, &str, u32) + Send + Sync>;

impl EventBusBackend {
    /// Create a new event bus backend.
    pub async fn new(config: &VlmInferenceConfig) -> Result<Self> {
        let extraction_config = ExtractionConfig {
            interval_ms: config.frame_interval_ms,
            max_frames: config.max_frames,
            target_width: None,
            target_height: None,
            jpeg_quality: 85,
        };

        let pending: Arc<RwLock<HashMap<String, PendingRequest>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let is_connected = Arc::new(AtomicBool::new(false));
        let consecutive_failures = Arc::new(AtomicU32::new(0));

        let capabilities = VlmCapabilities::event_bus("vlm-container");

        let backend = Self {
            client: Arc::new(RwLock::new(None)),
            extraction_config,
            request_timeout: Duration::from_secs(60),
            pending: pending.clone(),
            is_connected: is_connected.clone(),
            consecutive_failures: consecutive_failures.clone(),
            config: config.clone(),
            in_flight_semaphore: Arc::new(Semaphore::new(MAX_IN_FLIGHT_FRAMES)),
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
            health_check_handle: Arc::new(RwLock::new(None)),
            capabilities,
        };

        // Try initial connection (don't fail if container not running)
        match backend.connect_with_retry().await {
            Ok(()) => {
                info!("EventBusBackend connected to VLM container");
            }
            Err(e) => {
                warn!(
                    "EventBusBackend initial connection failed (VLM container may not be running): {}",
                    e
                );
                // Don't fail construction - allow graceful degradation
            }
        }

        Ok(backend)
    }

    /// Connect to the event bus with exponential backoff retry.
    async fn connect_with_retry(&self) -> Result<()> {
        let mut attempts = 0;
        let mut backoff = Duration::from_millis(INITIAL_BACKOFF_MS);

        loop {
            attempts += 1;

            match self.try_connect().await {
                Ok(()) => {
                    self.is_connected.store(true, Ordering::SeqCst);
                    self.consecutive_failures.store(0, Ordering::SeqCst);
                    return Ok(());
                }
                Err(e) => {
                    if attempts >= MAX_RETRY_ATTEMPTS {
                        self.is_connected.store(false, Ordering::SeqCst);
                        self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
                        return Err(e).context(format!(
                            "Failed to connect after {} attempts",
                            MAX_RETRY_ATTEMPTS
                        ));
                    }

                    warn!(
                        "Connection attempt {}/{} failed: {}. Retrying in {:?}",
                        attempts, MAX_RETRY_ATTEMPTS, e, backoff
                    );

                    tokio::time::sleep(backoff).await;
                    backoff *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Single connection attempt.
    async fn try_connect(&self) -> Result<()> {
        // Connect main client
        let client = EventBusClient::connect_ws("ws://localhost:8765", "yama-host-inference")
            .await
            .context("Failed to connect to event bus")?;

        // Subscribe to VLM results and progress
        client
            .subscribe(&["vlm.analyze.result", "vlm.analyze.progress"])
            .await
            .context("Failed to subscribe to vlm.analyze.result/progress")?;

        // Store the client
        {
            let mut client_guard = self.client.write().await;
            *client_guard = Some(client);
        }

        // Reset last activity timestamp
        {
            let mut activity = self.last_activity.write().await;
            *activity = std::time::Instant::now();
        }

        // Spawn background receiver task
        let pending_clone = self.pending.clone();
        let is_connected = self.is_connected.clone();
        let last_activity = self.last_activity.clone();

        let recv_client = EventBusClient::connect_ws("ws://localhost:8765", "yama-host-recv")
            .await
            .context("Failed to connect to event bus for receiving")?;

        recv_client
            .subscribe(&["vlm.analyze.result", "vlm.analyze.progress"])
            .await
            .context("Failed to subscribe to vlm.analyze.result/progress")?;

        tokio::spawn(async move {
            Self::response_loop(recv_client, pending_clone, is_connected, last_activity).await;
        });

        // Start health check task
        self.start_health_check();

        Ok(())
    }

    /// Background loop to receive and dispatch responses.
    async fn response_loop(
        mut client: EventBusClient,
        pending: Arc<RwLock<HashMap<String, PendingRequest>>>,
        is_connected: Arc<AtomicBool>,
        last_activity: Arc<RwLock<std::time::Instant>>,
    ) {
        info!("Starting VLM response receiver loop");

        while let Some(event) = client.recv().await {
            // Update last activity timestamp on any received message
            {
                let mut activity = last_activity.write().await;
                *activity = std::time::Instant::now();
            }

            match event.envelope.topic.as_str() {
                "vlm.analyze.result" => {
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
                "vlm.analyze.progress" => {
                    // Decode progress heartbeat
                    match VlmAnalyzeProgress::decode(event.payload.as_ref()) {
                        Ok(progress) => {
                            debug!(
                                "VLM progress: {} status={} elapsed={}ms",
                                progress.request_id, progress.status, progress.elapsed_ms
                            );
                            // Progress heartbeats confirm the VLM container is still processing
                        }
                        Err(e) => {
                            debug!("Failed to decode VLM progress: {}", e);
                        }
                    }
                }
                _ => {
                    debug!("Ignoring unknown topic: {}", event.envelope.topic);
                }
            }
        }

        // Connection lost
        is_connected.store(false, Ordering::SeqCst);
        warn!("VLM response receiver loop ended - connection lost");
    }

    /// Start background health check task.
    fn start_health_check(&self) {
        let is_connected = self.is_connected.clone();
        let last_activity = self.last_activity.clone();
        let consecutive_failures = self.consecutive_failures.clone();
        let config = self.config.clone();
        let client = self.client.clone();
        let pending = self.pending.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);
            let mut missed_checks = 0u32;

            loop {
                interval.tick().await;

                // Check if we're connected
                if !is_connected.load(Ordering::SeqCst) {
                    debug!("Health check: not connected, skipping");
                    continue;
                }

                // Check last activity time
                let last = {
                    let activity = last_activity.read().await;
                    *activity
                };

                let idle_time = last.elapsed();

                // If we have pending requests but no activity for too long, connection may be stale
                let pending_count = {
                    let pending_guard = pending.read().await;
                    pending_guard.len()
                };

                if pending_count > 0 && idle_time > HEALTH_CHECK_INTERVAL * 2 {
                    missed_checks += 1;
                    warn!(
                        "Health check: {} pending requests but no activity for {:?} ({} missed checks)",
                        pending_count, idle_time, missed_checks
                    );

                    if missed_checks >= MAX_MISSED_HEALTH_CHECKS {
                        warn!("Health check: too many missed checks, marking connection as stale");
                        is_connected.store(false, Ordering::SeqCst);
                        consecutive_failures.fetch_add(1, Ordering::SeqCst);
                        missed_checks = 0;

                        // Clear stale pending requests
                        {
                            let mut pending_guard = pending.write().await;
                            let stale_count = pending_guard.len();
                            pending_guard.clear();
                            if stale_count > 0 {
                                warn!("Cleared {} stale pending requests", stale_count);
                            }
                        }
                    }
                } else {
                    // Activity detected, reset counter
                    if missed_checks > 0 {
                        debug!("Health check: activity detected, resetting missed check counter");
                        missed_checks = 0;
                    }
                }
            }
        });

        // Store handle (we don't need to await it, just let it run)
        let health_handle = self.health_check_handle.clone();
        tokio::spawn(async move {
            let mut guard = health_handle.write().await;
            *guard = Some(handle);
        });
    }

    /// Ensure we have a valid connection, attempting to reconnect if needed.
    async fn ensure_connected(&self) -> Result<()> {
        if self.is_connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Attempting to reconnect to VLM container...");
        self.connect_with_retry().await
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
        // Ensure we're connected
        self.ensure_connected().await?;

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

        // Get client and send request
        {
            let client_guard = self.client.read().await;
            let client = client_guard
                .as_ref()
                .context("Not connected to event bus")?;

            client
                .publish("vlm.analyze.request", request)
                .await
                .context("Failed to send analyze request")?;
        }

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

    /// Analyze frames with pipelining for improved throughput.
    ///
    /// Sends up to `MAX_IN_FLIGHT_FRAMES` requests concurrently, collecting results
    /// and updating progress as each completes. Results may complete out of order.
    async fn analyze_frames_pipelined(
        &self,
        frames: Vec<ExtractedFrame>,
        prompt: &str,
        job_id: &str,
        jobs: &Arc<RwLock<HashMap<String, InferenceJob>>>,
    ) -> Result<(u64, u64)> {
        let total_frames = frames.len() as u64;
        let completed = Arc::new(AtomicU32::new(0));
        let failed = Arc::new(AtomicU32::new(0));

        // Create indexed frame data for concurrent processing
        let frame_data: Vec<(usize, ExtractedFrame)> = frames.into_iter().enumerate().collect();

        // Process frames concurrently with bounded parallelism
        let results: Vec<_> = stream::iter(frame_data)
            .map(|(i, frame)| {
                let semaphore = self.in_flight_semaphore.clone();
                let job_id = job_id.to_string();
                let prompt = prompt.to_string();
                let jobs = jobs.clone();
                let completed = completed.clone();
                let failed = failed.clone();

                async move {
                    // Acquire semaphore permit to limit concurrent requests
                    let _permit = semaphore.acquire().await.expect("semaphore closed");

                    let request_id = format!("{}-frame-{}", job_id, i);

                    // Skip empty frames
                    if frame.data.is_empty() {
                        warn!(
                            "Frame {} has empty data - frame extractor may not be working",
                            i
                        );
                        return (i, None);
                    }

                    debug!(
                        "Sending frame {} ({} bytes JPEG) to VLM container (pipeline)",
                        i,
                        frame.data.len()
                    );

                    match self
                        .analyze_frame(&request_id, &prompt, frame.data.clone(), frame.width, frame.height)
                        .await
                    {
                        Ok(response) => {
                            let chunk = InferenceChunk {
                                frame_number: frame.number,
                                timestamp_ms: frame.timestamp_ms,
                                text: response.analysis,
                            };

                            // Update progress (results may arrive out of order)
                            let completed_count = completed.fetch_add(1, Ordering::SeqCst) + 1;
                            update_job_progress(
                                &jobs,
                                &job_id,
                                completed_count as u64,
                                total_frames,
                                Some(chunk.clone()),
                            )
                            .await;

                            debug!(
                                "Job {} progress: {}/{} (frame {} completed)",
                                job_id,
                                completed_count,
                                total_frames,
                                i
                            );

                            (i, Some(chunk))
                        }
                        Err(e) => {
                            error!("Frame {} analysis failed: {}", i, e);
                            failed.fetch_add(1, Ordering::SeqCst);
                            (i, None)
                        }
                    }
                }
            })
            // Process up to MAX_IN_FLIGHT_FRAMES concurrently
            .buffer_unordered(MAX_IN_FLIGHT_FRAMES)
            .collect()
            .await;

        let final_completed = completed.load(Ordering::SeqCst) as u64;
        let final_failed = failed.load(Ordering::SeqCst) as u64;

        // Check if too many frames failed
        if final_failed > total_frames / 2 {
            anyhow::bail!(
                "Too many frame failures ({}/{}) during pipelined analysis",
                final_failed,
                total_frames
            );
        }

        // Sort results by frame index and return successful ones
        let mut sorted_results: Vec<_> = results.into_iter().filter_map(|(i, chunk)| chunk.map(|c| (i, c))).collect();
        sorted_results.sort_by_key(|(i, _)| *i);

        info!(
            "Pipelined analysis completed: {}/{} frames succeeded",
            final_completed, total_frames
        );

        Ok((final_completed, final_failed))
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
        info!(
            "Starting event bus inference for job {} (pipeline depth: {})",
            job_id, MAX_IN_FLIGHT_FRAMES
        );

        // Ensure connection before starting
        if let Err(e) = self.ensure_connected().await {
            error!("Cannot run inference: {}", e);
            let mut jobs_guard = jobs.write().await;
            if let Some(j) = jobs_guard.get_mut(&job_id) {
                j.status = JobStatus::Failed;
                j.error = Some(format!("VLM container not available: {}", e));
            }
            return Err(e);
        }

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

        // Update to inferring status
        update_job_status(&jobs, &job_id, JobStatus::Inferring).await;

        // Process frames with pipelining for improved throughput
        match self
            .analyze_frames_pipelined(frames, &job.prompt, &job_id, &jobs)
            .await
        {
            Ok((completed, failed)) => {
                info!(
                    "Job {} pipelined analysis complete: {} succeeded, {} failed",
                    job_id, completed, failed
                );

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
            Err(e) => {
                error!("Pipelined analysis failed for job {}: {}", job_id, e);
                let mut jobs_guard = jobs.write().await;
                if let Some(j) = jobs_guard.get_mut(&job_id) {
                    j.status = JobStatus::Failed;
                    j.error = Some(format!("Pipelined analysis failed: {}", e));
                }
                Err(e)
            }
        }
    }

    fn name(&self) -> &'static str {
        "event_bus"
    }

    async fn is_ready(&self) -> bool {
        // Check if we have an active connection
        if !self.is_connected.load(Ordering::SeqCst) {
            return false;
        }

        // Verify the client is still valid
        let client_guard = self.client.read().await;
        client_guard.is_some()
    }

    async fn warmup(&self) -> Result<()> {
        info!("Warming up VLM connection...");

        // Try to establish connection if not already connected
        if !self.is_connected.load(Ordering::SeqCst) {
            if let Err(e) = self.connect_with_retry().await {
                warn!("Warmup connection failed: {}", e);
                return Err(e);
            }
        }

        info!("VLM connection warmup complete");
        Ok(())
    }
}

/// Create an event bus backend with default configuration.
pub async fn create_event_bus_backend(config: &VlmInferenceConfig) -> Result<EventBusBackend> {
    EventBusBackend::new(config).await
}

// =============================================================================
// VlmBackend trait implementation (new unified interface)
// =============================================================================

#[async_trait]
impl VlmBackend for EventBusBackend {
    fn capabilities(&self) -> &VlmCapabilities {
        &self.capabilities
    }

    async fn analyze(&self, request: VlmRequest) -> PlatformResult<VlmResponse> {
        let start = std::time::Instant::now();

        // Convert image source to JPEG bytes
        let (image_data, width, height) = match request.image {
            VlmImageSource::Jpeg(data) => {
                // Decode to get dimensions
                let img = image::load_from_memory(&data)
                    .map_err(|e| PlatformError::Other(format!("Failed to decode JPEG: {}", e)))?;
                (data, img.width(), img.height())
            }
            VlmImageSource::Png(data) => {
                // Convert PNG to JPEG
                let img = image::load_from_memory(&data)
                    .map_err(|e| PlatformError::Other(format!("Failed to decode PNG: {}", e)))?;
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .map_err(|e| PlatformError::Other(format!("Failed to encode JPEG: {}", e)))?;
                (jpeg_data, img.width(), img.height())
            }
            VlmImageSource::RawRgb { width, height, data } => {
                // Convert RGB to JPEG
                let img = image::RgbImage::from_raw(width, height, data)
                    .ok_or_else(|| PlatformError::Other("Invalid RGB dimensions".to_string()))?;
                let dynamic = image::DynamicImage::ImageRgb8(img);
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                dynamic.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .map_err(|e| PlatformError::Other(format!("Failed to encode JPEG: {}", e)))?;
                (jpeg_data, width, height)
            }
            VlmImageSource::RawRgba { width, height, data } => {
                // Convert RGBA to JPEG
                let img = image::RgbaImage::from_raw(width, height, data)
                    .ok_or_else(|| PlatformError::Other("Invalid RGBA dimensions".to_string()))?;
                let dynamic = image::DynamicImage::ImageRgba8(img);
                let mut jpeg_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut jpeg_data);
                dynamic.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .map_err(|e| PlatformError::Other(format!("Failed to encode JPEG: {}", e)))?;
                (jpeg_data, width, height)
            }
            VlmImageSource::Nv12 { .. } => {
                return Err(PlatformError::Other(
                    "NV12 format not supported by event bus backend - use JPEG".to_string(),
                ));
            }
            VlmImageSource::SharedMemory { .. } => {
                return Err(PlatformError::Other(
                    "Shared memory not supported by event bus backend".to_string(),
                ));
            }
        };

        // Send request via event bus
        let response = self
            .analyze_frame(&request.request_id, &request.prompt, image_data, width, height)
            .await
            .map_err(|e| PlatformError::Other(format!("Event bus analyze failed: {}", e)))?;

        let inference_time_ms = start.elapsed().as_secs_f32() * 1000.0;

        Ok(VlmResponse {
            request_id: response.request_id,
            analysis: response.analysis,
            inference_time_ms,
            tokens_generated: response.tokens_generated,
            model: "vlm-container".to_string(),
        })
    }

    async fn is_ready(&self) -> bool {
        // Check if we have an active connection
        if !self.is_connected.load(Ordering::SeqCst) {
            return false;
        }

        // Verify the client is still valid
        let client_guard = self.client.read().await;
        client_guard.is_some()
    }

    async fn warmup(&self) -> PlatformResult<()> {
        info!("Warming up event bus VLM backend...");

        // Try to establish connection if not already connected
        if !self.is_connected.load(Ordering::SeqCst) {
            self.connect_with_retry()
                .await
                .map_err(|e| PlatformError::Other(format!("Connection failed: {}", e)))?;
        }

        info!("Event bus VLM backend warmup complete");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "event_bus"
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        info!("Shutting down event bus VLM backend");

        // Cancel health check task
        {
            let mut handle_guard = self.health_check_handle.write().await;
            if let Some(handle) = handle_guard.take() {
                handle.abort();
            }
        }

        // Clear pending requests
        {
            let mut pending_guard = self.pending.write().await;
            pending_guard.clear();
        }

        // Mark as disconnected
        self.is_connected.store(false, Ordering::SeqCst);

        Ok(())
    }
}
