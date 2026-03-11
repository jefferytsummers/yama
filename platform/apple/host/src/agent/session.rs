//! Chat session management for agent conversations.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::db::chat_history::{ChatHistory, ChatMessageRow, ChatSessionRow, MessageRole, StoredToolCall};
use crate::inference::VlmInferenceService;

// Agent SDK imports for LLM interaction
use yama_agent_sdk::context::{ContentBlock, ImageSource, Message, MessageContent, MessageRole as SdkMessageRole};
use yama_agent_sdk::llm::{AnthropicBackend, LlmBackend, LlmConfig, LlmResponse};
use yama_agent_sdk::tools::{ToolDefinition as SdkToolDefinition, ToolRegistry as SdkToolRegistry};

use super::executor::ToolExecutor;
use super::presets::{AgentPreset, PresetRegistry};
use super::tools::ToolContext;

/// A streaming response chunk from the chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatChunk {
    /// Text content being streamed.
    Text(String),
    /// Tool call being made.
    ToolCall {
        /// Tool call ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool arguments.
        arguments: Value,
    },
    /// Tool call result.
    ToolResult {
        /// Tool call ID this result is for.
        tool_call_id: String,
        /// Whether the tool succeeded.
        success: bool,
        /// Result data or error message.
        content: String,
    },
    /// Final message complete.
    Done {
        /// Total token count (if available).
        token_count: Option<i64>,
    },
    /// Error occurred.
    Error(String),
}

/// Configuration for a chat session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum context messages to include.
    pub max_context_messages: usize,
    /// Whether to automatically generate titles.
    pub auto_title: bool,
    /// Whether to persist messages.
    pub persist: bool,
    /// Model override (uses preset default if None).
    pub model_override: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 50,
            auto_title: true,
            persist: true,
            model_override: None,
        }
    }
}

/// A chat session with an agent.
pub struct ChatSession {
    /// Session ID.
    id: String,
    /// Preset being used.
    preset: AgentPreset,
    /// Session configuration.
    config: SessionConfig,
    /// Chat history handler.
    history: Arc<ChatHistory>,
    /// VLM inference service.
    vlm_service: Arc<VlmInferenceService>,
    /// Tool executor.
    executor: Arc<ToolExecutor>,
    /// Tool context for execution.
    tool_context: ToolContext,
}

/// Simple wrapper tool for the SDK registry.
/// The actual execution is handled by our ToolExecutor, not through this wrapper.
struct SimpleToolWrapper {
    definition: SdkToolDefinition,
}

impl yama_agent_sdk::tools::Tool for SimpleToolWrapper {
    fn definition(&self) -> SdkToolDefinition {
        self.definition.clone()
    }

    fn execute(
        &self,
        _input: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = yama_agent_sdk::tools::ToolResult> + Send + '_>> {
        // This is a placeholder - actual execution is done through ToolExecutor
        Box::pin(async {
            yama_agent_sdk::tools::ToolResult::error("Tool execution is handled externally")
        })
    }
}

impl ChatSession {
    /// Create a new chat session.
    pub async fn new(
        preset: AgentPreset,
        history: Arc<ChatHistory>,
        vlm_service: Arc<VlmInferenceService>,
        executor: Arc<ToolExecutor>,
        tool_context: ToolContext,
        config: SessionConfig,
    ) -> Result<Self> {
        Self::new_with_details(preset, history, vlm_service, executor, tool_context, config, None, None).await
    }

    /// Create a new chat session with project ID and title.
    pub async fn new_with_details(
        preset: AgentPreset,
        history: Arc<ChatHistory>,
        vlm_service: Arc<VlmInferenceService>,
        executor: Arc<ToolExecutor>,
        tool_context: ToolContext,
        config: SessionConfig,
        project_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<Self> {
        let session_row = history
            .create_session(&preset.id.as_str(), project_id, title)
            .await
            .context("Failed to create session in database")?;

        // Add system prompt as first message if configured
        if config.persist && !preset.system_prompt.is_empty() {
            history
                .add_message(
                    &session_row.id,
                    MessageRole::System,
                    &preset.system_prompt,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .context("Failed to add system prompt")?;
        }

        Ok(Self {
            id: session_row.id,
            preset,
            config,
            history,
            vlm_service,
            executor,
            tool_context,
        })
    }

    /// Load an existing session.
    pub async fn load(
        session_id: &str,
        preset_registry: &PresetRegistry,
        history: Arc<ChatHistory>,
        vlm_service: Arc<VlmInferenceService>,
        executor: Arc<ToolExecutor>,
        tool_context: ToolContext,
        config: SessionConfig,
    ) -> Result<Self> {
        let session_row = history
            .get_session(session_id)
            .await
            .context("Failed to get session")?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let preset = preset_registry
            .get(&session_row.preset_id)
            .ok_or_else(|| anyhow::anyhow!("Preset not found: {}", session_row.preset_id))?
            .clone();

        Ok(Self {
            id: session_id.to_string(),
            preset,
            config,
            history,
            vlm_service,
            executor,
            tool_context,
        })
    }

    /// Get the session ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the preset.
    pub fn preset(&self) -> &AgentPreset {
        &self.preset
    }

    /// Get the session info from database.
    pub async fn get_info(&self) -> Result<Option<ChatSessionRow>> {
        self.history.get_session(&self.id).await
    }

    /// Get all messages in the session.
    pub async fn get_messages(&self) -> Result<Vec<ChatMessageRow>> {
        self.history.get_messages(&self.id).await
    }

    /// Update the session title.
    pub async fn set_title(&self, title: &str) -> Result<()> {
        self.history.update_session_title(&self.id, title).await
    }

    /// Send a message and get a streaming response.
    pub fn send_message(&self, content: String) -> mpsc::Receiver<ChatChunk> {
        let (tx, rx) = mpsc::channel(32);

        let session_id = self.id.clone();
        let preset = self.preset.clone();
        let config = self.config.clone();
        let history = self.history.clone();
        let vlm_service = self.vlm_service.clone();
        let executor = self.executor.clone();
        let tool_context = self.tool_context.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::process_message(
                tx.clone(),
                session_id,
                content,
                preset,
                config,
                history,
                vlm_service,
                executor,
                tool_context,
            )
            .await
            {
                error!("Error processing message: {}", e);
                let _ = tx.send(ChatChunk::Error(e.to_string())).await;
            }
        });

        rx
    }

    /// Internal message processing.
    async fn process_message(
        tx: mpsc::Sender<ChatChunk>,
        session_id: String,
        content: String,
        preset: AgentPreset,
        config: SessionConfig,
        history: Arc<ChatHistory>,
        vlm_service: Arc<VlmInferenceService>,
        executor: Arc<ToolExecutor>,
        tool_context: ToolContext,
    ) -> Result<()> {
        info!(session = %session_id, "Processing message");

        // Store user message
        if config.persist {
            history
                .add_message(&session_id, MessageRole::User, &content, None, None, None, None)
                .await
                .context("Failed to store user message")?;
        }

        // Build context from recent messages
        let context_messages = history
            .get_recent_messages(&session_id, config.max_context_messages as i64)
            .await
            .context("Failed to get context messages")?;

        // Convert to VLM format
        let messages: Vec<Value> = context_messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });

                if let Some(ref tool_calls) = m.tool_calls {
                    msg["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
                }

                if let Some(ref tool_call_id) = m.tool_call_id {
                    msg["tool_call_id"] = Value::String(tool_call_id.clone());
                }

                msg
            })
            .collect();

        // Get tool definitions if tools are enabled
        let tools: Option<Vec<Value>> = if !preset.tools.is_empty() {
            let tool_schemas: Vec<Value> = preset
                .tools
                .iter()
                .filter_map(|t| {
                    executor
                        .get_tool_definition(&t.name)
                        .map(|def| def.to_json_schema())
                })
                .collect();

            if tool_schemas.is_empty() {
                None
            } else {
                Some(tool_schemas)
            }
        } else {
            None
        };

        // Get model configuration
        let default_model = preset.model.preferred_model.clone().unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());
        let model = config.model_override.as_ref().unwrap_or(&default_model).clone();

        // Get API key from environment
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        // Convert context messages to SDK format
        let sdk_messages = Self::convert_to_sdk_messages(&context_messages, &preset.system_prompt);

        // Build SDK tool registry from executor definitions
        let sdk_tools = Self::build_sdk_tools(&executor, &preset);

        // Create LLM backend
        let llm_config = LlmConfig {
            endpoint: String::new(), // Use default Anthropic endpoint
            api_key,
            model: model.clone(),
            temperature: preset.model.temperature,
            max_tokens: preset.model.max_tokens,
            options: HashMap::new(),
        };

        // If no API key, fall back to placeholder response
        if llm_config.api_key.is_none() {
            warn!("No ANTHROPIC_API_KEY set, using placeholder response");
            let response_content = format!(
                "I received your message: '{}'. Set ANTHROPIC_API_KEY to enable AI responses.",
                content
            );
            tx.send(ChatChunk::Text(response_content.clone())).await?;

            if config.persist {
                history
                    .add_message(
                        &session_id,
                        MessageRole::Assistant,
                        &response_content,
                        None,
                        None,
                        Some(&model),
                        None,
                    )
                    .await
                    .context("Failed to store assistant message")?;
            }

            tx.send(ChatChunk::Done { token_count: None }).await?;
            return Ok(());
        }

        let backend = match AnthropicBackend::new(llm_config) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to create Anthropic backend: {}", e);
                tx.send(ChatChunk::Error(format!("Failed to initialize LLM: {}", e))).await?;
                return Ok(());
            }
        };

        // Agentic loop: continue until we get end_turn
        let mut loop_messages = sdk_messages;
        let mut accumulated_text = String::new();
        let mut tool_uses: Vec<(String, String, Value)> = Vec::new();

        'agentic_loop: loop {
            debug!(message_count = loop_messages.len(), "Calling LLM");

            // Call LLM
            let mut response_rx = match backend.chat(&loop_messages, &sdk_tools).await {
                Ok(rx) => rx,
                Err(e) => {
                    error!("LLM call failed: {}", e);
                    tx.send(ChatChunk::Error(format!("LLM call failed: {}", e))).await?;
                    break 'agentic_loop;
                }
            };

            // Process streaming responses
            let mut current_turn_text = String::new();
            let mut current_turn_tools: Vec<(String, String, Value)> = Vec::new();
            let mut stop_reason = String::new();

            while let Some(response) = response_rx.recv().await {
                match response {
                    LlmResponse::TextDelta { text } => {
                        current_turn_text.push_str(&text);
                        accumulated_text.push_str(&text);
                        tx.send(ChatChunk::Text(text)).await?;
                    }
                    LlmResponse::ToolUse { id, name, input } => {
                        info!(tool = %name, id = %id, "Tool use requested");
                        tx.send(ChatChunk::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: input.clone(),
                        }).await?;
                        current_turn_tools.push((id, name, input));
                    }
                    LlmResponse::Complete { stop_reason: reason } => {
                        debug!(stop_reason = %reason, "LLM response complete");
                        stop_reason = reason;
                    }
                    LlmResponse::Error { message } => {
                        error!("LLM error: {}", message);
                        tx.send(ChatChunk::Error(message)).await?;
                        break 'agentic_loop;
                    }
                }
            }

            // If there were tool uses, execute them and continue the loop
            if !current_turn_tools.is_empty() {
                // Add assistant message with tool uses to context
                let assistant_blocks = Self::build_assistant_blocks(&current_turn_text, &current_turn_tools);
                loop_messages.push(Message {
                    role: SdkMessageRole::Assistant,
                    content: MessageContent::Structured(assistant_blocks),
                    token_count: 0,
                });

                // Execute tools and add results
                let mut tool_results = Vec::new();
                for (tool_id, tool_name, tool_input) in &current_turn_tools {
                    // Execute the tool
                    let result = executor
                        .execute_with_session(&tool_name, tool_input.clone(), tool_context.clone(), Some(session_id.clone()))
                        .await;

                    let (content, is_error) = match result {
                        Ok(r) => {
                            let success = r.success;
                            let content = if r.success {
                                serde_json::to_string(&r.data).unwrap_or_default()
                            } else {
                                r.error.unwrap_or_default()
                            };
                            tx.send(ChatChunk::ToolResult {
                                tool_call_id: tool_id.clone(),
                                success,
                                content: content.clone(),
                            }).await?;
                            (content, !success)
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            tx.send(ChatChunk::ToolResult {
                                tool_call_id: tool_id.clone(),
                                success: false,
                                content: error_msg.clone(),
                            }).await?;
                            (error_msg, true)
                        }
                    };

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: tool_id.clone(),
                        content,
                        is_error,
                    });
                }

                // Add tool results as a user message (Anthropic API requires this)
                loop_messages.push(Message {
                    role: SdkMessageRole::Tool,
                    content: MessageContent::Structured(tool_results),
                    token_count: 0,
                });

                // Store tool uses for persistence
                let had_tools = !current_turn_tools.is_empty();
                tool_uses.extend(current_turn_tools);

                // Continue the agentic loop if stop_reason is "tool_use"
                if stop_reason == "tool_use" && had_tools {
                    continue 'agentic_loop;
                }
            }

            // End turn - we're done
            break 'agentic_loop;
        }

        // Store final assistant response
        if config.persist && !accumulated_text.is_empty() {
            let stored_tool_calls: Option<Vec<StoredToolCall>> = if tool_uses.is_empty() {
                None
            } else {
                Some(tool_uses.iter().map(|(id, name, args)| StoredToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                }).collect())
            };

            history
                .add_message(
                    &session_id,
                    MessageRole::Assistant,
                    &accumulated_text,
                    stored_tool_calls.as_deref(),
                    None,
                    Some(&model),
                    None,
                )
                .await
                .context("Failed to store assistant message")?;
        }

        // Auto-generate title if this is the first user message
        if config.auto_title {
            let message_count = history.count_messages(&session_id).await.unwrap_or(0);
            // System message + first user message + first response = 3
            if message_count <= 3 {
                let title = Self::generate_title(&content);
                let _ = history.update_session_title(&session_id, &title).await;
            }
        }

        tx.send(ChatChunk::Done { token_count: None }).await?;

        Ok(())
    }

    /// Generate a title from the first message.
    fn generate_title(content: &str) -> String {
        // Simple title generation - take first N characters
        let max_len = 50;
        let trimmed = content.trim();

        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            // Find a good break point
            let truncated = &trimmed[..max_len];
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        }
    }

    /// Convert database message rows to SDK message format.
    fn convert_to_sdk_messages(messages: &[ChatMessageRow], system_prompt: &str) -> Vec<Message> {
        let mut sdk_messages = Vec::with_capacity(messages.len() + 1);

        // Add system prompt first
        if !system_prompt.is_empty() {
            sdk_messages.push(Message {
                role: SdkMessageRole::System,
                content: MessageContent::Text(system_prompt.to_string()),
                token_count: (system_prompt.len() / 4) as u32,
            });
        }

        // Convert each message
        for msg in messages {
            let role = match msg.role {
                MessageRole::User => SdkMessageRole::User,
                MessageRole::Assistant => SdkMessageRole::Assistant,
                MessageRole::System => SdkMessageRole::System,
                MessageRole::Tool => SdkMessageRole::Tool,
            };

            // Build content
            let content = if let Some(ref tool_calls) = msg.tool_calls {
                // Assistant message with tool calls
                let mut blocks = Vec::new();
                if !msg.content.is_empty() {
                    blocks.push(ContentBlock::Text {
                        text: msg.content.clone(),
                    });
                }
                for tc in tool_calls {
                    blocks.push(ContentBlock::ToolUse {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        input: tc.arguments.clone(),
                    });
                }
                MessageContent::Structured(blocks)
            } else if let Some(ref tool_call_id) = msg.tool_call_id {
                // Tool result message
                MessageContent::Structured(vec![ContentBlock::ToolResult {
                    tool_use_id: tool_call_id.clone(),
                    content: msg.content.clone(),
                    is_error: false,
                }])
            } else {
                // Regular text message
                MessageContent::Text(msg.content.clone())
            };

            sdk_messages.push(Message {
                role,
                content,
                token_count: (msg.content.len() / 4) as u32,
            });
        }

        sdk_messages
    }

    /// Build SDK tool registry from executor definitions.
    fn build_sdk_tools(executor: &ToolExecutor, preset: &AgentPreset) -> SdkToolRegistry {
        let mut registry = SdkToolRegistry::new();

        for tool_config in &preset.tools {
            if !tool_config.enabled {
                continue;
            }

            if let Some(def) = executor.get_tool_definition(&tool_config.name) {
                // Convert to SDK tool definition format
                let sdk_def = SdkToolDefinition {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    input_schema: def.to_json_schema()["function"]["parameters"].clone(),
                };

                // Create a simple wrapper tool
                registry.register(SimpleToolWrapper { definition: sdk_def });
            }
        }

        registry
    }

    /// Build assistant content blocks from text and tool uses.
    fn build_assistant_blocks(text: &str, tool_uses: &[(String, String, Value)]) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();

        if !text.is_empty() {
            blocks.push(ContentBlock::Text {
                text: text.to_string(),
            });
        }

        for (id, name, input) in tool_uses {
            blocks.push(ContentBlock::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
        }

        blocks
    }

    /// Execute a tool and return the result.
    pub async fn execute_tool(&self, name: &str, arguments: Value) -> Result<ChatChunk> {
        let result = self
            .executor
            .execute_with_session(name, arguments, self.tool_context.clone(), Some(self.id.clone()))
            .await;

        match result {
            Ok(tool_result) => Ok(ChatChunk::ToolResult {
                tool_call_id: uuid::Uuid::new_v4().to_string(),
                success: tool_result.success,
                content: if tool_result.success {
                    serde_json::to_string(&tool_result.data).unwrap_or_default()
                } else {
                    tool_result.error.unwrap_or_default()
                },
            }),
            Err(e) => Ok(ChatChunk::ToolResult {
                tool_call_id: uuid::Uuid::new_v4().to_string(),
                success: false,
                content: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::presets::PresetRegistry;
    use crate::agent::tools::ToolRegistry;

    async fn setup_test_components() -> (Arc<ChatHistory>, Arc<VlmInferenceService>, Arc<ToolExecutor>) {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();

        // Run migrations
        sqlx::query("CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();

        let history = Arc::new(ChatHistory::new(pool.clone()));
        history.migrate().await.unwrap();

        // Create a mock VLM service
        let vlm_service = Arc::new(VlmInferenceService::new_mock());

        // Create tool executor
        let registry = ToolRegistry::with_builtins();
        let executor = Arc::new(ToolExecutor::new(registry).unwrap().without_validation());

        (history, vlm_service, executor)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (history, vlm_service, executor) = setup_test_components().await;
        let registry = PresetRegistry::load_builtin().unwrap();
        let preset = registry.get("video-analyst").unwrap().clone();
        let tool_context = ToolContext::without_db();

        let session = ChatSession::new(
            preset,
            history,
            vlm_service,
            executor,
            tool_context,
            SessionConfig::default(),
        )
        .await
        .unwrap();

        assert!(!session.id().is_empty());
        assert_eq!(session.preset().id.as_str(), "video-analyst");
    }

    #[tokio::test]
    async fn test_session_messages() {
        let (history, vlm_service, executor) = setup_test_components().await;
        let registry = PresetRegistry::load_builtin().unwrap();
        let preset = registry.get("video-analyst").unwrap().clone();
        let tool_context = ToolContext::without_db();

        let session = ChatSession::new(
            preset,
            history.clone(),
            vlm_service,
            executor,
            tool_context,
            SessionConfig::default(),
        )
        .await
        .unwrap();

        // Send a message
        let mut rx = session.send_message("Hello, agent!".to_string());

        // Collect chunks
        let mut chunks = Vec::new();
        while let Some(chunk) = rx.recv().await {
            chunks.push(chunk);
        }

        // Should have at least text and done
        assert!(chunks.len() >= 2);
        assert!(matches!(chunks.last(), Some(ChatChunk::Done { .. })));

        // Check messages were stored
        let messages = history.get_messages(session.id()).await.unwrap();
        // System + User + Assistant = at least 3
        assert!(messages.len() >= 2);
    }

    #[tokio::test]
    async fn test_load_session() {
        let (history, vlm_service, executor) = setup_test_components().await;
        let preset_registry = PresetRegistry::load_builtin().unwrap();
        let preset = preset_registry.get("quick-search").unwrap().clone();
        let tool_context = ToolContext::without_db();

        // Create a session
        let session = ChatSession::new(
            preset,
            history.clone(),
            vlm_service.clone(),
            executor.clone(),
            tool_context.clone(),
            SessionConfig::default(),
        )
        .await
        .unwrap();

        let session_id = session.id().to_string();

        // Load it
        let loaded = ChatSession::load(
            &session_id,
            &preset_registry,
            history,
            vlm_service,
            executor,
            tool_context,
            SessionConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(loaded.id(), session_id);
        assert_eq!(loaded.preset().id.as_str(), "quick-search");
    }

    #[tokio::test]
    async fn test_session_title() {
        let (history, vlm_service, executor) = setup_test_components().await;
        let registry = PresetRegistry::load_builtin().unwrap();
        let preset = registry.get("video-analyst").unwrap().clone();
        let tool_context = ToolContext::without_db();

        let session = ChatSession::new(
            preset,
            history,
            vlm_service,
            executor,
            tool_context,
            SessionConfig::default(),
        )
        .await
        .unwrap();

        session.set_title("My Test Session").await.unwrap();

        let info = session.get_info().await.unwrap().unwrap();
        assert_eq!(info.title, Some("My Test Session".to_string()));
    }

    #[test]
    fn test_generate_title() {
        assert_eq!(
            ChatSession::generate_title("Short title"),
            "Short title"
        );

        let long_message = "This is a very long message that should be truncated to a reasonable title length for display";
        let title = ChatSession::generate_title(long_message);
        assert!(title.len() <= 53); // 50 + "..."
        assert!(title.ends_with("..."));
    }
}
