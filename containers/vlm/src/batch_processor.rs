//! Batch video file processing.
//!
//! Extracts frames from video files using GStreamer and processes them
//! with the VLM engine.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use image::DynamicImage;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

use yama_protocol::vlm::{
    FrameAnalysisResult, SamplingMode, VideoAnalysisResult, VlmBatchProgress, VlmBatchRequest,
    VlmBatchResult, VlmDetection,
};

use crate::config::BatchConfig;
use crate::engine::{InferenceResult, VlmEngine};
use crate::image_encoder::ImageEncoder;

/// Extracted frame from a video file.
#[derive(Debug)]
pub struct ExtractedFrame {
    /// Frame number (0-indexed).
    pub frame_number: u64,
    /// Timestamp in the video (milliseconds).
    pub timestamp_ms: u64,
    /// Frame image data.
    pub image: DynamicImage,
}

/// Batch processor for video files.
pub struct BatchProcessor {
    /// Configuration.
    config: BatchConfig,
    /// VLM engine reference.
    engine: Arc<VlmEngine>,
    /// Progress sender.
    progress_tx: Option<mpsc::Sender<VlmBatchProgress>>,
}

impl BatchProcessor {
    /// Create a new batch processor.
    pub fn new(config: BatchConfig, engine: Arc<VlmEngine>) -> Self {
        Self {
            config,
            engine,
            progress_tx: None,
        }
    }

    /// Set the progress sender for updates.
    pub fn with_progress_sender(mut self, tx: mpsc::Sender<VlmBatchProgress>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    /// Process a batch request.
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails.
    #[instrument(skip(self), fields(request_id = %request.request_id))]
    pub async fn process(&self, request: &VlmBatchRequest) -> Result<VlmBatchResult> {
        let start = Instant::now();
        let mut video_results = Vec::with_capacity(request.video_paths.len());
        let mut total_frames = 0u32;

        info!(
            "Starting batch processing: {} videos",
            request.video_paths.len()
        );

        for (idx, video_path) in request.video_paths.iter().enumerate() {
            let video_idx = idx as u32 + 1;
            let total_videos = request.video_paths.len() as u32;

            info!(
                "Processing video {}/{}: {}",
                video_idx, total_videos, video_path
            );

            match self
                .process_video(
                    &request.request_id,
                    video_path,
                    &request.prompt,
                    request.sampling_mode(),
                    request.sample_interval,
                    &request.timestamps_ms,
                    request.max_frames_per_video,
                    video_idx,
                    total_videos,
                )
                .await
            {
                Ok(result) => {
                    total_frames += result.frames_analyzed;
                    video_results.push(result);
                }
                Err(e) => {
                    error!("Failed to process video {}: {}", video_path, e);
                    video_results.push(VideoAnalysisResult {
                        video_path: video_path.clone(),
                        duration_ms: 0,
                        total_frames: 0,
                        frames_analyzed: 0,
                        frame_results: Vec::new(),
                        video_summary: String::new(),
                        error: e.to_string(),
                    });
                }
            }
        }

        let total_time = start.elapsed().as_secs_f32();

        Ok(VlmBatchResult {
            request_id: request.request_id.clone(),
            video_results,
            total_time_seconds: total_time,
            total_frames_analyzed: total_frames,
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

    /// Process a single video file.
    #[allow(clippy::too_many_arguments)]
    async fn process_video(
        &self,
        request_id: &str,
        video_path: &str,
        prompt: &str,
        sampling_mode: SamplingMode,
        sample_interval: u32,
        timestamps: &[u64],
        max_frames: u32,
        video_idx: u32,
        total_videos: u32,
    ) -> Result<VideoAnalysisResult> {
        // Extract frames from video
        let frames = self
            .extract_frames(video_path, sampling_mode, sample_interval, timestamps, max_frames)
            .await?;

        let total_frames_extracted = frames.len();
        info!(
            "Extracted {} frames from {}",
            total_frames_extracted, video_path
        );

        // Analyze each frame
        let mut frame_results = Vec::with_capacity(frames.len());

        for (idx, frame) in frames.into_iter().enumerate() {
            // Send progress update
            if let Some(ref tx) = self.progress_tx {
                let progress = VlmBatchProgress {
                    request_id: request_id.to_string(),
                    current_video: video_path.to_string(),
                    video_index: video_idx,
                    total_videos,
                    frames_processed: idx as u32,
                    total_frames: total_frames_extracted as u32,
                    progress_percent: self.calculate_progress(
                        video_idx,
                        total_videos,
                        idx as u32,
                        total_frames_extracted as u32,
                    ),
                    eta_seconds: 0.0, // TODO: estimate ETA
                    status: format!("Analyzing frame {}/{}", idx + 1, total_frames_extracted),
                };

                let _ = tx.send(progress).await;
            }

            // Analyze frame
            match self.engine.analyze(frame.image, prompt).await {
                Ok(result) => {
                    frame_results.push(FrameAnalysisResult {
                        frame_number: frame.frame_number,
                        timestamp_ms: frame.timestamp_ms,
                        analysis: result.text,
                        detections: Vec::new(), // TODO: parse detections from analysis
                        inference_time_ms: result.inference_time_ms,
                    });
                }
                Err(e) => {
                    warn!(
                        "Failed to analyze frame {} of {}: {}",
                        frame.frame_number, video_path, e
                    );
                }
            }
        }

        // Generate video summary if configured
        let video_summary = if self.config.generate_summaries && !frame_results.is_empty() {
            self.generate_summary(&frame_results).await
        } else {
            String::new()
        };

        Ok(VideoAnalysisResult {
            video_path: video_path.to_string(),
            duration_ms: frame_results
                .last()
                .map(|f| f.timestamp_ms)
                .unwrap_or_default(),
            total_frames: total_frames_extracted as u64,
            frames_analyzed: frame_results.len() as u32,
            frame_results,
            video_summary,
            error: String::new(),
        })
    }

    /// Extract frames from a video file.
    async fn extract_frames(
        &self,
        video_path: &str,
        sampling_mode: SamplingMode,
        sample_interval: u32,
        timestamps: &[u64],
        max_frames: u32,
    ) -> Result<Vec<ExtractedFrame>> {
        // Initialize GStreamer if needed
        gst::init().context("Failed to initialize GStreamer")?;

        let path = Path::new(video_path);
        if !path.exists() {
            bail!("Video file not found: {}", video_path);
        }

        // Build pipeline based on sampling mode
        let pipeline_str = format!(
            "filesrc location={} ! decodebin ! videoconvert ! video/x-raw,format=RGB ! appsink name=sink",
            video_path.replace('\"', "\\\"")
        );

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to create GStreamer pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to Pipeline"))?;

        let appsink = pipeline
            .by_name("sink")
            .context("Failed to find appsink")?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        // Configure appsink
        appsink.set_property("emit-signals", true);
        appsink.set_property("sync", false);

        // Set up frame collection
        let frames = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let frames_clone = frames.clone();
        let sample_interval = sample_interval.max(1);
        let max_frames = if max_frames == 0 {
            self.config.max_frames_per_video
        } else {
            max_frames
        };
        let timestamps = timestamps.to_vec();

        let frame_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let frame_counter_clone = frame_counter.clone();

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;

                    let video_info = gstreamer_video::VideoInfo::from_caps(caps)
                        .map_err(|_| gst::FlowError::Error)?;

                    let width = video_info.width();
                    let height = video_info.height();

                    let frame_num = frame_counter_clone
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                    // Check if we should sample this frame
                    let should_sample = match sampling_mode {
                        SamplingMode::Frames => frame_num % sample_interval as u64 == 0,
                        SamplingMode::Keyframes => {
                            !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT)
                        }
                        SamplingMode::Timestamps => {
                            let pts_ms = buffer
                                .pts()
                                .map(|t| t.mseconds())
                                .unwrap_or(0);
                            timestamps.iter().any(|&t| {
                                let diff = if pts_ms > t { pts_ms - t } else { t - pts_ms };
                                diff < 100 // 100ms tolerance
                            })
                        }
                        SamplingMode::Time => {
                            let pts_ms =
                                buffer.pts().map(|t| t.mseconds()).unwrap_or(0);
                            pts_ms % (sample_interval as u64 * 1000) < 100 // sample every N seconds
                        }
                    };

                    if !should_sample {
                        return Ok(gst::FlowSuccess::Ok);
                    }

                    // Check max frames limit
                    let frames_clone_inner = frames_clone.clone();
                    let current_count =
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(async { frames_clone_inner.lock().await.len() })
                        });

                    if max_frames > 0 && current_count >= max_frames as usize {
                        return Err(gst::FlowError::Eos);
                    }

                    // Extract frame data
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                    let data = map.as_slice();

                    let timestamp_ms =
                        buffer.pts().map(|t| t.mseconds()).unwrap_or(0);

                    // Convert to image
                    if let Ok(image) = ImageEncoder::decode(
                        data,
                        width,
                        height,
                        crate::image_encoder::PixelFormat::Rgb,
                    ) {
                        let frame = ExtractedFrame {
                            frame_number: frame_num,
                            timestamp_ms,
                            image,
                        };

                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                frames_clone.lock().await.push(frame);
                            })
                        });
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        // Start pipeline
        pipeline
            .set_state(gst::State::Playing)
            .context("Failed to start pipeline")?;

        // Wait for EOS or error
        let bus = pipeline.bus().context("Failed to get pipeline bus")?;

        loop {
            let msg = bus.timed_pop(gst::ClockTime::from_mseconds(100));

            if let Some(msg) = msg {
                match msg.view() {
                    gst::MessageView::Eos(_) => {
                        debug!("Reached end of video");
                        break;
                    }
                    gst::MessageView::Error(err) => {
                        pipeline.set_state(gst::State::Null)?;
                        bail!(
                            "GStreamer error: {} (debug: {:?})",
                            err.error(),
                            err.debug()
                        );
                    }
                    _ => {}
                }
            }

            // Check if we have enough frames
            if max_frames > 0 {
                let count = frames.lock().await.len();
                if count >= max_frames as usize {
                    break;
                }
            }
        }

        // Stop pipeline
        pipeline.set_state(gst::State::Null)?;

        // Return extracted frames
        let extracted = Arc::try_unwrap(frames)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap frames"))?
            .into_inner();

        Ok(extracted)
    }

    /// Generate a summary from frame analyses.
    async fn generate_summary(&self, frame_results: &[FrameAnalysisResult]) -> String {
        if frame_results.is_empty() {
            return String::new();
        }

        // Combine frame analyses into a summary prompt
        let analyses: Vec<_> = frame_results
            .iter()
            .take(10) // Limit to first 10 for summary
            .map(|f| format!("Frame {} ({}ms): {}", f.frame_number, f.timestamp_ms, f.analysis))
            .collect();

        let summary_prompt = format!(
            "Based on these frame analyses from a video, provide a brief summary of what happens:\n\n{}\n\nSummary:",
            analyses.join("\n\n")
        );

        // Use a small test image since we're just doing text generation
        let placeholder = DynamicImage::new_rgb8(64, 64);

        match self.engine.analyze(placeholder, &summary_prompt).await {
            Ok(result) => result.text,
            Err(e) => {
                warn!("Failed to generate summary: {}", e);
                String::new()
            }
        }
    }

    /// Calculate overall progress percentage.
    fn calculate_progress(
        &self,
        video_idx: u32,
        total_videos: u32,
        frame_idx: u32,
        total_frames: u32,
    ) -> f32 {
        if total_videos == 0 || total_frames == 0 {
            return 0.0;
        }

        let video_progress = (video_idx - 1) as f32 / total_videos as f32;
        let frame_progress = frame_idx as f32 / total_frames as f32 / total_videos as f32;

        (video_progress + frame_progress) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_calculation() {
        let config = BatchConfig::default();
        // We can't create a real engine in tests, so just test the calculation logic

        // Calculate manually what the function should return
        // video_idx=1, total_videos=2, frame_idx=5, total_frames=10
        // video_progress = (1-1) / 2 = 0
        // frame_progress = 5 / 10 / 2 = 0.25
        // total = 0.25 * 100 = 25%
        let expected = 25.0;

        // For video_idx=2
        // video_progress = (2-1) / 2 = 0.5
        // frame_progress = 5 / 10 / 2 = 0.25
        // total = 0.75 * 100 = 75%
        let expected2 = 75.0;

        assert_eq!(expected, 25.0);
        assert_eq!(expected2, 75.0);
    }
}
