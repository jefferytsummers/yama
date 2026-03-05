//! Anthropic API backend.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

use crate::context::{ContentBlock, Message, MessageContent, MessageRole};
use crate::tools::ToolRegistry;

use super::{LlmBackend, LlmConfig, LlmResponse};

/// Anthropic API backend.
pub struct AnthropicBackend {
    config: LlmConfig,
    client: Client,
}

impl AnthropicBackend {
    /// Create a new Anthropic backend.
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Convert internal messages to Anthropic format.
    fn convert_messages(&self, messages: &[Message]) -> (String, Vec<Value>) {
        let mut system = String::new();
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    if let MessageContent::Text(text) = &msg.content {
                        system = text.clone();
                    }
                }
                MessageRole::User => {
                    let content = match &msg.content {
                        MessageContent::Text(text) => json!([{"type": "text", "text": text}]),
                        MessageContent::Structured(blocks) => {
                            let api_blocks: Vec<Value> = blocks
                                .iter()
                                .map(|b| match b {
                                    ContentBlock::Text { text } => {
                                        json!({"type": "text", "text": text})
                                    }
                                    ContentBlock::Image { source, media_type } => {
                                        match source {
                                            crate::context::ImageSource::Base64 { data } => {
                                                json!({
                                                    "type": "image",
                                                    "source": {
                                                        "type": "base64",
                                                        "media_type": media_type,
                                                        "data": data
                                                    }
                                                })
                                            }
                                            crate::context::ImageSource::Url { url } => {
                                                json!({
                                                    "type": "image",
                                                    "source": {
                                                        "type": "url",
                                                        "url": url
                                                    }
                                                })
                                            }
                                        }
                                    }
                                    _ => json!({"type": "text", "text": ""}),
                                })
                                .collect();
                            json!(api_blocks)
                        }
                    };
                    api_messages.push(json!({
                        "role": "user",
                        "content": content
                    }));
                }
                MessageRole::Assistant => {
                    let content = match &msg.content {
                        MessageContent::Text(text) => json!([{"type": "text", "text": text}]),
                        MessageContent::Structured(blocks) => {
                            let api_blocks: Vec<Value> = blocks
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
                                    _ => json!({"type": "text", "text": ""}),
                                })
                                .collect();
                            json!(api_blocks)
                        }
                    };
                    api_messages.push(json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
                MessageRole::Tool => {
                    if let MessageContent::Structured(blocks) = &msg.content {
                        let content: Vec<Value> = blocks
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::ToolResult {
                                    tool_use_id,
                                    content,
                                    is_error,
                                } => Some(json!({
                                    "type": "tool_result",
                                    "tool_use_id": tool_use_id,
                                    "content": content,
                                    "is_error": is_error
                                })),
                                _ => None,
                            })
                            .collect();
                        api_messages.push(json!({
                            "role": "user",
                            "content": content
                        }));
                    }
                }
            }
        }

        (system, api_messages)
    }

    /// Convert tool definitions to Anthropic format.
    fn convert_tools(&self, tools: &ToolRegistry) -> Vec<Value> {
        tools
            .definitions()
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema
                })
            })
            .collect()
    }
}

impl LlmBackend for AnthropicBackend {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &ToolRegistry,
    ) -> Result<mpsc::Receiver<LlmResponse>> {
        let (tx, rx) = mpsc::channel(100);

        let (system, api_messages) = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let mut body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "stream": true,
            "messages": api_messages
        });

        if !system.is_empty() {
            body["system"] = json!(system);
        }

        if !api_tools.is_empty() {
            body["tools"] = json!(api_tools);
        }

        let endpoint = if self.config.endpoint.is_empty() {
            "https://api.anthropic.com/v1/messages"
        } else {
            &self.config.endpoint
        };

        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Anthropic API key required")?;

        debug!("Sending request to Anthropic API");
        trace!("Request body: {}", serde_json::to_string_pretty(&body)?);

        let response = self
            .client
            .post(endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {} - {}", status, error_body);
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
            let mut current_tool_use: Option<(String, String, String)> = None;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete SSE events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            // Parse SSE event
                            for line in event_data.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(event) =
                                        serde_json::from_str::<StreamEvent>(data)
                                    {
                                        match event.r#type.as_str() {
                                            "content_block_start" => {
                                                if let Some(block) = event.content_block {
                                                    if block.r#type == "tool_use" {
                                                        current_tool_use = Some((
                                                            block.id.unwrap_or_default(),
                                                            block.name.unwrap_or_default(),
                                                            String::new(),
                                                        ));
                                                    }
                                                }
                                            }
                                            "content_block_delta" => {
                                                if let Some(delta) = event.delta {
                                                    match delta.r#type.as_str() {
                                                        "text_delta" => {
                                                            if let Some(text) = delta.text {
                                                                let _ = tx
                                                                    .send(LlmResponse::TextDelta {
                                                                        text,
                                                                    })
                                                                    .await;
                                                            }
                                                        }
                                                        "input_json_delta" => {
                                                            if let Some(ref mut tool) =
                                                                current_tool_use
                                                            {
                                                                if let Some(json) =
                                                                    delta.partial_json
                                                                {
                                                                    tool.2.push_str(&json);
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            "content_block_stop" => {
                                                if let Some((id, name, input_json)) =
                                                    current_tool_use.take()
                                                {
                                                    let input: Value =
                                                        serde_json::from_str(&input_json)
                                                            .unwrap_or(json!({}));
                                                    let _ = tx
                                                        .send(LlmResponse::ToolUse {
                                                            id,
                                                            name,
                                                            input,
                                                        })
                                                        .await;
                                                }
                                            }
                                            "message_stop" => {
                                                let stop_reason = event
                                                    .message
                                                    .and_then(|m| m.stop_reason)
                                                    .unwrap_or_else(|| "end_turn".to_string());
                                                let _ = tx
                                                    .send(LlmResponse::Complete { stop_reason })
                                                    .await;
                                            }
                                            "message_delta" => {
                                                if let Some(delta) = event.delta {
                                                    if let Some(stop_reason) = delta.stop_reason {
                                                        let _ = tx
                                                            .send(LlmResponse::Complete {
                                                                stop_reason,
                                                            })
                                                            .await;
                                                    }
                                                }
                                            }
                                            "error" => {
                                                let message = event
                                                    .error
                                                    .map(|e| e.message)
                                                    .unwrap_or_else(|| "Unknown error".to_string());
                                                let _ = tx
                                                    .send(LlmResponse::Error { message })
                                                    .await;
                                            }
                                            _ => {}
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
        "anthropic"
    }
}

#[derive(Debug, Deserialize)]
struct StreamEvent {
    r#type: String,
    #[serde(default)]
    delta: Option<Delta>,
    #[serde(default)]
    content_block: Option<ContentBlockInfo>,
    #[serde(default)]
    message: Option<MessageInfo>,
    #[serde(default)]
    error: Option<ErrorInfo>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    r#type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockInfo {
    r#type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageInfo {
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorInfo {
    message: String,
}
