//! Frame sampling for live video streams.
//!
//! Rate-limits frame sampling per video source to avoid overwhelming
//! the VLM with too many frames. Supports adaptive sampling that adjusts
//! the sample interval based on observed inference latency.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use image::DynamicImage;
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use crate::config::LiveStreamConfig;

/// Minimum sample interval to prevent overwhelming the system.
const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(100);

/// Maximum sample interval to ensure some frames are analyzed.
const MAX_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);

/// EWMA smoothing factor for latency tracking (0.0-1.0, higher = more responsive).
const EWMA_ALPHA: f64 = 0.2;

/// How much slower than target latency triggers interval increase.
const SLOWDOWN_THRESHOLD: f64 = 1.2;

/// How much faster than target latency triggers interval decrease.
const SPEEDUP_THRESHOLD: f64 = 0.8;

/// Configuration for a single video source.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Source identifier.
    pub source_id: String,
    /// Sampling interval for this source.
    pub sample_interval: Duration,
    /// Analysis prompt for this source.
    pub prompt: String,
    /// Whether analysis is enabled.
    pub enabled: bool,
    /// Whether adaptive sampling is enabled.
    pub adaptive: bool,
    /// Target latency for adaptive sampling.
    pub target_latency: Duration,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            source_id: String::new(),
            sample_interval: Duration::from_secs(1),
            prompt: String::new(),
            enabled: true,
            adaptive: true,
            target_latency: Duration::from_millis(500),
        }
    }
}

/// Adaptive sampling state for a source.
#[derive(Debug)]
struct AdaptiveState {
    /// Exponentially weighted moving average of inference latency.
    ewma_latency: Duration,
    /// Current adaptive interval (may differ from config).
    current_interval: Duration,
    /// Number of adjustments made.
    adjustments: u64,
    /// Last adjustment direction (true = increased, false = decreased).
    last_increased: bool,
}

impl AdaptiveState {
    fn new(initial_interval: Duration) -> Self {
        Self {
            ewma_latency: Duration::ZERO,
            current_interval: initial_interval,
            adjustments: 0,
            last_increased: false,
        }
    }

    /// Update the EWMA latency and adjust interval if needed.
    fn update(&mut self, inference_latency: Duration, target_latency: Duration) {
        // Update EWMA
        if self.ewma_latency == Duration::ZERO {
            self.ewma_latency = inference_latency;
        } else {
            let alpha = EWMA_ALPHA;
            let new_ms = alpha * inference_latency.as_secs_f64() * 1000.0
                + (1.0 - alpha) * self.ewma_latency.as_secs_f64() * 1000.0;
            self.ewma_latency = Duration::from_secs_f64(new_ms / 1000.0);
        }

        // Check if adjustment is needed
        let ratio = self.ewma_latency.as_secs_f64() / target_latency.as_secs_f64();

        if ratio > SLOWDOWN_THRESHOLD {
            // Inference is slow, increase interval (sample less frequently)
            let new_interval = Duration::from_secs_f64(
                (self.current_interval.as_secs_f64() * 1.5).min(MAX_SAMPLE_INTERVAL.as_secs_f64()),
            );
            if new_interval != self.current_interval {
                debug!(
                    "Adaptive: slowing down, interval {:?} -> {:?} (EWMA latency: {:?})",
                    self.current_interval, new_interval, self.ewma_latency
                );
                self.current_interval = new_interval;
                self.adjustments += 1;
                self.last_increased = true;
            }
        } else if ratio < SPEEDUP_THRESHOLD {
            // Inference is fast, decrease interval (sample more frequently)
            let new_interval = Duration::from_secs_f64(
                (self.current_interval.as_secs_f64() * 0.9).max(MIN_SAMPLE_INTERVAL.as_secs_f64()),
            );
            if new_interval != self.current_interval {
                debug!(
                    "Adaptive: speeding up, interval {:?} -> {:?} (EWMA latency: {:?})",
                    self.current_interval, new_interval, self.ewma_latency
                );
                self.current_interval = new_interval;
                self.adjustments += 1;
                self.last_increased = false;
            }
        }
    }
}

/// State for a single video source.
struct SourceState {
    /// Configuration for this source.
    config: SourceConfig,
    /// Last time a frame was sampled.
    last_sample: Option<Instant>,
    /// Number of frames sampled.
    frames_sampled: u64,
    /// Number of frames skipped.
    frames_skipped: u64,
    /// Adaptive sampling state (if enabled).
    adaptive_state: Option<AdaptiveState>,
}

/// Frame sampler for rate-limiting live video analysis.
pub struct FrameSampler {
    /// Per-source state.
    sources: Arc<RwLock<HashMap<String, SourceState>>>,
    /// Default configuration.
    default_config: LiveStreamConfig,
}

/// A sampled frame ready for analysis.
#[derive(Debug)]
pub struct SampledFrame {
    /// Source identifier.
    pub source_id: String,
    /// Frame number.
    pub frame_number: u64,
    /// Presentation timestamp.
    pub pts: Option<Duration>,
    /// The image data.
    pub image: DynamicImage,
    /// Prompt to use for analysis.
    pub prompt: String,
}

impl FrameSampler {
    /// Create a new frame sampler with default configuration.
    pub fn new(config: LiveStreamConfig) -> Self {
        Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Register a video source for sampling.
    pub async fn register_source(&self, config: SourceConfig) {
        let mut sources = self.sources.write().await;
        let source_id = config.source_id.clone();
        let adaptive = config.adaptive;
        let sample_interval = config.sample_interval;

        let adaptive_state = if adaptive {
            Some(AdaptiveState::new(sample_interval))
        } else {
            None
        };

        sources.insert(
            source_id.clone(),
            SourceState {
                config,
                last_sample: None,
                frames_sampled: 0,
                frames_skipped: 0,
                adaptive_state,
            },
        );

        debug!(
            "Registered source for sampling: {} (adaptive: {})",
            source_id, adaptive
        );
    }

    /// Register a source with default configuration.
    pub async fn register_source_default(&self, source_id: &str) {
        let config = SourceConfig {
            source_id: source_id.to_string(),
            sample_interval: Duration::from_millis(self.default_config.sample_interval_ms),
            prompt: self.default_config.default_prompt.clone(),
            enabled: true,
            adaptive: true, // Enable adaptive sampling by default
            target_latency: Duration::from_millis(500),
        };

        self.register_source(config).await;
    }

    /// Unregister a video source.
    pub async fn unregister_source(&self, source_id: &str) {
        let mut sources = self.sources.write().await;
        sources.remove(source_id);
        debug!("Unregistered source: {}", source_id);
    }

    /// Check if a frame should be sampled and update state.
    ///
    /// Returns the sampling configuration if the frame should be processed,
    /// or None if it should be skipped.
    pub async fn should_sample(&self, source_id: &str) -> Option<SourceConfig> {
        let mut sources = self.sources.write().await;

        let state = sources.get_mut(source_id)?;

        if !state.config.enabled {
            state.frames_skipped += 1;
            return None;
        }

        let now = Instant::now();

        // Use adaptive interval if available, otherwise use config interval
        let sample_interval = state
            .adaptive_state
            .as_ref()
            .map(|s| s.current_interval)
            .unwrap_or(state.config.sample_interval);

        let should_sample = match state.last_sample {
            Some(last) => now.duration_since(last) >= sample_interval,
            None => true, // First frame
        };

        if should_sample {
            state.last_sample = Some(now);
            state.frames_sampled += 1;
            trace!(
                "Sampling frame from {}: {} sampled, {} skipped (interval: {:?})",
                source_id,
                state.frames_sampled,
                state.frames_skipped,
                sample_interval
            );
            Some(state.config.clone())
        } else {
            state.frames_skipped += 1;
            None
        }
    }

    /// Report inference latency for adaptive sampling.
    ///
    /// Call this after each inference completes to allow the sampler
    /// to adjust the sampling rate based on observed performance.
    pub async fn report_latency(&self, source_id: &str, inference_latency: Duration) {
        let mut sources = self.sources.write().await;

        if let Some(state) = sources.get_mut(source_id) {
            if let Some(ref mut adaptive) = state.adaptive_state {
                adaptive.update(inference_latency, state.config.target_latency);
            }
        }
    }

    /// Get the current adaptive interval for a source.
    pub async fn current_interval(&self, source_id: &str) -> Option<Duration> {
        let sources = self.sources.read().await;
        sources.get(source_id).map(|state| {
            state
                .adaptive_state
                .as_ref()
                .map(|s| s.current_interval)
                .unwrap_or(state.config.sample_interval)
        })
    }

    /// Process a frame if it should be sampled.
    ///
    /// Returns a `SampledFrame` if the frame was sampled, None otherwise.
    pub async fn process_frame(
        &self,
        source_id: &str,
        frame_number: u64,
        pts: Option<Duration>,
        image: DynamicImage,
    ) -> Option<SampledFrame> {
        let config = self.should_sample(source_id).await?;

        Some(SampledFrame {
            source_id: source_id.to_string(),
            frame_number,
            pts,
            image,
            prompt: config.prompt,
        })
    }

    /// Update configuration for a source.
    pub async fn update_source(&self, config: SourceConfig) -> bool {
        let mut sources = self.sources.write().await;

        if let Some(state) = sources.get_mut(&config.source_id) {
            state.config = config;
            true
        } else {
            false
        }
    }

    /// Enable or disable a source.
    pub async fn set_source_enabled(&self, source_id: &str, enabled: bool) -> bool {
        let mut sources = self.sources.write().await;

        if let Some(state) = sources.get_mut(source_id) {
            state.config.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get statistics for a source.
    pub async fn get_stats(&self, source_id: &str) -> Option<SourceStats> {
        let sources = self.sources.read().await;
        let state = sources.get(source_id)?;

        let (current_interval, ewma_latency, adjustments) = state
            .adaptive_state
            .as_ref()
            .map(|a| (a.current_interval, Some(a.ewma_latency), a.adjustments))
            .unwrap_or((state.config.sample_interval, None, 0));

        Some(SourceStats {
            source_id: source_id.to_string(),
            frames_sampled: state.frames_sampled,
            frames_skipped: state.frames_skipped,
            enabled: state.config.enabled,
            sample_interval: current_interval,
            adaptive_enabled: state.config.adaptive,
            ewma_latency,
            adaptive_adjustments: adjustments,
        })
    }

    /// Get all registered source IDs.
    pub async fn source_ids(&self) -> Vec<String> {
        let sources = self.sources.read().await;
        sources.keys().cloned().collect()
    }

    /// Check if a source is registered.
    pub async fn has_source(&self, source_id: &str) -> bool {
        let sources = self.sources.read().await;
        sources.contains_key(source_id)
    }

    /// Get the number of active sources.
    pub async fn active_source_count(&self) -> usize {
        let sources = self.sources.read().await;
        sources.values().filter(|s| s.config.enabled).count()
    }
}

/// Statistics for a video source.
#[derive(Debug, Clone)]
pub struct SourceStats {
    /// Source identifier.
    pub source_id: String,
    /// Total frames sampled.
    pub frames_sampled: u64,
    /// Total frames skipped.
    pub frames_skipped: u64,
    /// Whether sampling is enabled.
    pub enabled: bool,
    /// Current sample interval (may be adapted).
    pub sample_interval: Duration,
    /// Whether adaptive sampling is enabled.
    pub adaptive_enabled: bool,
    /// EWMA inference latency (if adaptive).
    pub ewma_latency: Option<Duration>,
    /// Number of adaptive adjustments made.
    pub adaptive_adjustments: u64,
}

impl SourceStats {
    /// Calculate the sampling rate.
    pub fn sample_rate(&self) -> f64 {
        let total = self.frames_sampled + self.frames_skipped;
        if total == 0 {
            0.0
        } else {
            self.frames_sampled as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    fn default_live_config() -> LiveStreamConfig {
        LiveStreamConfig {
            enabled: true,
            sample_interval_ms: 100, // 100ms for faster tests
            default_prompt: "Test prompt".to_string(),
            max_concurrent_sources: 4,
            auto_start: false,
            auto_start_sources: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_register_source() {
        let sampler = FrameSampler::new(default_live_config());

        sampler.register_source_default("camera-1").await;

        assert!(sampler.has_source("camera-1").await);
        assert!(!sampler.has_source("camera-2").await);
    }

    #[tokio::test]
    async fn test_should_sample_first_frame() {
        let sampler = FrameSampler::new(default_live_config());
        sampler.register_source_default("camera-1").await;

        // First frame should always be sampled
        let config = sampler.should_sample("camera-1").await;
        assert!(config.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let sampler = FrameSampler::new(default_live_config());
        sampler.register_source_default("camera-1").await;

        // First frame should be sampled
        assert!(sampler.should_sample("camera-1").await.is_some());

        // Immediate second frame should be skipped
        assert!(sampler.should_sample("camera-1").await.is_none());

        // Wait for interval
        sleep(Duration::from_millis(150)).await;

        // Now should be sampled again
        assert!(sampler.should_sample("camera-1").await.is_some());
    }

    #[tokio::test]
    async fn test_source_stats() {
        let sampler = FrameSampler::new(default_live_config());
        sampler.register_source_default("camera-1").await;

        // Sample first frame
        sampler.should_sample("camera-1").await;
        // Skip second frame (too soon)
        sampler.should_sample("camera-1").await;

        let stats = sampler.get_stats("camera-1").await.unwrap();
        assert_eq!(stats.frames_sampled, 1);
        assert_eq!(stats.frames_skipped, 1);
    }

    #[tokio::test]
    async fn test_disable_source() {
        let sampler = FrameSampler::new(default_live_config());
        sampler.register_source_default("camera-1").await;

        // Disable source
        sampler.set_source_enabled("camera-1", false).await;

        // Should not sample when disabled
        assert!(sampler.should_sample("camera-1").await.is_none());
    }
}
