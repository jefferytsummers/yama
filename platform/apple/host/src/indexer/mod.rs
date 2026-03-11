//! Video indexing module.
//!
//! This module provides tools for indexing video content:
//! - [`KeyframeExtractor`] - Extract representative frames from videos
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::indexer::{KeyframeExtractor, KeyframeConfig};
//!
//! let config = KeyframeConfig::default();
//! let extractor = KeyframeExtractor::new(config);
//!
//! let keyframes = extractor.extract(Path::new("video.mp4"), None).await?;
//! for kf in keyframes {
//!     println!("Frame at {}ms: {:?}", kf.timestamp_ms, kf.image_path);
//! }
//! ```

pub mod keyframe_extractor;

pub use keyframe_extractor::{
    ExtractionProgress, Keyframe, KeyframeConfig, KeyframeExtractor,
};
