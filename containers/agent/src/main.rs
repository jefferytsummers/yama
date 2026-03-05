//! Yama Agent Runtime Container
//!
//! This container runs LLM agents, managing their lifecycle, tool execution,
//! and communication with the event bus.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use yama_agent_sdk::{AgentConfig, AgentRuntime, ToolRegistry};
use yama_agent_sdk::llm::{AnthropicBackend, LlmConfig, TtraBackend};
use yama_container_sdk::{EventBusClient, HealthReporter};

/// Service configuration.
#[derive(Debug, Clone, serde::Deserialize)]
struct Config {
    /// Event bus socket path.
    event_bus_socket: String,
    /// Skills directory.
    skills_path: PathBuf,
    /// Agent configurations.
    agents: Vec<AgentInstanceConfig>,
    /// Default LLM backend.
    default_llm: LlmBackendConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AgentInstanceConfig {
    agent_id: String,
    name: String,
    system_prompt: String,
    enabled: bool,
    #[serde(default)]
    llm: Option<LlmBackendConfig>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LlmBackendConfig {
    backend: String,
    endpoint: Option<String>,
    api_key: Option<String>,
    model: String,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: std::env::var("YAMA_EVENT_BUS")
                .unwrap_or_else(|_| "/tmp/yama-event.sock".to_string()),
            skills_path: std::env::var("SKILLS_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/app/skills")),
            agents: Vec::new(),
            default_llm: LlmBackendConfig {
                backend: "ttra".to_string(),
                endpoint: Some("http://inference:8080".to_string()),
                api_key: None,
                model: "llama-3.2-3b".to_string(),
                temperature: 0.7,
                max_tokens: 4096,
            },
        }
    }
}

/// Active agent instance.
struct AgentInstance {
    config: AgentInstanceConfig,
    // Runtime would be stored here
    active_connections: usize,
}

/// Agent service managing multiple agents.
struct AgentService {
    config: Config,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
    agents: Arc<RwLock<HashMap<String, AgentInstance>>>,
}

impl AgentService {
    async fn new(config: Config) -> Result<Self> {
        // Connect to event bus
        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "agent")
                .await
                .context("Failed to connect to event bus")?,
        );
        info!("Connected to event bus");

        // Set up health reporter
        let health = HealthReporter::new(event_bus.clone(), "agent");

        Ok(Self {
            config,
            event_bus,
            health,
            agents: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn run(mut self) -> Result<()> {
        info!("Starting agent service");

        // Start health reporting
        self.health.healthy().await;
        self.health.start().await?;

        // Subscribe to agent messages
        self.event_bus
            .subscribe(&["agent.*"])
            .await
            .context("Failed to subscribe")?;

        // Initialize configured agents
        for agent_config in &self.config.agents {
            if agent_config.enabled {
                if let Err(e) = self.start_agent(&agent_config.agent_id).await {
                    error!("Failed to start agent {}: {}", agent_config.agent_id, e);
                    self.health
                        .set_component(
                            &agent_config.agent_id,
                            yama_container_sdk::health::ComponentStatus::unhealthy(e.to_string()),
                        )
                        .await;
                } else {
                    self.health
                        .set_component(
                            &agent_config.agent_id,
                            yama_container_sdk::health::ComponentStatus::healthy(),
                        )
                        .await;
                }
            }
        }

        info!("Agent service ready, waiting for messages");

        // Main event loop
        loop {
            // Process events from event bus
            // In a real implementation, we'd receive and dispatch messages to agents
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    async fn start_agent(&self, agent_id: &str) -> Result<()> {
        let agent_config = self
            .config
            .agents
            .iter()
            .find(|a| a.agent_id == agent_id)
            .context("Agent not found")?
            .clone();

        info!("Starting agent: {}", agent_id);

        // Create agent instance
        let instance = AgentInstance {
            config: agent_config.clone(),
            active_connections: 0,
        };

        self.agents
            .write()
            .await
            .insert(agent_id.to_string(), instance);

        info!("Agent {} started", agent_id);
        Ok(())
    }

    async fn stop_agent(&self, agent_id: &str) -> Result<()> {
        info!("Stopping agent: {}", agent_id);
        self.agents.write().await.remove(agent_id);
        Ok(())
    }

    async fn list_agents(&self) -> Vec<(String, bool)> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(id, instance)| (id.clone(), instance.active_connections > 0))
            .collect()
    }
}

fn load_config() -> Result<Config> {
    // Try to load from environment-specified path
    if let Ok(config_path) = std::env::var("CONFIG_PATH") {
        let config_file = PathBuf::from(config_path).join("agent.toml");
        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            return toml::from_str(&content).context("Failed to parse config");
        }
    }

    // Fall back to defaults
    Ok(Config::default())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama Agent Runtime");

    // Load configuration
    let config = load_config()?;
    info!("Configuration loaded");

    // Create and run service
    let service = AgentService::new(config).await?;
    service.run().await
}
