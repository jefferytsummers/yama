//! OpenAI-compatible API backend.
//!
//! This can be used with OpenAI's API or any OpenAI-compatible endpoint.

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

/// OpenAI-compatible API backend.
pub struct OpenAiBackend {
    config: LlmConfig,
    client: Client,
}

impl OpenAiBackend {
    /// Create a new OpenAI backend.
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

                let mut message = json!({ "role": role });

                match &msg.content {
                    MessageContent::Text(text) => {
                        message["content"] = json!(text);
                    }
                    MessageContent::Structured(blocks) => {
                        // Check if this is a tool result
                        if msg.role == MessageRole::Tool {
                            if let Some(ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            }) = blocks.first()
                            {
                                message["tool_call_id"] = json!(tool_use_id);
                                message["content"] = json!(content);
                            }
                        } else {
                            // Convert to content array for other roles
                            let content_parts: Vec<Value> = blocks
                                .iter()
                                .filter_map(|b| match b {
                                    ContentBlock::Text { text } => {
                                        Some(json!({"type": "text", "text": text}))
                                    }
                                    ContentBlock::Image { source, media_type } => {
                                        match source {
                                            crate::context::ImageSource::Base64 { data } => {
                                                Some(json!({
                                                    "type": "image_url",
                                                    "image_url": {
                                                        "url": format!("data:{};base64,{}", media_type, data)
                                                    }
                                                }))
                                            }
                                            crate::context::ImageSource::Url { url } => {
                                                Some(json!({
                                                    "type": "image_url",
                                                    "image_url": {"url": url}
                                                }))
                                            }
                                        }
                                    }
                                    _ => None,
                                })
                                .collect();
                            message["content"] = json!(content_parts);

                            // Add tool calls if assistant message
                            if msg.role == MessageRole::Assistant {
                                let tool_calls: Vec<Value> = blocks
                                    .iter()
                                    .filter_map(|b| match b {
                                        ContentBlock::ToolUse { id, name, input } => Some(json!({
                                            "id": id,
                                            "type": "function",
                                            "function": {
                                                "name": name,
                                                "arguments": input.to_string()
                                            }
                                        })),
                                        _ => None,
                                    })
                                    .collect();
                                if !tool_calls.is_empty() {
                                    message["tool_calls"] = json!(tool_calls);
                                }
                            }
                        }
                    }
                }

                message
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

impl LlmBackend for OpenAiBackend {
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
            "https://api.openai.com/v1/chat/completions"
        } else {
            &self.config.endpoint
        };

        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("OpenAI API key required")?;

        debug!("Sending request to OpenAI API");
        trace!("Request body: {}", serde_json::to_string_pretty(&body)?);

        let response = self
            .client
            .post(endpoint)
            .header("authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("OpenAI API error: {} - {}", status, error_body);
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
            let mut tool_call_buffers: std::collections::HashMap<String, (String, String)> =
                std::collections::HashMap::new();

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
                                        // Emit any pending tool calls
                                        for (id, (name, args)) in tool_call_buffers.drain() {
                                            let input: Value =
                                                serde_json::from_str(&args).unwrap_or(json!({}));
                                            let _ = tx
                                                .send(LlmResponse::ToolUse { id, name, input })
                                                .await;
                                        }
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
                                                    let id = tool_call.id.clone().unwrap_or_else(
                                                        || format!("call_{}", tool_call.index),
                                                    );

                                                    let entry = tool_call_buffers
                                                        .entry(id)
                                                        .or_insert_with(|| (String::new(), String::new()));

                                                    if let Some(function) = &tool_call.function {
                                                        if let Some(name) = &function.name {
                                                            entry.0 = name.clone();
                                                        }
                                                        if let Some(args) = &function.arguments {
                                                            entry.1.push_str(args);
                                                        }
                                                    }
                                                }
                                            }

                                            if let Some(finish_reason) = &choice.finish_reason {
                                                // Emit any pending tool calls
                                                for (id, (name, args)) in tool_call_buffers.drain()
                                                {
                                                    let input: Value =
                                                        serde_json::from_str(&args)
                                                            .unwrap_or(json!({}));
                                                    let _ = tx
                                                        .send(LlmResponse::ToolUse {
                                                            id,
                                                            name,
                                                            input,
                                                        })
                                                        .await;
                                                }
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
        "openai"
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
    index: usize,
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
