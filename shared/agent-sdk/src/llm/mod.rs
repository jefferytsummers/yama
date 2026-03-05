//! LLM backend abstractions.
//!
//! This module provides a trait for LLM backends and implementations
//! for various providers (TTRA, Anthropic, OpenAI, llama.cpp).

mod anthropic;
mod openai;
mod ttra;

pub use anthropic::AnthropicBackend;
pub use openai::OpenAiBackend;
pub use ttra::TtraBackend;

use crate::context::Message;
use crate::tools::ToolRegistry;
use anyhow::Result;
use serde_json::Value;
use tokio::sync::mpsc;

/// Response from an LLM backend.
#[derive(Debug, Clone)]
pub enum LlmResponse {
    /// Text chunk from the model.
    TextDelta { text: String },
    /// Tool use request from the model.
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    /// Generation complete.
    Complete { stop_reason: String },
    /// Error occurred.
    Error { message: String },
}

/// Trait for LLM backends.
pub trait LlmBackend: Send + Sync {
    /// Send a chat request and receive streaming responses.
    fn chat(
        &self,
        messages: &[Message],
        tools: &ToolRegistry,
    ) -> impl std::future::Future<Output = Result<mpsc::Receiver<LlmResponse>>> + Send;

    /// Get the model name.
    fn model(&self) -> &str;

    /// Get the backend name.
    fn backend_name(&self) -> &str;
}

/// Configuration for LLM backends.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// API endpoint URL.
    pub endpoint: String,
    /// API key (if required).
    pub api_key: Option<String>,
    /// Model identifier.
    pub model: String,
    /// Sampling temperature.
    pub temperature: f32,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Additional options.
    pub options: std::collections::HashMap<String, String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: None,
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            options: std::collections::HashMap::new(),
        }
    }
}
