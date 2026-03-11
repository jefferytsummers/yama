//! Tool executor with validation and rate limiting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::tools::{Tool, ToolContext, ToolRegistry, ToolResult};
use super::validator::ToolValidator;

/// Rate limit configuration for a tool.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum calls per window.
    pub max_calls: u32,
    /// Time window duration.
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_calls: 10,
            window: Duration::from_secs(60),
        }
    }
}

/// Rate limiter state for a single tool.
#[derive(Debug)]
struct RateLimitState {
    /// Timestamps of recent calls.
    calls: Vec<Instant>,
    /// Configuration.
    config: RateLimitConfig,
}

impl RateLimitState {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            calls: Vec::new(),
            config,
        }
    }

    /// Check if rate limit would be exceeded.
    fn would_exceed(&self) -> bool {
        let now = Instant::now();
        let window_start = now - self.config.window;
        let recent_calls = self.calls.iter().filter(|&&t| t > window_start).count();
        recent_calls >= self.config.max_calls as usize
    }

    /// Record a call.
    fn record_call(&mut self) {
        let now = Instant::now();
        self.calls.push(now);

        // Clean up old entries
        let window_start = now - self.config.window;
        self.calls.retain(|&t| t > window_start);
    }
}

/// Audit log entry for tool execution.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Tool name.
    pub tool_name: String,
    /// Input (may be redacted).
    pub input: Value,
    /// Whether execution succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Session ID if available.
    pub session_id: Option<String>,
}

/// Tool executor with validation, rate limiting, and audit logging.
pub struct ToolExecutor {
    /// Tool registry.
    registry: Arc<ToolRegistry>,
    /// Cached validators per tool.
    validators: HashMap<String, ToolValidator>,
    /// Rate limit state per tool.
    rate_limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
    /// Rate limit configs per tool (None = default).
    rate_limit_configs: HashMap<String, RateLimitConfig>,
    /// Audit log (in-memory, recent entries only).
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
    /// Maximum audit log size.
    max_audit_entries: usize,
    /// Whether to validate inputs.
    validate_inputs: bool,
    /// Whether to enable rate limiting.
    enable_rate_limiting: bool,
}

impl ToolExecutor {
    /// Create a new executor with the given registry.
    pub fn new(registry: ToolRegistry) -> Result<Self> {
        let registry = Arc::new(registry);

        // Pre-compile validators for all tools
        let mut validators = HashMap::new();
        for name in registry.list() {
            if let Some(tool) = registry.get(name) {
                let definition = tool.definition();
                let validator = ToolValidator::from_definition(&definition)
                    .with_context(|| format!("Failed to create validator for tool '{}'", name))?;
                validators.insert(name.to_string(), validator);
            }
        }

        Ok(Self {
            registry,
            validators,
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_configs: HashMap::new(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            max_audit_entries: 1000,
            validate_inputs: true,
            enable_rate_limiting: true,
        })
    }

    /// Set a custom rate limit for a specific tool.
    pub fn with_rate_limit(mut self, tool_name: &str, config: RateLimitConfig) -> Self {
        self.rate_limit_configs.insert(tool_name.to_string(), config);
        self
    }

    /// Disable input validation (for testing).
    pub fn without_validation(mut self) -> Self {
        self.validate_inputs = false;
        self
    }

    /// Disable rate limiting.
    pub fn without_rate_limiting(mut self) -> Self {
        self.enable_rate_limiting = false;
        self
    }

    /// Set maximum audit log entries.
    pub fn with_max_audit_entries(mut self, max: usize) -> Self {
        self.max_audit_entries = max;
        self
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        tool_name: &str,
        input: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult> {
        self.execute_with_session(tool_name, input, ctx, None).await
    }

    /// Execute a tool with session tracking.
    pub async fn execute_with_session(
        &self,
        tool_name: &str,
        input: Value,
        ctx: ToolContext,
        session_id: Option<String>,
    ) -> Result<ToolResult> {
        let start = Instant::now();

        // Get the tool
        let tool = self
            .registry
            .get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;

        // Validate input
        if self.validate_inputs {
            if let Some(validator) = self.validators.get(tool_name) {
                validator.validate_or_err(&input).with_context(|| {
                    format!("Input validation failed for tool '{}'", tool_name)
                })?;
            }
        }

        // Check rate limit
        if self.enable_rate_limiting {
            self.check_rate_limit(tool_name).await?;
        }

        debug!(tool = tool_name, "Executing tool");

        // Execute the tool
        let result = tool.execute(input.clone(), ctx).await;

        let elapsed = start.elapsed().as_millis() as u64;

        // Record audit entry
        let audit_entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            tool_name: tool_name.to_string(),
            input: self.redact_sensitive(&input),
            success: result.is_ok() && result.as_ref().map(|r| r.success).unwrap_or(false),
            error: result
                .as_ref()
                .err()
                .map(|e| e.to_string())
                .or_else(|| {
                    result
                        .as_ref()
                        .ok()
                        .and_then(|r| r.error.clone())
                }),
            execution_time_ms: elapsed,
            session_id,
        };

        self.record_audit(audit_entry).await;

        // Record rate limit call
        if self.enable_rate_limiting {
            self.record_rate_limit_call(tool_name).await;
        }

        result
    }

    /// Check if a tool call would exceed rate limits.
    async fn check_rate_limit(&self, tool_name: &str) -> Result<()> {
        let mut rate_limits = self.rate_limits.write().await;

        let state = rate_limits.entry(tool_name.to_string()).or_insert_with(|| {
            let config = self
                .rate_limit_configs
                .get(tool_name)
                .cloned()
                .unwrap_or_default();
            RateLimitState::new(config)
        });

        if state.would_exceed() {
            warn!(
                tool = tool_name,
                max_calls = state.config.max_calls,
                window_secs = state.config.window.as_secs(),
                "Rate limit exceeded"
            );
            return Err(anyhow::anyhow!(
                "Rate limit exceeded for tool '{}': max {} calls per {} seconds",
                tool_name,
                state.config.max_calls,
                state.config.window.as_secs()
            ));
        }

        Ok(())
    }

    /// Record a rate limit call.
    async fn record_rate_limit_call(&self, tool_name: &str) {
        let mut rate_limits = self.rate_limits.write().await;
        if let Some(state) = rate_limits.get_mut(tool_name) {
            state.record_call();
        }
    }

    /// Record an audit entry.
    async fn record_audit(&self, entry: AuditEntry) {
        let mut log = self.audit_log.write().await;

        if entry.success {
            info!(
                tool = %entry.tool_name,
                execution_time_ms = entry.execution_time_ms,
                "Tool executed successfully"
            );
        } else {
            error!(
                tool = %entry.tool_name,
                error = ?entry.error,
                "Tool execution failed"
            );
        }

        log.push(entry);

        // Trim old entries
        if log.len() > self.max_audit_entries {
            let excess = log.len() - self.max_audit_entries;
            log.drain(0..excess);
        }
    }

    /// Redact sensitive fields from input for audit logging.
    fn redact_sensitive(&self, input: &Value) -> Value {
        // For now, just clone the input
        // In production, we might want to redact certain fields
        input.clone()
    }

    /// Get recent audit entries.
    pub async fn get_audit_log(&self, limit: Option<usize>) -> Vec<AuditEntry> {
        let log = self.audit_log.read().await;
        let limit = limit.unwrap_or(100).min(log.len());
        log.iter().rev().take(limit).cloned().collect()
    }

    /// List available tools.
    pub fn list_tools(&self) -> Vec<&str> {
        self.registry.list()
    }

    /// Get tool definition by name.
    pub fn get_tool_definition(&self, name: &str) -> Option<super::tools::ToolDefinition> {
        self.registry.get(name).map(|t| t.definition())
    }

    /// Get all tool definitions as JSON schemas.
    pub fn get_tool_schemas(&self) -> Vec<Value> {
        self.registry.to_json_schemas()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::ToolRegistry;

    async fn create_test_executor() -> ToolExecutor {
        let registry = ToolRegistry::with_builtins();
        ToolExecutor::new(registry).unwrap()
    }

    fn create_test_context() -> ToolContext {
        ToolContext::without_db()
            .with_project("test-project")
            .with_library("test-library")
    }

    #[tokio::test]
    async fn test_executor_list_tools() {
        let executor = create_test_executor().await;
        let tools = executor.list_tools();

        assert!(tools.contains(&"search_videos"));
        assert!(tools.contains(&"extract_clip"));
        assert!(tools.contains(&"compare_frames"));
        assert!(tools.contains(&"summarize_video"));
        assert!(tools.contains(&"get_keyframes"));
        assert!(tools.contains(&"get_transcript"));
    }

    #[tokio::test]
    async fn test_executor_get_tool_definition() {
        let executor = create_test_executor().await;

        let def = executor.get_tool_definition("search_videos");
        assert!(def.is_some());
        assert_eq!(def.unwrap().name, "search_videos");

        let def = executor.get_tool_definition("nonexistent");
        assert!(def.is_none());
    }

    #[tokio::test]
    async fn test_executor_unknown_tool() {
        let executor = create_test_executor().await;
        let ctx = create_test_context();

        let result = executor
            .execute("nonexistent_tool", serde_json::json!({}), ctx)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_executor_validation_failure() {
        let executor = create_test_executor().await;
        let ctx = create_test_context();

        // Missing required 'query' field for search_videos
        let result = executor
            .execute("search_videos", serde_json::json!({}), ctx)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("validation"));
    }

    #[tokio::test]
    async fn test_executor_without_validation() {
        let registry = ToolRegistry::with_builtins();
        let executor = ToolExecutor::new(registry).unwrap().without_validation();
        let ctx = create_test_context();

        // Should pass without validation even with empty input
        // (tool will still fail internally due to missing video_id)
        let result = executor
            .execute("search_videos", serde_json::json!({}), ctx)
            .await;

        // The tool itself will return an error for missing video_id,
        // but it won't be a validation error
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().contains("validation"));
    }

    #[tokio::test]
    async fn test_executor_rate_limiting() {
        let registry = ToolRegistry::with_builtins();
        let executor = ToolExecutor::new(registry)
            .unwrap()
            .without_validation()
            .with_rate_limit(
                "search_videos",
                RateLimitConfig {
                    max_calls: 2,
                    window: Duration::from_secs(60),
                },
            );

        // Record two calls manually
        {
            let mut rate_limits = executor.rate_limits.write().await;
            let state = rate_limits
                .entry("search_videos".to_string())
                .or_insert_with(|| {
                    RateLimitState::new(RateLimitConfig {
                        max_calls: 2,
                        window: Duration::from_secs(60),
                    })
                });
            state.record_call();
            state.record_call();
        }

        let ctx = create_test_context();

        // Third call should be rate limited
        let result = executor
            .execute(
                "search_videos",
                serde_json::json!({"query": "test"}),
                ctx,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Rate limit"));
    }

    #[tokio::test]
    async fn test_executor_audit_log() {
        let executor = create_test_executor().await.without_validation();
        let ctx = create_test_context();

        // Execute a tool (will fail due to missing required params, but audit should record)
        let _ = executor
            .execute(
                "search_videos",
                serde_json::json!({"query": "test search"}),
                ctx,
            )
            .await;

        let audit = executor.get_audit_log(Some(10)).await;
        assert!(!audit.is_empty());
        assert_eq!(audit[0].tool_name, "search_videos");
    }

    #[tokio::test]
    async fn test_get_tool_schemas() {
        let executor = create_test_executor().await;
        let schemas = executor.get_tool_schemas();

        assert!(!schemas.is_empty());
        // Each schema should have the function calling format
        for schema in &schemas {
            assert!(schema["function"]["name"].is_string());
            assert!(schema["function"]["parameters"].is_object());
        }
    }
}
