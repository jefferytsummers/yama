//! TTRA (local inference) backend.
//!
//! TTRA provides OpenAI-compatible API endpoints for local inference
//! using llama.cpp or other backends.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

use crate::context::{ContentBlock, Message, MessageContent, MessageRole};
use crate::tools::ToolRegistry;

use super::{LlmBackend, LlmConfig, LlmResponse};

/// TTRA local inference backend.
pub struct TtraBackend {
    config: LlmConfig,
    client: Client,
}

impl TtraBackend {
    /// Create a new TTRA backend.
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Convert internal messages to OpenAI format.
    fn convert_messages(&self, messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                };

                let content = match &msg.content {
                    MessageContent::Text(text) => json!(text),
                    MessageContent::Structured(blocks) => {
                        let content_parts: Vec<Value> = blocks
                            .iter()
                            .map(|b| match b {
                                ContentBlock::Text { text } => {
                                    json!({"type": "text", "text": text})
                                }
                                ContentBlock::ToolUse { id, name, input } => {
                                    json!({
                                        "type": "tool_use",
                                        "id": id,
                                        "name": name,
                                        "input": input
                                    })
                                }
                                ContentBlock::ToolResult {
                                    tool_use_id,
                                    content,
                                    is_error,
                                } => {
                                    json!({
                                        "type": "tool_result",
                                        "tool_use_id": tool_use_id,
                                        "content": content,
                                        "is_error": is_error
                                    })
                                }
                                ContentBlock::Image { .. } => {
                                    // TTRA may not support images
                                    json!({"type": "text", "text": "[image]"})
                                }
                            })
                            .collect();
                        json!(content_parts)
                    }
                };

                json!({
                    "role": role,
                    "content": content
                })
            })
            .collect()
    }

    /// Convert tool definitions to OpenAI format.
    fn convert_tools(&self, tools: &ToolRegistry) -> Vec<Value> {
        tools
            .definitions()
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    }
                })
            })
            .collect()
    }
}

impl LlmBackend for TtraBackend {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &ToolRegistry,
    ) -> Result<mpsc::Receiver<LlmResponse>> {
        let (tx, rx) = mpsc::channel(100);

        let api_messages = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "stream": true,
            "messages": api_messages
        });

        if !api_tools.is_empty() {
            body["tools"] = json!(api_tools);
        }

        let endpoint = if self.config.endpoint.is_empty() {
            "http://localhost:8080/v1/chat/completions"
        } else {
            &self.config.endpoint
        };

        debug!("Sending request to TTRA API at {}", endpoint);
        trace!("Request body: {}", serde_json::to_string_pretty(&body)?);

        let mut request = self
            .client
            .post(endpoint)
            .header("content-type", "application/json");

        if let Some(api_key) = &self.config.api_key {
            request = request.header("authorization", format!("Bearer {api_key}"));
        }

        let response = request
            .json(&body)
            .send()
            .await
            .context("Failed to send request to TTRA")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("TTRA API error: {} - {}", status, error_body);
            let _ = tx
                .send(LlmResponse::Error {
                    message: format!("API error: {status}"),
                })
                .await;
            return Ok(rx);
        }

        // Process SSE stream
        let mut stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete SSE events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            for line in event_data.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        let _ = tx
                                            .send(LlmResponse::Complete {
                                                stop_reason: "end_turn".to_string(),
                                            })
                                            .await;
                                        return;
                                    }

                                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                                        if let Some(choice) = chunk.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                let _ = tx
                                                    .send(LlmResponse::TextDelta {
                                                        text: content.clone(),
                                                    })
                                                    .await;
                                            }

                                            if let Some(tool_calls) = &choice.delta.tool_calls {
                                                for tool_call in tool_calls {
                                                    if let Some(function) = &tool_call.function {
                                                        let input: Value =
                                                            serde_json::from_str(
                                                                function.arguments.as_deref().unwrap_or("{}"),
                                                            )
                                                            .unwrap_or(json!({}));

                                                        let _ = tx
                                                            .send(LlmResponse::ToolUse {
                                                                id: tool_call
                                                                    .id
                                                                    .clone()
                                                                    .unwrap_or_default(),
                                                                name: function
                                                                    .name
                                                                    .clone()
                                                                    .unwrap_or_default(),
                                                                input,
                                                            })
                                                            .await;
                                                    }
                                                }
                                            }

                                            if let Some(finish_reason) = &choice.finish_reason {
                                                let _ = tx
                                                    .send(LlmResponse::Complete {
                                                        stop_reason: finish_reason.clone(),
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        let _ = tx
                            .send(LlmResponse::Error {
                                message: e.to_string(),
                            })
                            .await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn backend_name(&self) -> &str {
        "ttra"
    }
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}
