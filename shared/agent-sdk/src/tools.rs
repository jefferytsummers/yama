//! Tool system for agent capabilities.
//!
//! Tools are functions that agents can call to interact with the outside world.
//! Each tool has a name, description, and JSON schema for its parameters.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, instrument};

use crate::runtime::ToolConfig;

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolResult {
    /// Successful result with content.
    Success(String),
    /// Error result with message.
    Error(String),
}

impl ToolResult {
    /// Create a success result.
    pub fn success(content: impl Into<String>) -> Self {
        Self::Success(content.into())
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(message.into())
    }

    /// Check if this is a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Check if this is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get the content, whether success or error.
    pub fn content(&self) -> &str {
        match self {
            Self::Success(s) | Self::Error(s) => s,
        }
    }
}

/// Tool definition for the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (must be unique).
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub input_schema: Value,
}

/// A tool that can be executed.
pub trait Tool: Send + Sync {
    /// Get the tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given input.
    fn execute(
        &self,
        input: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send + '_>>;
}

/// Request to execute a tool.
#[derive(Debug, Clone)]
pub struct ToolRequest {
    /// Unique ID for this tool use.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool input.
    pub input: Value,
    /// Channel to send the result.
    pub result_tx: mpsc::Sender<ToolResult>,
}

/// Registry of available tools.
pub struct ToolRegistry {
    /// Registered tools by name.
    tools: HashMap<String, Arc<dyn Tool>>,
    /// Tool definitions for the LLM.
    definitions: Vec<ToolDefinition>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            definitions: Vec::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: impl Tool + 'static) {
        let def = tool.definition();
        let name = def.name.clone();
        self.definitions.push(def);
        self.tools.insert(name, Arc::new(tool));
    }

    /// Register a tool from configuration.
    ///
    /// This creates a proxy tool that forwards requests to a handler service.
    pub fn register_from_config(&mut self, config: &ToolConfig) -> Result<()> {
        let schema: Value = serde_json::from_str(&config.parameters_schema)?;

        let tool = ProxyTool {
            name: config.name.clone(),
            description: config.description.clone(),
            input_schema: schema,
            handler: config.handler.clone(),
        };

        self.register(tool);
        Ok(())
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Get all tool definitions.
    pub fn definitions(&self) -> &[ToolDefinition] {
        &self.definitions
    }

    /// Check if a tool exists.
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool by name.
    #[instrument(skip(self, input), fields(tool_name = %name))]
    pub async fn execute(&self, name: &str, input: Value) -> ToolResult {
        debug!("Executing tool");

        match self.tools.get(name) {
            Some(tool) => tool.execute(input).await,
            None => ToolResult::error(format!("Unknown tool: {name}")),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A proxy tool that forwards requests to a handler service.
struct ProxyTool {
    name: String,
    description: String,
    input_schema: Value,
    handler: String,
}

impl Tool for ProxyTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }

    fn execute(
        &self,
        input: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send + '_>> {
        let handler = self.handler.clone();
        let name = self.name.clone();

        Box::pin(async move {
            // TODO: Actually send request to handler service
            debug!("Proxy tool {} forwarding to handler {}", name, handler);
            ToolResult::error(format!(
                "Proxy tool {} not yet implemented (handler: {})",
                name, handler
            ))
        })
    }
}

/// Built-in tool: get current time.
pub struct GetTimeTool;

impl Tool for GetTimeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_time".to_string(),
            description: "Get the current date and time in ISO 8601 format".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    fn execute(
        &self,
        _input: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send + '_>> {
        Box::pin(async {
            use std::time::{SystemTime, UNIX_EPOCH};

            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();

            // Simple ISO 8601 timestamp
            let secs = duration.as_secs();
            let timestamp = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                1970 + secs / 31536000,
                (secs % 31536000) / 2592000 + 1,
                (secs % 2592000) / 86400 + 1,
                (secs % 86400) / 3600,
                (secs % 3600) / 60,
                secs % 60
            );

            ToolResult::success(timestamp)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(GetTimeTool);

        assert!(registry.contains("get_time"));
        assert_eq!(registry.len(), 1);

        let result = registry.execute("get_time", Value::Null).await;
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("unknown", Value::Null).await;
        assert!(result.is_error());
    }

    #[test]
    fn test_tool_result() {
        let success = ToolResult::success("hello");
        assert!(success.is_success());
        assert_eq!(success.content(), "hello");

        let error = ToolResult::error("oops");
        assert!(error.is_error());
        assert_eq!(error.content(), "oops");
    }
}
