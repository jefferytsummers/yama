//! Model download and cache management.
//!
//! Handles downloading models from HuggingFace and local caching.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;
use tracing::{debug, info, warn};

use super::registry::{ModelDefinition, ModelSource};

/// Default cache directory.
const DEFAULT_CACHE_DIR: &str = ".yama/models";

/// Model cache manager.
pub struct ModelCache {
    /// Cache directory path.
    cache_dir: PathBuf,
}

impl ModelCache {
    /// Create a new cache manager.
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Create with default cache directory (~/.yama/models).
    pub fn default_location() -> Self {
        let cache_dir = dirs_next::home_dir()
            .map(|h| h.join(DEFAULT_CACHE_DIR))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CACHE_DIR));

        Self { cache_dir }
    }

    /// Get the cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Ensure cache directory exists.
    pub async fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)
            .await
            .context("Failed to create cache directory")?;
        Ok(())
    }

    /// Get the local path for a model.
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        self.cache_dir.join(model_id)
    }

    /// Check if a model is cached.
    pub async fn is_cached(&self, model_id: &str) -> bool {
        let path = self.model_path(model_id);
        fs::try_exists(&path).await.unwrap_or(false)
    }

    /// Get cached model size.
    pub async fn cached_size(&self, model_id: &str) -> Option<u64> {
        let path = self.model_path(model_id);
        fs::metadata(&path).await.ok().map(|m| m.len())
    }

    /// Ensure a model is downloaded and cached.
    ///
    /// Returns the local path to the model.
    pub async fn ensure_model(
        &self,
        model: &ModelDefinition,
        progress: Option<Box<dyn Fn(DownloadProgress) + Send>>,
    ) -> Result<PathBuf> {
        self.ensure_dir().await?;

        let model_path = self.model_path(&model.id);

        // Check if already cached
        if self.is_cached(&model.id).await {
            // Verify size if known
            if model.size_bytes > 0 {
                if let Some(cached_size) = self.cached_size(&model.id).await {
                    // Allow 10% variance for compression differences
                    let min_size = model.size_bytes * 9 / 10;
                    let max_size = model.size_bytes * 11 / 10;
                    if cached_size >= min_size && cached_size <= max_size {
                        info!("Model {} already cached at {:?}", model.id, model_path);
                        return Ok(model_path);
                    } else {
                        warn!(
                            "Cached model {} size mismatch (expected ~{}, got {}), re-downloading",
                            model.id, model.size_bytes, cached_size
                        );
                    }
                }
            } else {
                info!("Model {} already cached at {:?}", model.id, model_path);
                return Ok(model_path);
            }
        }

        // Download based on source
        match &model.source {
            ModelSource::HuggingFace {
                repo_id,
                filename,
                revision,
            } => {
                self.download_huggingface(
                    repo_id,
                    filename.as_deref(),
                    revision.as_deref(),
                    &model_path,
                    model.size_bytes,
                    progress,
                )
                .await?;
            }
            ModelSource::Url(url) => {
                self.download_url(url, &model_path, model.size_bytes, progress)
                    .await?;
            }
            ModelSource::Local(path) => {
                let source = PathBuf::from(path);
                if !source.exists() {
                    anyhow::bail!("Local model not found: {:?}", source);
                }
                // Copy or symlink
                fs::copy(&source, &model_path)
                    .await
                    .context("Failed to copy local model")?;
            }
        }

        info!("Model {} downloaded to {:?}", model.id, model_path);
        Ok(model_path)
    }

    /// Download from HuggingFace Hub.
    async fn download_huggingface(
        &self,
        repo_id: &str,
        filename: Option<&str>,
        revision: Option<&str>,
        target: &Path,
        expected_size: u64,
        progress: Option<Box<dyn Fn(DownloadProgress) + Send>>,
    ) -> Result<()> {
        // Build HuggingFace URL
        let revision = revision.unwrap_or("main");
        let filename = filename.unwrap_or("model.safetensors");
        let url = format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            repo_id, revision, filename
        );

        info!("Downloading from HuggingFace: {}", url);
        self.download_url(&url, target, expected_size, progress)
            .await
    }

    /// Download from a URL.
    async fn download_url(
        &self,
        url: &str,
        target: &Path,
        expected_size: u64,
        progress: Option<Box<dyn Fn(DownloadProgress) + Send>>,
    ) -> Result<()> {
        let client = reqwest::Client::builder()
            .user_agent("yama/0.1")
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Download failed: HTTP {}", status);
        }

        let total_size = response
            .content_length()
            .or_else(|| if expected_size > 0 { Some(expected_size) } else { None });

        debug!("Download size: {:?}", total_size);

        // Create parent directory
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Create temp file
        let temp_path = target.with_extension("download");
        let mut file = fs::File::create(&temp_path)
            .await
            .context("Failed to create temp file")?;

        // Stream download
        use tokio::io::AsyncWriteExt;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_report = std::time::Instant::now();

        while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
            let chunk = chunk.context("Download stream error")?;
            file.write_all(&chunk).await.context("Write error")?;
            downloaded += chunk.len() as u64;

            // Report progress at most every 100ms
            if let Some(ref callback) = progress {
                if last_report.elapsed() > std::time::Duration::from_millis(100) {
                    callback(DownloadProgress {
                        downloaded,
                        total: total_size,
                        percent: total_size
                            .map(|t| (downloaded as f64 / t as f64) * 100.0)
                            .unwrap_or(0.0),
                    });
                    last_report = std::time::Instant::now();
                }
            }
        }

        file.flush().await?;
        drop(file);

        // Rename to final path
        fs::rename(&temp_path, target)
            .await
            .context("Failed to move downloaded file")?;

        // Final progress report
        if let Some(callback) = progress {
            callback(DownloadProgress {
                downloaded,
                total: Some(downloaded),
                percent: 100.0,
            });
        }

        Ok(())
    }

    /// Delete a cached model.
    pub async fn delete(&self, model_id: &str) -> Result<bool> {
        let path = self.model_path(model_id);
        if fs::try_exists(&path).await.unwrap_or(false) {
            fs::remove_file(&path)
                .await
                .context("Failed to delete model")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get total cache size in bytes.
    pub async fn total_size(&self) -> Result<u64> {
        let mut total = 0u64;

        if !fs::try_exists(&self.cache_dir).await.unwrap_or(false) {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    total += metadata.len();
                }
            }
        }

        Ok(total)
    }

    /// List cached models.
    pub async fn list_cached(&self) -> Result<Vec<String>> {
        let mut models = Vec::new();

        if !fs::try_exists(&self.cache_dir).await.unwrap_or(false) {
            return Ok(models);
        }

        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Skip temp files
                        if !name.ends_with(".download") {
                            models.push(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(models)
    }

    /// Clear all cached models.
    pub async fn clear(&self) -> Result<u64> {
        let mut removed = 0u64;

        if !fs::try_exists(&self.cache_dir).await.unwrap_or(false) {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    let size = metadata.len();
                    if fs::remove_file(entry.path()).await.is_ok() {
                        removed += size;
                    }
                }
            }
        }

        Ok(removed)
    }
}

/// Download progress information.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes (if known).
    pub total: Option<u64>,
    /// Percentage complete.
    pub percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_dir_creation() {
        let temp = TempDir::new().unwrap();
        let cache = ModelCache::new(temp.path().join("models"));
        cache.ensure_dir().await.unwrap();
        assert!(cache.cache_dir().exists());
    }

    #[tokio::test]
    async fn test_model_path() {
        let temp = TempDir::new().unwrap();
        let cache = ModelCache::new(temp.path());
        let path = cache.model_path("test-model");
        assert_eq!(path, temp.path().join("test-model"));
    }

    #[tokio::test]
    async fn test_is_cached() {
        let temp = TempDir::new().unwrap();
        let cache = ModelCache::new(temp.path());

        assert!(!cache.is_cached("test-model").await);

        // Create a dummy file
        fs::write(temp.path().join("test-model"), "dummy data")
            .await
            .unwrap();

        assert!(cache.is_cached("test-model").await);
    }
}
