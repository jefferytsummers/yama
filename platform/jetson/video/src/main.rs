//! Yama Video Service - NVIDIA Jetson
//!
//! This service handles video capture and decoding using GStreamer with
//! NVDEC hardware acceleration on NVIDIA Jetson.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_container_sdk::{EventBusClient, HealthReporter};
use yama_platform_traits::decoder::{CodecConfig, CodecType, VideoDecoder};
use yama_protocol::common::{PixelFormat, VideoFrameMeta};

mod decoder;

pub use decoder::{create_decoder, NvdecDecoder};

/// Video source configuration.
#[derive(Debug, Clone, serde::Deserialize)]
struct VideoSourceConfig {
    name: String,
    url: String,
    enabled: bool,
    /// Codec type for the source.
    #[serde(default = "default_codec")]
    codec: String,
    /// Video width.
    #[serde(default = "default_width")]
    width: u32,
    /// Video height.
    #[serde(default = "default_height")]
    height: u32,
}

fn default_codec() -> String {
    "h264".to_string()
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

/// Service configuration.
#[derive(Debug, Clone, serde::Deserialize)]
struct Config {
    event_bus_socket: String,
    sources: Vec<VideoSourceConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: "/tmp/yama-event.sock".to_string(),
            sources: Vec::new(),
        }
    }
}

/// A video pipeline for a single source.
struct VideoPipeline {
    name: String,
    pipeline: gst::Pipeline,
    decoder: Box<dyn VideoDecoder>,
    frame_count: u64,
}

impl VideoPipeline {
    /// Create a new video pipeline with NVDEC hardware decoding.
    fn new(name: &str, config: &VideoSourceConfig) -> Result<Self> {
        info!("Creating video pipeline for {}: {}", name, config.url);

        // Create platform-specific decoder
        let mut decoder = create_decoder()?;

        // Parse codec type
        let codec = match config.codec.to_lowercase().as_str() {
            "h264" | "avc" => CodecType::H264,
            "h265" | "hevc" => CodecType::H265,
            "vp9" => CodecType::Vp9,
            "av1" => CodecType::Av1,
            _ => CodecType::H264,
        };

        // Configure decoder using platform trait
        let codec_config = CodecConfig {
            codec,
            width: config.width,
            height: config.height,
            output_format: Some(yama_platform_traits::PixelFormat::Nv12),
            extra_data: None,
            low_latency: true,
            reference_frames: 0,
        };

        decoder.configure(codec_config)?;
        info!(
            "Decoder configured: {} (zero-copy: {})",
            decoder.capabilities().name,
            decoder.capabilities().supports_zero_copy
        );

        // Build GStreamer pipeline for capture
        // nvv4l2decoder is the Jetson-specific hardware decoder
        let pipeline_str = format!(
            "rtspsrc location={} latency=100 ! \
             rtph264depay ! \
             h264parse ! \
             appsink name=sink emit-signals=true sync=false",
            config.url
        );

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("Failed to create GStreamer pipeline")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to Pipeline"))?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            decoder,
            frame_count: 0,
        })
    }

    /// Start the pipeline.
    fn start(&self) -> Result<()> {
        info!("Starting pipeline: {}", self.name);
        self.pipeline
            .set_state(gst::State::Playing)
            .context("Failed to start pipeline")?;
        Ok(())
    }

    /// Stop the pipeline.
    fn stop(&self) -> Result<()> {
        info!("Stopping pipeline: {}", self.name);
        self.pipeline
            .set_state(gst::State::Null)
            .context("Failed to stop pipeline")?;
        Ok(())
    }

    /// Get the appsink element.
    fn appsink(&self) -> Result<gstreamer_app::AppSink> {
        self.pipeline
            .by_name("sink")
            .context("No appsink found")?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))
    }

    /// Get decoder statistics.
    fn decoder_stats(&self) -> yama_platform_traits::decoder::DecoderStatistics {
        self.decoder.statistics()
    }
}

/// Video service state.
struct VideoService {
    pipelines: HashMap<String, VideoPipeline>,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
}

impl VideoService {
    /// Create a new video service.
    async fn new(config: &Config) -> Result<Self> {
        // Initialize GStreamer
        gst::init().context("Failed to initialize GStreamer")?;
        info!("GStreamer initialized");

        // Check for NVIDIA plugins
        let registry = gst::Registry::get();
        if registry.find_plugin("nvvideo4linux2").is_some() {
            info!("NVIDIA V4L2 plugins available (Jetson hardware decoding)");
        } else {
            warn!("NVIDIA V4L2 plugins not found - hardware decoding may not work");
        }

        // Connect to event bus
        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "video-jetson")
                .await
                .context("Failed to connect to event bus")?,
        );
        info!("Connected to event bus");

        // Set up health reporter
        let health = HealthReporter::new(event_bus.clone(), "video-jetson");

        // Create pipelines for each source
        let mut pipelines = HashMap::new();
        for source in &config.sources {
            if source.enabled {
                match VideoPipeline::new(&source.name, source) {
                    Ok(pipeline) => {
                        info!(
                            "Created pipeline for {} with {:?} decoder",
                            source.name,
                            pipeline.decoder.capabilities().name
                        );
                        pipelines.insert(source.name.clone(), pipeline);
                    }
                    Err(e) => {
                        error!("Failed to create pipeline for {}: {}", source.name, e);
                    }
                }
            }
        }

        Ok(Self {
            pipelines,
            event_bus,
            health,
        })
    }

    /// Run the video service.
    async fn run(mut self) -> Result<()> {
        info!("Starting video service (Jetson)");

        // Start health reporting
        self.health.healthy().await;
        self.health.start().await?;

        // Subscribe to control messages
        self.event_bus
            .subscribe(&["video.control.*"])
            .await
            .context("Failed to subscribe to control topics")?;

        // Start all pipelines
        for (name, pipeline) in &self.pipelines {
            if let Err(e) = pipeline.start() {
                error!("Failed to start pipeline {}: {}", name, e);
                self.health
                    .set_component(
                        name,
                        yama_container_sdk::health::ComponentStatus::unhealthy(e.to_string()),
                    )
                    .await;
            } else {
                self.health
                    .set_component(
                        name,
                        yama_container_sdk::health::ComponentStatus::healthy(),
                    )
                    .await;
            }
        }

        // Set up frame callbacks
        for (name, pipeline) in &self.pipelines {
            let appsink = pipeline.appsink()?;
            let event_bus = self.event_bus.clone();
            let source_name = name.clone();

            appsink.set_callbacks(
                gstreamer_app::AppSinkCallbacks::builder()
                    .new_sample(move |sink| {
                        match sink.pull_sample() {
                            Ok(sample) => {
                                if let Some(buffer) = sample.buffer() {
                                    let caps = sample.caps().expect("Sample without caps");
                                    let video_info =
                                        gstreamer_video::VideoInfo::from_caps(caps)
                                            .expect("Invalid caps");

                                    let meta = VideoFrameMeta {
                                        source_id: source_name.clone(),
                                        frame_number: 0,
                                        pts: None,
                                        width: video_info.width(),
                                        height: video_info.height(),
                                        format: PixelFormat::Nv12.into(),
                                        dma_buf_fd: -1,
                                        shm_name: String::new(),
                                        shm_offset: 0,
                                        shm_size: buffer.size() as u64,
                                    };

                                    let eb = event_bus.clone();
                                    let topic = format!("video.frame.{}", source_name);
                                    tokio::spawn(async move {
                                        if let Err(e) = eb.publish(&topic, meta).await {
                                            warn!("Failed to publish frame: {}", e);
                                        }
                                    });
                                }
                                Ok(gst::FlowSuccess::Ok)
                            }
                            Err(_) => Err(gst::FlowError::Error),
                        }
                    })
                    .build(),
            );
        }

        // Periodically report decoder statistics
        let mut stats_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = stats_interval.tick() => {
                    for (name, pipeline) in &self.pipelines {
                        let stats = pipeline.decoder_stats();
                        debug!(
                            "Pipeline {} stats: {} frames decoded, {} dropped, avg decode: {}us",
                            name, stats.frames_decoded, stats.frames_dropped, stats.avg_decode_time_us
                        );
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    // Handle events from event bus
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama Video Service (NVIDIA Jetson)");
    info!(
        "Using platform decoder: {}",
        create_decoder()?.capabilities().name
    );

    let config = Config::default();
    let service = VideoService::new(&config).await?;
    service.run().await
}
