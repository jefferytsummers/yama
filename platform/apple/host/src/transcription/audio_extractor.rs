//! Audio extraction from video files using GStreamer.
//!
//! Extracts audio tracks from video files and converts them to the format
//! required by Whisper (16kHz mono PCM).

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use tracing::{debug, info};

/// Audio output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// 16-bit signed PCM (Whisper format).
    S16,
    /// 32-bit floating point.
    F32,
}

impl AudioFormat {
    /// Get the GStreamer format string.
    fn gst_format(&self) -> &'static str {
        match self {
            Self::S16 => "S16LE",
            Self::F32 => "F32LE",
        }
    }

    /// Get bytes per sample.
    fn bytes_per_sample(&self) -> usize {
        match self {
            Self::S16 => 2,
            Self::F32 => 4,
        }
    }
}

/// Extracted audio data.
#[derive(Debug)]
pub struct ExtractedAudio {
    /// Audio samples.
    pub samples: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Audio extractor configuration.
#[derive(Debug, Clone)]
pub struct AudioExtractorConfig {
    /// Target sample rate (default: 16000 for Whisper).
    pub sample_rate: u32,
    /// Number of channels (default: 1 for mono).
    pub channels: u32,
    /// Output format.
    pub format: AudioFormat,
}

impl Default for AudioExtractorConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            format: AudioFormat::F32,
        }
    }
}

/// Audio extractor for video files.
pub struct AudioExtractor {
    config: AudioExtractorConfig,
}

impl AudioExtractor {
    /// Create a new audio extractor.
    pub fn new(config: AudioExtractorConfig) -> Result<Self> {
        gst::init().context("Failed to initialize GStreamer")?;
        Ok(Self { config })
    }

    /// Create an audio extractor with default Whisper-compatible settings.
    pub fn for_whisper() -> Result<Self> {
        Self::new(AudioExtractorConfig::default())
    }

    /// Extract audio from a video file.
    pub fn extract(&self, video_path: impl AsRef<Path>) -> Result<ExtractedAudio> {
        let video_path = video_path.as_ref();
        info!("Extracting audio from {:?}", video_path);

        let path_str = video_path.to_string_lossy();
        let uri = if video_path.is_absolute() {
            format!("file://{}", path_str)
        } else {
            format!("file://{}/{}", std::env::current_dir()?.display(), path_str)
        };

        // Build the pipeline
        let pipeline_str = format!(
            "uridecodebin uri={uri} ! audioconvert ! audioresample ! \
             audio/x-raw,format={format},rate={rate},channels={channels} ! \
             appsink name=sink",
            uri = uri,
            format = self.config.format.gst_format(),
            rate = self.config.sample_rate,
            channels = self.config.channels,
        );

        debug!("Pipeline: {}", pipeline_str);

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to parse pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to Pipeline"))?;

        let sink = pipeline
            .by_name("sink")
            .context("Failed to get sink")?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        // Collect samples
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = samples.clone();
        let format = self.config.format;

        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    let mut samples_guard = samples_clone.lock().unwrap();

                    match format {
                        AudioFormat::F32 => {
                            // Directly copy f32 samples
                            let float_samples: &[f32] = bytemuck::cast_slice(&map);
                            samples_guard.extend_from_slice(float_samples);
                        }
                        AudioFormat::S16 => {
                            // Convert i16 to f32
                            let int_samples: &[i16] = bytemuck::cast_slice(&map);
                            for &s in int_samples {
                                samples_guard.push(s as f32 / 32768.0);
                            }
                        }
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        // Start the pipeline
        pipeline.set_state(gst::State::Playing)
            .context("Failed to start pipeline")?;

        // Wait for EOS or error
        let bus = pipeline.bus().context("Failed to get bus")?;
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    pipeline.set_state(gst::State::Null)?;
                    anyhow::bail!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                }
                _ => {}
            }
        }

        // Stop the pipeline
        pipeline.set_state(gst::State::Null)?;

        // Get the samples
        let samples = Arc::try_unwrap(samples)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap samples"))?
            .into_inner()
            .map_err(|_| anyhow::anyhow!("Failed to get samples"))?;

        let num_samples = samples.len();
        let duration_secs = num_samples as f64 / self.config.sample_rate as f64 / self.config.channels as f64;

        info!(
            "Extracted {} samples ({:.2}s) at {}Hz",
            num_samples, duration_secs, self.config.sample_rate
        );

        Ok(ExtractedAudio {
            samples,
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            duration_secs,
        })
    }

    /// Extract audio from a time range of a video file.
    pub fn extract_range(
        &self,
        video_path: impl AsRef<Path>,
        start_secs: f64,
        end_secs: f64,
    ) -> Result<ExtractedAudio> {
        let video_path = video_path.as_ref();
        info!(
            "Extracting audio from {:?} ({:.2}s - {:.2}s)",
            video_path, start_secs, end_secs
        );

        let path_str = video_path.to_string_lossy();
        let uri = if video_path.is_absolute() {
            format!("file://{}", path_str)
        } else {
            format!("file://{}/{}", std::env::current_dir()?.display(), path_str)
        };

        // Build the pipeline with seeking
        let pipeline_str = format!(
            "uridecodebin uri={uri} ! audioconvert ! audioresample ! \
             audio/x-raw,format={format},rate={rate},channels={channels} ! \
             appsink name=sink",
            uri = uri,
            format = self.config.format.gst_format(),
            rate = self.config.sample_rate,
            channels = self.config.channels,
        );

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to parse pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to Pipeline"))?;

        let sink = pipeline
            .by_name("sink")
            .context("Failed to get sink")?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        // Collect samples
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = samples.clone();
        let format = self.config.format;

        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    let mut samples_guard = samples_clone.lock().unwrap();

                    match format {
                        AudioFormat::F32 => {
                            let float_samples: &[f32] = bytemuck::cast_slice(&map);
                            samples_guard.extend_from_slice(float_samples);
                        }
                        AudioFormat::S16 => {
                            let int_samples: &[i16] = bytemuck::cast_slice(&map);
                            for &s in int_samples {
                                samples_guard.push(s as f32 / 32768.0);
                            }
                        }
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        // Set to paused first to allow seeking
        pipeline.set_state(gst::State::Paused)
            .context("Failed to pause pipeline")?;

        // Wait for state change
        pipeline.state(gst::ClockTime::from_seconds(5));

        // Seek to start position
        let start_ns = (start_secs * 1_000_000_000.0) as u64;
        let end_ns = (end_secs * 1_000_000_000.0) as u64;

        pipeline.seek(
            1.0,
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            gst::SeekType::Set,
            gst::ClockTime::from_nseconds(start_ns),
            gst::SeekType::Set,
            gst::ClockTime::from_nseconds(end_ns),
        ).context("Failed to seek")?;

        // Start playing
        pipeline.set_state(gst::State::Playing)
            .context("Failed to start pipeline")?;

        // Wait for EOS or error
        let bus = pipeline.bus().context("Failed to get bus")?;
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    pipeline.set_state(gst::State::Null)?;
                    anyhow::bail!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                }
                _ => {}
            }
        }

        // Stop the pipeline
        pipeline.set_state(gst::State::Null)?;

        // Get the samples
        let samples = Arc::try_unwrap(samples)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap samples"))?
            .into_inner()
            .map_err(|_| anyhow::anyhow!("Failed to get samples"))?;

        let num_samples = samples.len();
        let duration_secs = num_samples as f64 / self.config.sample_rate as f64 / self.config.channels as f64;

        info!(
            "Extracted {} samples ({:.2}s) from range at {}Hz",
            num_samples, duration_secs, self.config.sample_rate
        );

        Ok(ExtractedAudio {
            samples,
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            duration_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format() {
        assert_eq!(AudioFormat::S16.gst_format(), "S16LE");
        assert_eq!(AudioFormat::F32.gst_format(), "F32LE");
        assert_eq!(AudioFormat::S16.bytes_per_sample(), 2);
        assert_eq!(AudioFormat::F32.bytes_per_sample(), 4);
    }

    #[test]
    fn test_config_defaults() {
        let config = AudioExtractorConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.format, AudioFormat::F32);
    }
}
