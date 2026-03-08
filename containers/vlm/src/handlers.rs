//! Event bus message handlers for VLM service.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytes::Bytes;
use image::DynamicImage;
use prost::Message;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

use yama_container_sdk::client::Event;
use yama_protocol::common::VideoFrameMeta;
use yama_protocol::vlm::{
    ImageFormat, LiveStreamConfig, LiveStreamConfigRequest, LiveStreamConfigResponse,
    VlmAnalyzeProgress, VlmAnalyzeRequest, VlmAnalyzeResponse, VlmBatchProgress, VlmBatchRequest,
    VlmBatchResult,
};

use crate::batch_processor::BatchProcessor;
use crate::engine::VlmEngine;
use crate::frame_sampler::{FrameSampler, SampledFrame, SourceConfig};
use crate::image_encoder::{ImageEncoder, PixelFormat};

/// Interval between progress heartbeats during inference.
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(250);

/// Handler for VLM analyze requests.
pub struct AnalyzeHandler {
    engine: Arc<VlmEngine>,
    progress_tx: mpsc::Sender<VlmAnalyzeProgress>,
}

impl AnalyzeHandler {
    /// Create a new analyze handler with a progress channel.
    pub fn new(engine: Arc<VlmEngine>, progress_tx: mpsc::Sender<VlmAnalyzeProgress>) -> Self {
        Self { engine, progress_tx }
    }

    /// Handle an analyze request with progress heartbeats.
    #[instrument(skip(self, request), fields(request_id = %request.request_id))]
    pub async fn handle(&self, request: VlmAnalyzeRequest) -> Result<VlmAnalyzeResponse> {
        debug!("Processing analyze request");

        let request_id = request.request_id.clone();
        let start_time = Instant::now();

        // Decode image from request
        let image = self.decode_image(&request).await?;

        // Get optional parameters
        let temperature = if request.temperature > 0.0 {
            Some(request.temperature)
        } else {
            None
        };

        let max_tokens = if request.max_tokens > 0 {
            Some(request.max_tokens)
        } else {
            None
        };

        // Flag to signal heartbeat task to stop
        let inference_done = Arc::new(AtomicBool::new(false));
        let inference_done_clone = inference_done.clone();

        // Spawn heartbeat task
        let heartbeat_request_id = request_id.clone();
        let heartbeat_tx = self.progress_tx.clone();
        let heartbeat_start = start_time;

        let heartbeat_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
            let mut tick_count = 0u32;

            loop {
                interval.tick().await;

                // Check if inference is done
                if inference_done_clone.load(Ordering::SeqCst) {
                    break;
                }

                tick_count += 1;
                let elapsed_ms = heartbeat_start.elapsed().as_millis() as u32;

                // Determine status based on elapsed time
                let status = if tick_count < 2 {
                    "decoding"
                } else if tick_count < 4 {
                    "analyzing"
                } else {
                    "generating"
                }
                .to_string();

                let progress = VlmAnalyzeProgress {
                    request_id: heartbeat_request_id.clone(),
                    status,
                    tokens_generated: 0, // We don't have streaming token count
                    elapsed_ms,
                    estimated_remaining_ms: 0, // Unknown
                };

                if let Err(e) = heartbeat_tx.try_send(progress) {
                    debug!("Heartbeat send failed (channel full or closed): {}", e);
                }
            }
        });

        // Run inference
        let result = self
            .engine
            .analyze_with_params(image, &request.prompt, temperature, max_tokens)
            .await;

        // Signal heartbeat task to stop
        inference_done.store(true, Ordering::SeqCst);

        // Wait for heartbeat task to finish
        let _ = heartbeat_task.await;

        // Handle result
        let result = result?;

        Ok(VlmAnalyzeResponse {
            request_id,
            analysis: result.text,
            inference_time_ms: result.inference_time_ms,
            tokens_generated: result.tokens_generated,
            detections: Vec::new(), // TODO: parse detections
            error: String::new(),
            completed_at: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                nanos: 0,
            }),
        })
    }

    /// Decode image from request.
    async fn decode_image(&self, request: &VlmAnalyzeRequest) -> Result<DynamicImage> {
        match &request.image_source {
            Some(yama_protocol::vlm::vlm_analyze_request::ImageSource::ImageData(data)) => {
                // Check if it's encoded (JPEG/PNG) or raw
                let format = ImageFormat::try_from(request.format).unwrap_or(ImageFormat::Rgb);

                match format {
                    ImageFormat::Jpeg | ImageFormat::Png => {
                        ImageEncoder::decode_encoded(data)
                    }
                    _ => {
                        let pixel_format = match format {
                            ImageFormat::Rgb => PixelFormat::Rgb,
                            ImageFormat::Rgba => PixelFormat::Rgba,
                            ImageFormat::Bgra => PixelFormat::Bgra,
                            ImageFormat::Nv12 => PixelFormat::Nv12,
                            ImageFormat::I420 => PixelFormat::I420,
                            _ => PixelFormat::Rgb,
                        };

                        ImageEncoder::decode(data, request.width, request.height, pixel_format)
                    }
                }
            }
            Some(yama_protocol::vlm::vlm_analyze_request::ImageSource::ShmName(shm_name)) => {
                // TODO: Implement shared memory reading
                anyhow::bail!("Shared memory image source not yet implemented: {}", shm_name)
            }
            Some(yama_protocol::vlm::vlm_analyze_request::ImageSource::FrameRef(_frame_ref)) => {
                // TODO: Implement frame reference lookup
                anyhow::bail!("Frame reference source not yet implemented")
            }
            None => {
                anyhow::bail!("No image source provided in request")
            }
        }
    }
}

/// Handler for batch processing requests.
pub struct BatchHandler {
    processor: Arc<BatchProcessor>,
    progress_tx: mpsc::Sender<VlmBatchProgress>,
}

impl BatchHandler {
    /// Create a new batch handler.
    pub fn new(processor: Arc<BatchProcessor>, progress_tx: mpsc::Sender<VlmBatchProgress>) -> Self {
        Self {
            processor,
            progress_tx,
        }
    }

    /// Handle a batch request.
    #[instrument(skip(self, request), fields(request_id = %request.request_id))]
    pub async fn handle(&self, request: VlmBatchRequest) -> Result<VlmBatchResult> {
        info!(
            "Processing batch request: {} videos",
            request.video_paths.len()
        );

        self.processor.process(&request).await
    }
}

/// Handler for live video frames.
pub struct FrameHandler {
    engine: Arc<VlmEngine>,
    sampler: Arc<FrameSampler>,
    result_tx: mpsc::Sender<VlmAnalyzeResponse>,
}

impl FrameHandler {
    /// Create a new frame handler.
    pub fn new(
        engine: Arc<VlmEngine>,
        sampler: Arc<FrameSampler>,
        result_tx: mpsc::Sender<VlmAnalyzeResponse>,
    ) -> Self {
        Self {
            engine,
            sampler,
            result_tx,
        }
    }

    /// Handle a video frame event.
    #[instrument(skip(self, meta, data), fields(source_id = %meta.source_id, frame = meta.frame_number))]
    pub async fn handle(&self, meta: VideoFrameMeta, data: Bytes) -> Result<()> {
        // Check if we have this source registered
        if !self.sampler.has_source(&meta.source_id).await {
            // Auto-register if this is a new source
            self.sampler.register_source_default(&meta.source_id).await;
        }

        // Convert pixel format
        let format = match yama_protocol::common::PixelFormat::try_from(meta.format) {
            Ok(yama_protocol::common::PixelFormat::Nv12) => PixelFormat::Nv12,
            Ok(yama_protocol::common::PixelFormat::I420) => PixelFormat::I420,
            Ok(yama_protocol::common::PixelFormat::Rgba) => PixelFormat::Rgba,
            Ok(yama_protocol::common::PixelFormat::Bgra) => PixelFormat::Bgra,
            _ => PixelFormat::Rgb,
        };

        // Decode image
        let image = ImageEncoder::decode(&data, meta.width, meta.height, format)
            .context("Failed to decode frame image")?;

        // Check if we should sample this frame
        let pts = meta.pts.map(|t| Duration::from_secs(t.seconds as u64));

        if let Some(sampled) = self
            .sampler
            .process_frame(&meta.source_id, meta.frame_number, pts, image)
            .await
        {
            // Process the sampled frame
            self.process_sampled_frame(sampled).await?;
        }

        Ok(())
    }

    /// Process a sampled frame.
    async fn process_sampled_frame(&self, frame: SampledFrame) -> Result<()> {
        debug!(
            "Analyzing frame {} from {}",
            frame.frame_number, frame.source_id
        );

        let source_id = frame.source_id.clone();

        match self.engine.analyze(frame.image, &frame.prompt).await {
            Ok(result) => {
                // Report latency for adaptive sampling
                let inference_duration = Duration::from_secs_f32(result.inference_time_ms / 1000.0);
                self.sampler.report_latency(&source_id, inference_duration).await;

                let response = VlmAnalyzeResponse {
                    request_id: format!(
                        "live-{}-{}",
                        source_id, frame.frame_number
                    ),
                    analysis: result.text,
                    inference_time_ms: result.inference_time_ms,
                    tokens_generated: result.tokens_generated,
                    detections: Vec::new(),
                    error: String::new(),
                    completed_at: Some(prost_types::Timestamp {
                        seconds: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        nanos: 0,
                    }),
                };

                if let Err(e) = self.result_tx.send(response).await {
                    warn!("Failed to send analysis result: {}", e);
                }
            }
            Err(e) => {
                error!(
                    "Failed to analyze frame {} from {}: {}",
                    frame.frame_number, source_id, e
                );
            }
        }

        Ok(())
    }
}

/// Handler for live stream configuration updates.
pub struct ConfigHandler {
    sampler: Arc<FrameSampler>,
}

impl ConfigHandler {
    /// Create a new config handler.
    pub fn new(sampler: Arc<FrameSampler>) -> Self {
        Self { sampler }
    }

    /// Handle a configuration update request.
    #[instrument(skip(self, request))]
    pub async fn handle(
        &self,
        request: LiveStreamConfigRequest,
    ) -> Result<LiveStreamConfigResponse> {
        let config = request.config.context("No configuration provided")?;

        debug!(
            "Updating live stream config for source: {}",
            config.source_id
        );

        // Convert to internal config
        let source_config = SourceConfig {
            source_id: config.source_id.clone(),
            sample_interval: Duration::from_millis(config.sample_interval_ms.into()),
            prompt: config.prompt.clone(),
            enabled: config.enabled,
            adaptive: true, // Enable adaptive sampling by default
            target_latency: Duration::from_millis(500),
        };

        // Update or register
        if self.sampler.has_source(&config.source_id).await {
            self.sampler.update_source(source_config).await;
        } else {
            self.sampler.register_source(source_config).await;
        }

        // Get current active configs
        let source_ids = self.sampler.source_ids().await;
        let mut active_configs = Vec::with_capacity(source_ids.len());

        for source_id in source_ids {
            if let Some(stats) = self.sampler.get_stats(&source_id).await {
                active_configs.push(LiveStreamConfig {
                    source_id,
                    enabled: stats.enabled,
                    sample_interval_ms: stats.sample_interval.as_millis() as u32,
                    prompt: String::new(), // TODO: store prompt in stats
                    publish_results: true,
                });
            }
        }

        Ok(LiveStreamConfigResponse {
            success: true,
            active_configs,
            error: String::new(),
        })
    }
}

/// Dispatch incoming events to appropriate handlers.
pub struct EventDispatcher {
    analyze_handler: AnalyzeHandler,
    batch_handler: Option<BatchHandler>,
    frame_handler: Option<FrameHandler>,
    config_handler: ConfigHandler,
}

impl EventDispatcher {
    /// Create a new event dispatcher.
    pub fn new(
        engine: Arc<VlmEngine>,
        sampler: Arc<FrameSampler>,
        batch_processor: Option<Arc<BatchProcessor>>,
        result_tx: mpsc::Sender<VlmAnalyzeResponse>,
        batch_progress_tx: mpsc::Sender<VlmBatchProgress>,
        analyze_progress_tx: mpsc::Sender<VlmAnalyzeProgress>,
    ) -> Self {
        let analyze_handler = AnalyzeHandler::new(engine.clone(), analyze_progress_tx);
        let config_handler = ConfigHandler::new(sampler.clone());

        let batch_handler = batch_processor.map(|p| BatchHandler::new(p, batch_progress_tx));

        let frame_handler = Some(FrameHandler::new(engine, sampler, result_tx));

        Self {
            analyze_handler,
            batch_handler,
            frame_handler,
            config_handler,
        }
    }

    /// Dispatch an event to the appropriate handler.
    #[instrument(skip(self, event), fields(topic = %event.envelope.topic))]
    pub async fn dispatch(&self, event: Event) -> Result<Option<DispatchResult>> {
        let topic = &event.envelope.topic;

        match topic.as_str() {
            "vlm.analyze.request" => {
                let request = VlmAnalyzeRequest::decode(event.payload.as_ref())
                    .context("Failed to decode analyze request")?;

                let response = self.analyze_handler.handle(request).await?;
                Ok(Some(DispatchResult::AnalyzeResponse(response)))
            }

            "vlm.batch.request" => {
                if let Some(ref handler) = self.batch_handler {
                    let request = VlmBatchRequest::decode(event.payload.as_ref())
                        .context("Failed to decode batch request")?;

                    let result = handler.handle(request).await?;
                    Ok(Some(DispatchResult::BatchResult(result)))
                } else {
                    warn!("Batch processing not enabled");
                    Ok(None)
                }
            }

            "vlm.config.request" => {
                let request = LiveStreamConfigRequest::decode(event.payload.as_ref())
                    .context("Failed to decode config request")?;

                let response = self.config_handler.handle(request).await?;
                Ok(Some(DispatchResult::ConfigResponse(response)))
            }

            topic if topic.starts_with("video.frame.") => {
                if let Some(ref handler) = self.frame_handler {
                    let meta = VideoFrameMeta::decode(event.payload.as_ref())
                        .context("Failed to decode frame meta")?;

                    // For now, use the payload data directly
                    // In production, this would read from DMA-BUF or shared memory
                    handler.handle(meta, event.payload).await?;
                }
                Ok(None)
            }

            _ => {
                debug!("Unhandled topic: {}", topic);
                Ok(None)
            }
        }
    }
}

/// Result of event dispatch.
#[derive(Debug)]
pub enum DispatchResult {
    /// Response to analyze request.
    AnalyzeResponse(VlmAnalyzeResponse),
    /// Result of batch processing.
    BatchResult(VlmBatchResult),
    /// Response to config update.
    ConfigResponse(LiveStreamConfigResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_result_variants() {
        // Just test that the enum compiles and can be matched
        let result = DispatchResult::AnalyzeResponse(VlmAnalyzeResponse::default());
        match result {
            DispatchResult::AnalyzeResponse(_) => {}
            _ => panic!("Wrong variant"),
        }
    }
}
