//! Model management module.
//!
//! This module provides:
//! - [`ModelRegistry`] - Catalog of available models
//! - [`ModelCache`] - Download and cache management
//! - [`ModelManager`] - High-level API for model lifecycle
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::models::{ModelManager, ModelType};
//!
//! let manager = ModelManager::with_defaults();
//!
//! // Get recommended CLIP model
//! let clip = manager.get_recommended(ModelType::Clip).unwrap();
//! println!("Using CLIP model: {}", clip.name);
//!
//! // Ensure model is downloaded
//! let path = manager.ensure_model(&clip.id, None).await?;
//!
//! // Load into memory (with LRU eviction)
//! manager.load_model(&clip.id).await?;
//! ```

pub mod cache;
pub mod manager;
pub mod registry;

pub use cache::{DownloadProgress, ModelCache};
pub use manager::{LoadedModel, ModelManager, ModelManagerConfig, ModelManagerStats};
pub use registry::{
    ModelDefinition, ModelFormat, ModelRegistry, ModelSource, ModelType, Quantization,
};
