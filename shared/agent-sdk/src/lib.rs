//! Yama Agent SDK
//!
//! This crate provides the runtime library for building LLM agents in the Yama system.
//! It handles the agent loop, tool execution, context management, and LLM backend
//! communication.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              AgentRuntime               │
//! │  ┌─────────────┐  ┌─────────────────┐  │
//! │  │   Context   │  │  ToolRegistry   │  │
//! │  │   Manager   │  │                 │  │
//! │  └─────────────┘  └─────────────────┘  │
//! │           │              │              │
//! │           ▼              ▼              │
//! │  ┌─────────────────────────────────┐   │
//! │  │         LLM Backend             │   │
//! │  │  (TTRA / Anthropic / OpenAI)    │   │
//! │  └─────────────────────────────────┘   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use yama_agent_sdk::{AgentRuntime, AgentConfig};
//! use yama_agent_sdk::llm::AnthropicBackend;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = AgentConfig::from_file("agent.toml")?;
//!     let backend = AnthropicBackend::new(&config.llm)?;
//!     let runtime = AgentRuntime::new(config, backend)?;
//!
//!     runtime.run().await?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod context;
pub mod llm;
pub mod runtime;
pub mod skills;
pub mod tools;

pub use context::{Context, ContextConfig, ContextManager};
pub use runtime::{AgentConfig, AgentRuntime};
pub use skills::{Skill, SkillLoader};
pub use tools::{Tool, ToolRegistry, ToolResult};
