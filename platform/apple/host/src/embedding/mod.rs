//! Embedding generation module.
//!
//! This module provides embedding generation for:
//! - Visual embeddings (CLIP)
//! - Text embeddings (MiniLM)
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::embedding::{ClipEmbedder, ClipConfig};
//!
//! let config = ClipConfig::default();
//! let embedder = ClipEmbedder::new("model.onnx", config)?;
//!
//! let embedding = embedder.encode_bytes(&jpeg_data)?;
//! println!("Embedding dim: {}", embedding.len());
//! ```

pub mod clip;
pub mod preprocessor;
pub mod text;

pub use clip::{ClipConfig, ClipEmbedder, ClipModel, ClipTextEmbedder, CLIP_EMBEDDING_DIM};
pub use preprocessor::{
    cosine_similarity, l2_normalize, ImagePreprocessor, CLIP_IMAGE_SIZE, IMAGENET_MEAN,
    IMAGENET_STD,
};
pub use text::{TextEmbedder, TextEmbedderConfig, TEXT_EMBEDDING_DIM};
