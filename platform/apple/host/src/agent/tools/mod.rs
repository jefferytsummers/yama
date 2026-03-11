//! Agent tools for video operations.
//!
//! Tools are operations that agents can invoke to interact with video content.
//! Each tool has:
//! - A JSON schema definition for input validation
//! - An async execute method that performs the operation
//! - Structured output for the agent to process

pub mod compare;
pub mod extract;
pub mod keyframes;
pub mod search;
pub mod summarize;
pub mod transcript;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-exports
pub use compare::CompareFramesTool;
pub use extract::ExtractClipTool;
pub use keyframes::GetKeyframesTool;
pub use search::SearchVideosTool;
pub use summarize::SummarizeVideoTool;
pub use transcript::GetTranscriptTool;

/// Tool execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the execution was successful.
    pub success: bool,
    /// Result data (tool-specific).
    #[serde(default)]
    pub data: Value,
    /// Error message if not successful.
    #[serde(default)]
    pub error: Option<String>,
    /// Execution time in milliseconds.
    #[serde(default)]
    pub execution_time_ms: u64,
}

impl ToolResult {
    /// Create a successful result.
    pub fn ok(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).unwrap_or(Value::Null),
            error: None,
            execution_time_ms: 0,
        }
    }

    /// Create a failed result.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: Value::Null,
            error: Some(message.into()),
            execution_time_ms: 0,
        }
    }

    /// Set execution time.
    pub fn with_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }
}

/// Parameter definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    /// Parameter name.
    pub name: String,
    /// JSON Schema type (string, number, boolean, array, object).
    #[serde(rename = "type")]
    pub param_type: String,
    /// Description of the parameter.
    pub description: String,
    /// Whether the parameter is required.
    #[serde(default)]
    pub required: bool,
    /// Default value if not provided.
    #[serde(default)]
    pub default: Option<Value>,
    /// Enum values if restricted.
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<Value>>,
}

impl ParameterDef {
    /// Create a required string parameter.
    pub fn required_string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "string".to_string(),
            description: description.into(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Create an optional string parameter.
    pub fn optional_string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "string".to_string(),
            description: description.into(),
            required: false,
            default: None,
            enum_values: None,
        }
    }

    /// Create a required number parameter.
    pub fn required_number(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            param_type: "number".to_string(),
            description: description.into(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Create an optional number parameter with default.
    pub fn optional_number(
        name: impl Into<String>,
        description: impl Into<String>,
        default: f64,
    ) -> Self {
        Self {
            name: name.into(),
            param_type: "number".to_string(),
            description: description.into(),
            required: false,
            default: Some(Value::Number(serde_json::Number::from_f64(default).unwrap())),
            enum_values: None,
        }
    }

    /// Create an optional integer parameter with default.
    pub fn optional_integer(
        name: impl Into<String>,
        description: impl Into<String>,
        default: i64,
    ) -> Self {
        Self {
            name: name.into(),
            param_type: "integer".to_string(),
            description: description.into(),
            required: false,
            default: Some(Value::Number(default.into())),
            enum_values: None,
        }
    }

    /// Create an optional boolean parameter.
    pub fn optional_bool(
        name: impl Into<String>,
        description: impl Into<String>,
        default: bool,
    ) -> Self {
        Self {
            name: name.into(),
            param_type: "boolean".to_string(),
            description: description.into(),
            required: false,
            default: Some(Value::Bool(default)),
            enum_values: None,
        }
    }
}

/// Tool definition with JSON schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Parameter definitions.
    pub parameters: Vec<ParameterDef>,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
        }
    }

    /// Add a parameter.
    pub fn with_param(mut self, param: ParameterDef) -> Self {
        self.parameters.push(param);
        self
    }

    /// Convert to JSON Schema format (for OpenAI/Anthropic function calling).
    pub fn to_json_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), Value::String(param.param_type.clone()));
            prop.insert(
                "description".to_string(),
                Value::String(param.description.clone()),
            );

            if let Some(ref default) = param.default {
                prop.insert("default".to_string(), default.clone());
            }

            if let Some(ref enum_vals) = param.enum_values {
                prop.insert("enum".to_string(), Value::Array(enum_vals.clone()));
            }

            properties.insert(param.name.clone(), Value::Object(prop));

            if param.required {
                required.push(Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }
        })
    }
}

/// Execution context for tools.
#[derive(Clone)]
pub struct ToolContext {
    /// Database pool (optional for testing).
    pub db: Option<sqlx::SqlitePool>,
    /// Current project ID (if any).
    pub project_id: Option<String>,
    /// Current library ID (if any).
    pub library_id: Option<String>,
    /// Additional context data.
    pub extra: HashMap<String, Value>,
}

impl ToolContext {
    /// Create a new context with a database pool.
    pub fn new(db: sqlx::SqlitePool) -> Self {
        Self {
            db: Some(db),
            project_id: None,
            library_id: None,
            extra: HashMap::new(),
        }
    }

    /// Create a context without a database (for testing).
    pub fn without_db() -> Self {
        Self {
            db: None,
            project_id: None,
            library_id: None,
            extra: HashMap::new(),
        }
    }

    /// Set the project ID.
    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set the library ID.
    pub fn with_library(mut self, library_id: impl Into<String>) -> Self {
        self.library_id = Some(library_id.into());
        self
    }

    /// Get the database pool, returning an error if not available.
    pub fn require_db(&self) -> anyhow::Result<&sqlx::SqlitePool> {
        self.db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database pool not available"))
    }
}

/// Trait for implementing tools.
pub trait Tool: Send + Sync {
    /// Get the tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given input.
    fn execute(
        &self,
        input: Value,
        ctx: ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult>> + Send + '_>>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with all built-in tools.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();

        registry.register(Arc::new(SearchVideosTool::new()));
        registry.register(Arc::new(ExtractClipTool::new()));
        registry.register(Arc::new(CompareFramesTool::new()));
        registry.register(Arc::new(SummarizeVideoTool::new()));
        registry.register(Arc::new(GetKeyframesTool::new()));
        registry.register(Arc::new(GetTranscriptTool::new()));

        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all tool names.
    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get all tool definitions.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Get tool definitions as JSON schemas (for LLM function calling).
    pub fn to_json_schemas(&self) -> Vec<Value> {
        self.definitions()
            .iter()
            .map(|d| d.to_json_schema())
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result() {
        let ok = ToolResult::ok(serde_json::json!({"count": 5}));
        assert!(ok.success);
        assert!(ok.error.is_none());

        let err = ToolResult::err("Something went wrong");
        assert!(!err.success);
        assert_eq!(err.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_parameter_def() {
        let param = ParameterDef::required_string("query", "Search query");
        assert_eq!(param.name, "query");
        assert_eq!(param.param_type, "string");
        assert!(param.required);
    }

    #[test]
    fn test_tool_definition_schema() {
        let def = ToolDefinition::new("test_tool", "A test tool")
            .with_param(ParameterDef::required_string("query", "The query"))
            .with_param(ParameterDef::optional_integer("limit", "Max results", 10));

        let schema = def.to_json_schema();

        assert_eq!(schema["function"]["name"], "test_tool");
        assert!(schema["function"]["parameters"]["properties"]["query"].is_object());
        assert!(schema["function"]["parameters"]["required"]
            .as_array()
            .unwrap()
            .contains(&Value::String("query".to_string())));
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::with_builtins();

        assert!(registry.get("search_videos").is_some());
        assert!(registry.get("extract_clip").is_some());
        assert!(registry.get("nonexistent").is_none());

        let names = registry.list();
        assert!(names.contains(&"search_videos"));
    }
}
