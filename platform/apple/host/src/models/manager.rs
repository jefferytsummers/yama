//! Model manager for loading and caching ML models.
//!
//! The ModelManager handles:
//! - Model registry and discovery
//! - Download and caching from HuggingFace
//! - LRU eviction with memory budget
//! - Model loading and unloading

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::cache::{DownloadProgress, ModelCache};
use super::registry::{ModelDefinition, ModelRegistry, ModelType};

/// Default memory budget (8GB).
const DEFAULT_MEMORY_BUDGET: u64 = 8 * 1024 * 1024 * 1024;

/// Model manager configuration.
#[derive(Debug, Clone)]
pub struct ModelManagerConfig {
    /// Memory budget for loaded models (bytes).
    pub memory_budget: u64,
    /// Cache directory.
    pub cache_dir: Option<PathBuf>,
    /// Whether to auto-download missing models.
    pub auto_download: bool,
}

impl Default for ModelManagerConfig {
    fn default() -> Self {
        Self {
            memory_budget: DEFAULT_MEMORY_BUDGET,
            cache_dir: None,
            auto_download: true,
        }
    }
}

/// A loaded model instance.
#[derive(Debug)]
pub struct LoadedModel {
    /// Model definition.
    pub definition: ModelDefinition,
    /// Path to the model file.
    pub path: PathBuf,
    /// Memory used by this model.
    pub memory_used: u64,
    /// When the model was loaded.
    pub loaded_at: Instant,
    /// Last time the model was accessed.
    pub last_accessed: Instant,
    /// Access count.
    pub access_count: u64,
}

impl LoadedModel {
    /// Update last accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// Model manager state.
struct ManagerState {
    /// Currently loaded models.
    loaded: HashMap<String, LoadedModel>,
    /// Total memory used.
    memory_used: u64,
}

/// Model manager for ML models.
pub struct ModelManager {
    /// Configuration.
    config: ModelManagerConfig,
    /// Model registry.
    registry: ModelRegistry,
    /// Model cache.
    cache: ModelCache,
    /// Manager state.
    state: RwLock<ManagerState>,
}

impl ModelManager {
    /// Create a new model manager.
    pub fn new(config: ModelManagerConfig) -> Self {
        let cache = config
            .cache_dir
            .as_ref()
            .map(|p| ModelCache::new(p))
            .unwrap_or_else(ModelCache::default_location);

        Self {
            config,
            registry: ModelRegistry::new(),
            cache,
            state: RwLock::new(ManagerState {
                loaded: HashMap::new(),
                memory_used: 0,
            }),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ModelManagerConfig::default())
    }

    /// Get the model registry.
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Get a mutable reference to the registry.
    pub fn registry_mut(&mut self) -> &mut ModelRegistry {
        &mut self.registry
    }

    /// Get the cache directory.
    pub fn cache_dir(&self) -> &std::path::Path {
        self.cache.cache_dir()
    }

    /// Check if a model is loaded.
    pub async fn is_loaded(&self, model_id: &str) -> bool {
        self.state.read().await.loaded.contains_key(model_id)
    }

    /// Get current memory usage.
    pub async fn memory_used(&self) -> u64 {
        self.state.read().await.memory_used
    }

    /// Get available memory budget.
    pub async fn memory_available(&self) -> u64 {
        let used = self.memory_used().await;
        self.config.memory_budget.saturating_sub(used)
    }

    /// Get list of loaded model IDs.
    pub async fn loaded_models(&self) -> Vec<String> {
        self.state.read().await.loaded.keys().cloned().collect()
    }

    /// Ensure a model is downloaded (but not loaded).
    pub async fn ensure_model(
        &self,
        model_id: &str,
        progress: Option<Box<dyn Fn(DownloadProgress) + Send>>,
    ) -> Result<PathBuf> {
        let definition = self
            .registry
            .get(model_id)
            .context("Model not found in registry")?;

        self.cache.ensure_model(definition, progress).await
    }

    /// Load a model into memory.
    ///
    /// If there's not enough memory, evicts least-recently-used models.
    pub async fn load_model(&self, model_id: &str) -> Result<()> {
        // Check if already loaded
        if self.is_loaded(model_id).await {
            // Just touch it
            let mut state = self.state.write().await;
            if let Some(model) = state.loaded.get_mut(model_id) {
                model.touch();
            }
            return Ok(());
        }

        // Get model definition
        let definition = self
            .registry
            .get(model_id)
            .context("Model not found in registry")?
            .clone();

        // Ensure downloaded
        let path = self
            .cache
            .ensure_model(&definition, None)
            .await
            .context("Failed to ensure model is downloaded")?;

        // Check memory requirements
        let required = definition.memory_bytes;
        if required > self.config.memory_budget {
            anyhow::bail!(
                "Model {} requires {}GB but budget is {}GB",
                model_id,
                required / (1024 * 1024 * 1024),
                self.config.memory_budget / (1024 * 1024 * 1024)
            );
        }

        // Evict if necessary
        while self.memory_available().await < required {
            if !self.evict_one().await? {
                anyhow::bail!("Cannot free enough memory for model {}", model_id);
            }
        }

        // Load the model
        let loaded = LoadedModel {
            definition,
            path,
            memory_used: required,
            loaded_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
        };

        let mut state = self.state.write().await;
        state.memory_used += required;
        state.loaded.insert(model_id.to_string(), loaded);

        info!(
            "Loaded model {} (memory: {}MB, total: {}MB)",
            model_id,
            required / (1024 * 1024),
            state.memory_used / (1024 * 1024)
        );

        Ok(())
    }

    /// Unload a model from memory.
    pub async fn unload_model(&self, model_id: &str) -> Result<bool> {
        let mut state = self.state.write().await;

        if let Some(model) = state.loaded.remove(model_id) {
            state.memory_used = state.memory_used.saturating_sub(model.memory_used);
            info!(
                "Unloaded model {} (freed: {}MB, total: {}MB)",
                model_id,
                model.memory_used / (1024 * 1024),
                state.memory_used / (1024 * 1024)
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Evict the least-recently-used model.
    async fn evict_one(&self) -> Result<bool> {
        let mut state = self.state.write().await;

        // Find LRU model
        let lru_id = state
            .loaded
            .iter()
            .min_by_key(|(_, m)| m.last_accessed)
            .map(|(id, _)| id.clone());

        if let Some(id) = lru_id {
            if let Some(model) = state.loaded.remove(&id) {
                state.memory_used = state.memory_used.saturating_sub(model.memory_used);
                warn!(
                    "Evicted model {} (freed: {}MB, reason: LRU)",
                    id,
                    model.memory_used / (1024 * 1024)
                );
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get model path (ensure downloaded first).
    pub async fn get_model_path(&self, model_id: &str) -> Result<PathBuf> {
        // Check if loaded
        {
            let state = self.state.read().await;
            if let Some(model) = state.loaded.get(model_id) {
                return Ok(model.path.clone());
            }
        }

        // Ensure downloaded
        self.ensure_model(model_id, None).await
    }

    /// Get model info.
    pub fn get_model_info(&self, model_id: &str) -> Option<&ModelDefinition> {
        self.registry.get(model_id)
    }

    /// Get recommended model for a type.
    pub fn get_recommended(&self, model_type: ModelType) -> Option<&ModelDefinition> {
        self.registry.recommended(model_type)
    }

    /// Get statistics.
    pub async fn stats(&self) -> ModelManagerStats {
        let state = self.state.read().await;

        ModelManagerStats {
            loaded_count: state.loaded.len(),
            memory_used: state.memory_used,
            memory_budget: self.config.memory_budget,
            registry_count: self.registry.all().len(),
        }
    }
}

/// Model manager statistics.
#[derive(Debug, Clone)]
pub struct ModelManagerStats {
    /// Number of loaded models.
    pub loaded_count: usize,
    /// Memory used by loaded models.
    pub memory_used: u64,
    /// Total memory budget.
    pub memory_budget: u64,
    /// Number of models in registry.
    pub registry_count: usize,
}

impl ModelManagerStats {
    /// Memory utilization percentage.
    pub fn memory_utilization(&self) -> f64 {
        if self.memory_budget > 0 {
            (self.memory_used as f64 / self.memory_budget as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = ModelManager::with_defaults();
        assert!(manager.registry().all().len() > 0);
    }

    #[tokio::test]
    async fn test_memory_tracking() {
        let manager = ModelManager::new(ModelManagerConfig {
            memory_budget: 1024 * 1024 * 1024, // 1GB
            ..Default::default()
        });

        assert_eq!(manager.memory_used().await, 0);
        assert_eq!(manager.memory_available().await, 1024 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_model_info() {
        let manager = ModelManager::with_defaults();

        let info = manager.get_model_info("whisper-base");
        assert!(info.is_some());
        assert_eq!(info.unwrap().model_type, ModelType::Whisper);
    }

    #[tokio::test]
    async fn test_recommended_model() {
        let manager = ModelManager::with_defaults();

        let clip = manager.get_recommended(ModelType::Clip);
        assert!(clip.is_some());
        assert!(clip.unwrap().recommended);
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = ModelManager::with_defaults();
        let stats = manager.stats().await;

        assert_eq!(stats.loaded_count, 0);
        assert_eq!(stats.memory_used, 0);
        assert!(stats.registry_count > 0);
    }
}
