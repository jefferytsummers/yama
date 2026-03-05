//! Yama Code Executor Tool
//!
//! Provides sandboxed code execution for agents in multiple languages.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_container_sdk::{EventBusClient, HealthReporter};

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Language {
    Python,
    JavaScript,
    Bash,
}

impl Language {
    fn extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::Bash => "sh",
        }
    }

    fn command(&self) -> &'static str {
        match self {
            Language::Python => "python3",
            Language::JavaScript => "node",
            Language::Bash => "bash",
        }
    }
}

/// Code execution request.
#[derive(Debug, Clone, Deserialize)]
struct ExecuteRequest {
    request_id: String,
    language: Language,
    code: String,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30
}

/// Code execution result.
#[derive(Debug, Clone, Serialize)]
struct ExecuteResult {
    request_id: String,
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    error: Option<String>,
    duration_ms: u64,
}

/// Service configuration.
#[derive(Debug, Clone)]
struct Config {
    event_bus_socket: String,
    workspace: PathBuf,
    max_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: std::env::var("YAMA_EVENT_BUS")
                .unwrap_or_else(|_| "/tmp/yama-event.sock".to_string()),
            workspace: std::env::var("WORKSPACE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/workspace")),
            max_timeout_secs: 60,
        }
    }
}

/// Code executor service.
struct CodeExecutorService {
    config: Config,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
    active_executions: Arc<RwLock<usize>>,
}

impl CodeExecutorService {
    async fn new(config: Config) -> Result<Self> {
        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "tool-code-executor")
                .await
                .context("Failed to connect to event bus")?,
        );

        let health = HealthReporter::new(event_bus.clone(), "tool-code-executor");

        Ok(Self {
            config,
            event_bus,
            health,
            active_executions: Arc::new(RwLock::new(0)),
        })
    }

    async fn run(self) -> Result<()> {
        info!("Starting code executor service");

        self.health.healthy().await;
        self.health.start().await?;

        // Subscribe to execution requests
        self.event_bus
            .subscribe(&["tool.code_executor.execute"])
            .await?;

        info!("Code executor ready");

        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn execute(&self, request: ExecuteRequest) -> ExecuteResult {
        let start = std::time::Instant::now();
        let request_id = request.request_id.clone();

        // Increment active executions
        *self.active_executions.write().await += 1;

        // Create temp file for code
        let file_path = self.config.workspace.join(format!(
            "exec_{}.{}",
            request.request_id,
            request.language.extension()
        ));

        let result = self.run_code(&request, &file_path).await;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&file_path).await;

        // Decrement active executions
        *self.active_executions.write().await -= 1;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((stdout, stderr, exit_code)) => ExecuteResult {
                request_id,
                success: exit_code == 0,
                stdout,
                stderr,
                exit_code: Some(exit_code),
                error: None,
                duration_ms,
            },
            Err(e) => ExecuteResult {
                request_id,
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                error: Some(e.to_string()),
                duration_ms,
            },
        }
    }

    async fn run_code(
        &self,
        request: &ExecuteRequest,
        file_path: &PathBuf,
    ) -> Result<(String, String, i32)> {
        // Write code to file
        tokio::fs::write(file_path, &request.code)
            .await
            .context("Failed to write code file")?;

        // Build command
        let mut cmd = Command::new(request.language.command());
        cmd.arg(file_path)
            .args(&request.args)
            .current_dir(&self.config.workspace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Limit timeout
        let timeout_secs = request.timeout_secs.min(self.config.max_timeout_secs);

        // Execute with timeout
        let child = cmd.spawn().context("Failed to spawn process")?;

        let output = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
            .await
            .context("Execution timed out")?
            .context("Failed to wait for process")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama Code Executor Tool");

    let config = Config::default();
    let service = CodeExecutorService::new(config).await?;
    service.run().await
}
