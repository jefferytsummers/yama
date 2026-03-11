//! Agent system for Yama.
//!
//! This module provides the agent framework for video analysis:
//! - Agent presets with configurable system prompts and tools
//! - Tool definitions for video operations
//! - Tool executor with validation and rate limiting
//! - Chat session management with history
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::agent::{AgentPreset, PresetRegistry, ChatSession};
//!
//! let registry = PresetRegistry::load_builtin()?;
//! let preset = registry.get("video-analyst")?;
//!
//! let session = ChatSession::new(preset, db.clone(), vlm_service.clone());
//! let response = session.send_message("Describe this video").await?;
//! ```

pub mod executor;
pub mod presets;
pub mod session;
pub mod tools;
pub mod validator;

pub use executor::{AuditEntry, RateLimitConfig, ToolExecutor};
pub use presets::{AgentPreset, PresetId, PresetRegistry, ToolConfig};
pub use session::{ChatChunk, ChatSession, SessionConfig};
pub use tools::{
    CompareFramesTool, ExtractClipTool, GetKeyframesTool, GetTranscriptTool, ParameterDef,
    SearchVideosTool, SummarizeVideoTool, Tool, ToolContext, ToolDefinition, ToolRegistry,
    ToolResult,
};
pub use validator::{ToolValidator, ValidationError, ValidationResult};
