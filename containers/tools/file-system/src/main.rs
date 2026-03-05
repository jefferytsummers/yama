//! Yama File System Tool
//!
//! Provides file system operations for agents with sandboxed access.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use glob::glob;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use yama_container_sdk::{EventBusClient, HealthReporter};

/// File system operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum FileOperation {
    Read { path: String },
    Write { path: String, content: String },
    List { path: String, pattern: Option<String> },
    Exists { path: String },
    Mkdir { path: String },
    Remove { path: String },
    Move { from: String, to: String },
    Copy { from: String, to: String },
}

/// File operation request.
#[derive(Debug, Clone, Deserialize)]
struct FileRequest {
    request_id: String,
    #[serde(flatten)]
    operation: FileOperation,
}

/// File operation result.
#[derive(Debug, Clone, Serialize)]
struct FileResult {
    request_id: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<FileInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// File information.
#[derive(Debug, Clone, Serialize)]
struct FileInfo {
    path: String,
    name: String,
    is_dir: bool,
    size: u64,
}

/// Service configuration.
#[derive(Debug, Clone)]
struct Config {
    event_bus_socket: String,
    workspace: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: std::env::var("YAMA_EVENT_BUS")
                .unwrap_or_else(|_| "/tmp/yama-event.sock".to_string()),
            workspace: std::env::var("WORKSPACE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/workspace")),
        }
    }
}

/// File system service.
struct FileSystemService {
    config: Config,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
}

impl FileSystemService {
    async fn new(config: Config) -> Result<Self> {
        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "tool-file-system")
                .await
                .context("Failed to connect to event bus")?,
        );

        let health = HealthReporter::new(event_bus.clone(), "tool-file-system");

        Ok(Self {
            config,
            event_bus,
            health,
        })
    }

    async fn run(self) -> Result<()> {
        info!("Starting file system service");

        self.health.healthy().await;
        self.health.start().await?;

        self.event_bus
            .subscribe(&["tool.file_system.*"])
            .await?;

        info!("File system service ready");

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Resolve and validate a path within the workspace.
    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        let requested = PathBuf::from(path);

        // Make path absolute relative to workspace
        let full_path = if requested.is_absolute() {
            requested
        } else {
            self.config.workspace.join(requested)
        };

        // Canonicalize to resolve .. and symlinks
        let canonical = full_path
            .canonicalize()
            .unwrap_or_else(|_| full_path.clone());

        // Ensure path is within workspace
        if !canonical.starts_with(&self.config.workspace) {
            anyhow::bail!("Path is outside workspace");
        }

        Ok(canonical)
    }

    async fn handle_request(&self, request: FileRequest) -> FileResult {
        let request_id = request.request_id.clone();

        match self.execute_operation(request.operation).await {
            Ok(result) => result.with_request_id(request_id),
            Err(e) => FileResult {
                request_id,
                success: false,
                content: None,
                files: None,
                exists: None,
                error: Some(e.to_string()),
            },
        }
    }

    async fn execute_operation(&self, operation: FileOperation) -> Result<PartialResult> {
        match operation {
            FileOperation::Read { path } => {
                let full_path = self.resolve_path(&path)?;
                let content = tokio::fs::read_to_string(&full_path)
                    .await
                    .context("Failed to read file")?;
                Ok(PartialResult::Content(content))
            }

            FileOperation::Write { path, content } => {
                let full_path = self.resolve_path(&path)?;
                tokio::fs::write(&full_path, content)
                    .await
                    .context("Failed to write file")?;
                Ok(PartialResult::Success)
            }

            FileOperation::List { path, pattern } => {
                let full_path = self.resolve_path(&path)?;

                let mut files = Vec::new();

                if let Some(pattern) = pattern {
                    let glob_pattern = full_path.join(pattern);
                    for entry in glob(glob_pattern.to_str().unwrap_or("*"))? {
                        if let Ok(path) = entry {
                            if let Ok(metadata) = path.metadata() {
                                files.push(FileInfo {
                                    path: path.to_string_lossy().to_string(),
                                    name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                    is_dir: metadata.is_dir(),
                                    size: metadata.len(),
                                });
                            }
                        }
                    }
                } else {
                    let mut entries = tokio::fs::read_dir(&full_path).await?;
                    while let Some(entry) = entries.next_entry().await? {
                        if let Ok(metadata) = entry.metadata().await {
                            files.push(FileInfo {
                                path: entry.path().to_string_lossy().to_string(),
                                name: entry.file_name().to_string_lossy().to_string(),
                                is_dir: metadata.is_dir(),
                                size: metadata.len(),
                            });
                        }
                    }
                }

                Ok(PartialResult::Files(files))
            }

            FileOperation::Exists { path } => {
                let full_path = self.resolve_path(&path)?;
                let exists = full_path.exists();
                Ok(PartialResult::Exists(exists))
            }

            FileOperation::Mkdir { path } => {
                let full_path = self.resolve_path(&path)?;
                tokio::fs::create_dir_all(&full_path)
                    .await
                    .context("Failed to create directory")?;
                Ok(PartialResult::Success)
            }

            FileOperation::Remove { path } => {
                let full_path = self.resolve_path(&path)?;
                if full_path.is_dir() {
                    tokio::fs::remove_dir_all(&full_path).await?;
                } else {
                    tokio::fs::remove_file(&full_path).await?;
                }
                Ok(PartialResult::Success)
            }

            FileOperation::Move { from, to } => {
                let from_path = self.resolve_path(&from)?;
                let to_path = self.resolve_path(&to)?;
                tokio::fs::rename(&from_path, &to_path).await?;
                Ok(PartialResult::Success)
            }

            FileOperation::Copy { from, to } => {
                let from_path = self.resolve_path(&from)?;
                let to_path = self.resolve_path(&to)?;
                tokio::fs::copy(&from_path, &to_path).await?;
                Ok(PartialResult::Success)
            }
        }
    }
}

enum PartialResult {
    Success,
    Content(String),
    Files(Vec<FileInfo>),
    Exists(bool),
}

impl PartialResult {
    fn with_request_id(self, request_id: String) -> FileResult {
        match self {
            PartialResult::Success => FileResult {
                request_id,
                success: true,
                content: None,
                files: None,
                exists: None,
                error: None,
            },
            PartialResult::Content(content) => FileResult {
                request_id,
                success: true,
                content: Some(content),
                files: None,
                exists: None,
                error: None,
            },
            PartialResult::Files(files) => FileResult {
                request_id,
                success: true,
                content: None,
                files: Some(files),
                exists: None,
                error: None,
            },
            PartialResult::Exists(exists) => FileResult {
                request_id,
                success: true,
                content: None,
                files: None,
                exists: Some(exists),
                error: None,
            },
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

    info!("Starting Yama File System Tool");

    let config = Config::default();
    let service = FileSystemService::new(config).await?;
    service.run().await
}
