//! Agent runtime - the main entry point for running an LLM agent.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};

use crate::context::{Context, ContextConfig, ContextManager};
use crate::llm::{LlmBackend, LlmResponse};
use crate::skills::SkillLoader;
use crate::tools::{Tool, ToolRegistry, ToolResult};

/// Agent configuration loaded from TOML or protobuf.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgentConfig {
    /// Unique identifier for this agent.
    pub agent_id: String,
    /// Human-readable name.
    pub name: String,
    /// System prompt defining agent behavior.
    pub system_prompt: String,
    /// LLM backend configuration.
    pub llm: LlmConfig,
    /// Context window configuration.
    pub context: ContextConfig,
    /// Enabled tools.
    pub tools: Vec<ToolConfig>,
    /// Enabled skills.
    pub skills: Vec<SkillConfig>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// LLM backend configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LlmConfig {
    /// Backend type: "ttra", "anthropic", "openai", "llama_cpp".
    pub backend: String,
    /// Model identifier.
    pub model: String,
    /// Sampling temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Backend-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

/// Tool configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ToolConfig {
    /// Tool name.
    pub name: String,
    /// Tool description for the LLM.
    pub description: String,
    /// JSON Schema for parameters.
    pub parameters_schema: String,
    /// Handler service/container.
    pub handler: String,
    /// Whether the tool is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Skill configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SkillConfig {
    /// Skill name.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Path to SKILL.md file.
    pub path: PathBuf,
    /// Whether the skill is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl AgentConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(content)?;
        Ok(config)
    }
}

/// Message types for the agent runtime.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// User message to process.
    UserMessage {
        connection_id: String,
        content: String,
    },
    /// Tool result from a tool execution.
    ToolResult {
        connection_id: String,
        tool_use_id: String,
        result: ToolResult,
    },
    /// Request to stop processing.
    Stop { connection_id: String },
    /// Shutdown the agent.
    Shutdown,
}

/// Response types from the agent runtime.
#[derive(Debug, Clone)]
pub enum AgentResponse {
    /// Text chunk from the LLM.
    TextDelta {
        connection_id: String,
        text: String,
    },
    /// Tool use request.
    ToolUse {
        connection_id: String,
        tool_use_id: String,
        tool_name: String,
        input: serde_json::Value,
    },
    /// Message complete.
    Complete {
        connection_id: String,
        stop_reason: String,
    },
    /// Error occurred.
    Error {
        connection_id: String,
        error: String,
    },
}

/// Connection state for a single client.
struct ConnectionState {
    /// Connection identifier.
    id: String,
    /// Context manager for this connection.
    context: ContextManager,
    /// Currently processing a request.
    processing: bool,
}

/// The agent runtime manages the agent loop and connections.
pub struct AgentRuntime<B: LlmBackend> {
    /// Agent configuration.
    config: AgentConfig,
    /// LLM backend.
    backend: Arc<B>,
    /// Tool registry.
    tools: Arc<ToolRegistry>,
    /// Skill loader.
    skills: Arc<SkillLoader>,
    /// Active connections.
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    /// Message channel sender.
    tx: mpsc::Sender<AgentMessage>,
    /// Message channel receiver.
    rx: mpsc::Receiver<AgentMessage>,
    /// Response broadcast channel.
    response_tx: broadcast::Sender<AgentResponse>,
}

impl<B: LlmBackend + Send + Sync + 'static> AgentRuntime<B> {
    /// Create a new agent runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new(config: AgentConfig, backend: B) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(100);
        let (response_tx, _) = broadcast::channel(100);

        let mut tools = ToolRegistry::new();
        for tool_config in &config.tools {
            if tool_config.enabled {
                tools.register_from_config(tool_config)?;
            }
        }

        let mut skills = SkillLoader::new();
        for skill_config in &config.skills {
            if skill_config.enabled {
                skills.load(&skill_config.path)?;
            }
        }

        Ok(Self {
            config,
            backend: Arc::new(backend),
            tools: Arc::new(tools),
            skills: Arc::new(skills),
            connections: Arc::new(RwLock::new(HashMap::new())),
            tx,
            rx,
            response_tx,
        })
    }

    /// Get a sender for sending messages to the runtime.
    pub fn sender(&self) -> mpsc::Sender<AgentMessage> {
        self.tx.clone()
    }

    /// Subscribe to responses from the runtime.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentResponse> {
        self.response_tx.subscribe()
    }

    /// Run the agent runtime loop.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime fails.
    #[instrument(skip(self), fields(agent_id = %self.config.agent_id))]
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Starting agent runtime");

        while let Some(message) = self.rx.recv().await {
            match message {
                AgentMessage::UserMessage {
                    connection_id,
                    content,
                } => {
                    self.handle_user_message(&connection_id, &content).await?;
                }
                AgentMessage::ToolResult {
                    connection_id,
                    tool_use_id,
                    result,
                } => {
                    self.handle_tool_result(&connection_id, &tool_use_id, result)
                        .await?;
                }
                AgentMessage::Stop { connection_id } => {
                    self.handle_stop(&connection_id).await?;
                }
                AgentMessage::Shutdown => {
                    info!("Shutting down agent runtime");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a user message.
    #[instrument(skip(self, content), fields(connection_id = %connection_id))]
    async fn handle_user_message(
        &self,
        connection_id: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        debug!("Handling user message");

        // Get or create connection state
        let mut connections = self.connections.write().await;
        let state = connections.entry(connection_id.to_string()).or_insert_with(|| {
            ConnectionState {
                id: connection_id.to_string(),
                context: ContextManager::new(self.config.context.clone()),
                processing: false,
            }
        });

        if state.processing {
            warn!("Connection already processing a request");
            return Ok(());
        }

        state.processing = true;

        // Add user message to context
        state.context.add_user_message(content);

        // Build messages for LLM
        let messages = state.context.build_messages(&self.config.system_prompt);
        drop(connections);

        // Call LLM
        let response_tx = self.response_tx.clone();
        let backend = self.backend.clone();
        let tools = self.tools.clone();
        let connection_id = connection_id.to_string();
        let connections = self.connections.clone();

        tokio::spawn(async move {
            match backend.chat(&messages, tools.as_ref()).await {
                Ok(mut stream) => {
                    let mut full_text = String::new();
                    let mut tool_uses = Vec::new();

                    while let Some(response) = stream.recv().await {
                        match response {
                            LlmResponse::TextDelta { text } => {
                                full_text.push_str(&text);
                                let _ = response_tx.send(AgentResponse::TextDelta {
                                    connection_id: connection_id.clone(),
                                    text,
                                });
                            }
                            LlmResponse::ToolUse {
                                id,
                                name,
                                input,
                            } => {
                                tool_uses.push((id.clone(), name.clone(), input.clone()));
                                let _ = response_tx.send(AgentResponse::ToolUse {
                                    connection_id: connection_id.clone(),
                                    tool_use_id: id,
                                    tool_name: name,
                                    input,
                                });
                            }
                            LlmResponse::Complete { stop_reason } => {
                                // Update context with assistant response
                                let mut conns = connections.write().await;
                                if let Some(state) = conns.get_mut(&connection_id) {
                                    state.context.add_assistant_message(&full_text, &tool_uses);
                                    state.processing = tool_uses.is_empty();
                                }
                                drop(conns);

                                let _ = response_tx.send(AgentResponse::Complete {
                                    connection_id: connection_id.clone(),
                                    stop_reason,
                                });
                            }
                            LlmResponse::Error { message } => {
                                let mut conns = connections.write().await;
                                if let Some(state) = conns.get_mut(&connection_id) {
                                    state.processing = false;
                                }
                                drop(conns);

                                let _ = response_tx.send(AgentResponse::Error {
                                    connection_id: connection_id.clone(),
                                    error: message,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("LLM error: {}", e);
                    let mut conns = connections.write().await;
                    if let Some(state) = conns.get_mut(&connection_id) {
                        state.processing = false;
                    }
                    drop(conns);

                    let _ = response_tx.send(AgentResponse::Error {
                        connection_id,
                        error: e.to_string(),
                    });
                }
            }
        });

        Ok(())
    }

    /// Handle a tool result.
    #[instrument(skip(self, result), fields(connection_id = %connection_id, tool_use_id = %tool_use_id))]
    async fn handle_tool_result(
        &self,
        connection_id: &str,
        tool_use_id: &str,
        result: ToolResult,
    ) -> anyhow::Result<()> {
        debug!("Handling tool result");

        let mut connections = self.connections.write().await;
        if let Some(state) = connections.get_mut(connection_id) {
            state.context.add_tool_result(tool_use_id, &result);
        }
        drop(connections);

        // Continue the conversation with the tool result
        // This will trigger another LLM call
        self.handle_user_message(connection_id, "").await
    }

    /// Handle a stop request.
    #[instrument(skip(self), fields(connection_id = %connection_id))]
    async fn handle_stop(&self, connection_id: &str) -> anyhow::Result<()> {
        debug!("Handling stop request");

        let mut connections = self.connections.write().await;
        if let Some(state) = connections.get_mut(connection_id) {
            state.processing = false;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_str() {
        let toml = r#"
            agent_id = "test-agent"
            name = "Test Agent"
            system_prompt = "You are a helpful assistant."

            [llm]
            backend = "anthropic"
            model = "claude-sonnet-4-20250514"
            temperature = 0.7
            max_tokens = 4096

            [context]
            max_tokens = 100000
            overflow_strategy = "sliding_window"

            [[tools]]
            name = "get_time"
            description = "Get the current time"
            parameters_schema = "{}"
            handler = "system"
            enabled = true

            [[skills]]
            name = "video-analysis"
            description = "Analyze video streams"
            path = "skills/video-analysis"
            enabled = true
        "#;

        let config = AgentConfig::from_str(toml).expect("should parse");
        assert_eq!(config.agent_id, "test-agent");
        assert_eq!(config.llm.backend, "anthropic");
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.skills.len(), 1);
    }
}
