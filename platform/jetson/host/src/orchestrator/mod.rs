//! Service orchestrator for managing containers and processes.
//!
//! The orchestrator:
//! - Spawns and monitors container services
//! - Handles health checks and restarts
//! - Manages port allocation
//! - Coordinates service lifecycle

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::models::{DeviceMapping, HostConfig, Mount, MountTypeEnum, PortBinding};
use bollard::Docker;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::event_bus::EventBus;

/// Orchestrator configuration.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OrchestratorConfig {
    /// Container runtime: "docker" or "podman".
    #[serde(default = "default_runtime")]
    pub runtime: String,
    /// Health check interval in seconds.
    #[serde(default = "default_health_interval")]
    pub health_interval: u64,
    /// Restart policy.
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    /// Maximum restart attempts.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Service definitions.
    #[serde(default)]
    pub services: Vec<ServiceDefinition>,
}

fn default_runtime() -> String {
    "docker".to_string()
}

fn default_health_interval() -> u64 {
    5
}

fn default_max_restarts() -> u32 {
    3
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            runtime: default_runtime(),
            health_interval: default_health_interval(),
            restart_policy: RestartPolicy::default(),
            max_restarts: default_max_restarts(),
            services: Vec::new(),
        }
    }
}

/// Restart policy for services.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart.
    Never,
    /// Restart on failure.
    #[default]
    OnFailure,
    /// Always restart.
    Always,
}

/// Service definition.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServiceDefinition {
    /// Service name.
    pub name: String,
    /// Container image.
    pub image: String,
    /// Whether GPU access is required.
    #[serde(default)]
    pub gpu_required: bool,
    /// Health check endpoint.
    pub health_endpoint: Option<String>,
    /// Unix socket path.
    pub socket: Option<String>,
    /// Environment variables.
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Port mappings.
    #[serde(default)]
    pub ports: Vec<PortMapping>,
    /// Volume mounts.
    #[serde(default)]
    pub volumes: Vec<VolumeMount>,
    /// Auto-start on orchestrator start.
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

fn default_true() -> bool {
    true
}

/// Port mapping.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: u16,
    #[serde(default = "default_tcp")]
    pub protocol: String,
}

fn default_tcp() -> String {
    "tcp".to_string()
}

/// Volume mount.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

/// Service state.
#[derive(Debug, Clone)]
pub struct ServiceState {
    pub name: String,
    pub container_id: Option<String>,
    pub status: ServiceStatus,
    pub restart_count: u32,
    pub last_health_check: Option<std::time::Instant>,
}

/// Service status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Unhealthy,
    Stopping,
    Failed,
}

/// The service orchestrator.
pub struct Orchestrator {
    config: OrchestratorConfig,
    docker: Docker,
    services: Arc<RwLock<HashMap<String, ServiceState>>>,
    event_bus: Arc<EventBus>,
    running: bool,
}

impl Orchestrator {
    /// Create a new orchestrator.
    pub async fn new(config: OrchestratorConfig, event_bus: Arc<EventBus>) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker")?;

        // Verify Docker connection
        let info = docker.info().await.context("Failed to get Docker info")?;
        info!("Connected to Docker: {:?}", info.name);

        let services = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            docker,
            services,
            event_bus,
            running: false,
        })
    }

    /// Start the orchestrator and auto-start services.
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting orchestrator");
        self.running = true;

        // Initialize service states
        for def in &self.config.services {
            let state = ServiceState {
                name: def.name.clone(),
                container_id: None,
                status: ServiceStatus::Stopped,
                restart_count: 0,
                last_health_check: None,
            };
            self.services.write().await.insert(def.name.clone(), state);
        }

        // Auto-start services
        for def in self.config.services.clone() {
            if def.auto_start {
                if let Err(e) = self.start_service(&def.name).await {
                    error!("Failed to auto-start service {}: {}", def.name, e);
                }
            }
        }

        // Start health check loop
        self.run_health_checks().await?;

        Ok(())
    }

    /// Stop the orchestrator and all services.
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping orchestrator");
        self.running = false;

        let services = self.services.read().await;
        let names: Vec<String> = services.keys().cloned().collect();
        drop(services);

        for name in names {
            if let Err(e) = self.stop_service(&name).await {
                error!("Failed to stop service {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// Start a service.
    pub async fn start_service(&self, name: &str) -> Result<()> {
        let def = self
            .config
            .services
            .iter()
            .find(|s| s.name == name)
            .context("Service not found")?
            .clone();

        info!("Starting service: {}", name);

        // Update status
        {
            let mut services = self.services.write().await;
            if let Some(state) = services.get_mut(name) {
                state.status = ServiceStatus::Starting;
            }
        }

        // Build container config
        let mut env: Vec<String> = def
            .environment
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();

        // Add event bus socket
        env.push(format!("YAMA_EVENT_BUS=/tmp/yama-event.sock"));

        let mut port_bindings = HashMap::new();
        for pm in &def.ports {
            let key = format!("{}/{}", pm.container_port, pm.protocol);
            port_bindings.insert(
                key,
                Some(vec![PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(pm.host_port.to_string()),
                }]),
            );
        }

        let mut mounts = Vec::new();
        for vm in &def.volumes {
            mounts.push(Mount {
                target: Some(vm.target.clone()),
                source: Some(vm.source.clone()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(vm.read_only),
                ..Default::default()
            });
        }

        // Add event bus socket mount
        mounts.push(Mount {
            target: Some("/tmp/yama-event.sock".to_string()),
            source: Some("/tmp/yama-event.sock".to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        });

        let mut host_config = HostConfig {
            port_bindings: Some(port_bindings),
            mounts: Some(mounts),
            ..Default::default()
        };

        // Add GPU access for Apple Silicon
        if def.gpu_required {
            // On macOS, GPU access is handled differently than Linux
            // Metal devices are accessible through /dev/dri/*
            host_config.devices = Some(vec![DeviceMapping {
                path_on_host: Some("/dev/dri".to_string()),
                path_in_container: Some("/dev/dri".to_string()),
                cgroup_permissions: Some("rwm".to_string()),
            }]);
        }

        let config = Config {
            image: Some(def.image.clone()),
            env: Some(env),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("yama-{}", name),
            platform: None,
        };

        // Create and start container
        let container = self
            .docker
            .create_container(Some(options), config)
            .await
            .context("Failed to create container")?;

        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        info!("Service {} started with container {}", name, container.id);

        // Update state
        {
            let mut services = self.services.write().await;
            if let Some(state) = services.get_mut(name) {
                state.container_id = Some(container.id);
                state.status = ServiceStatus::Running;
            }
        }

        Ok(())
    }

    /// Stop a service.
    pub async fn stop_service(&self, name: &str) -> Result<()> {
        let state = {
            let services = self.services.read().await;
            services.get(name).cloned()
        };

        if let Some(state) = state {
            if let Some(container_id) = &state.container_id {
                info!("Stopping service: {}", name);

                // Update status
                {
                    let mut services = self.services.write().await;
                    if let Some(state) = services.get_mut(name) {
                        state.status = ServiceStatus::Stopping;
                    }
                }

                // Stop container
                let options = StopContainerOptions { t: 10 };
                if let Err(e) = self.docker.stop_container(container_id, Some(options)).await {
                    warn!("Failed to stop container: {}", e);
                }

                // Remove container
                let options = RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                };
                if let Err(e) = self.docker.remove_container(container_id, Some(options)).await {
                    warn!("Failed to remove container: {}", e);
                }

                // Update state
                {
                    let mut services = self.services.write().await;
                    if let Some(state) = services.get_mut(name) {
                        state.container_id = None;
                        state.status = ServiceStatus::Stopped;
                    }
                }
            }
        }

        Ok(())
    }

    /// Restart a service.
    pub async fn restart_service(&self, name: &str) -> Result<()> {
        self.stop_service(name).await?;
        self.start_service(name).await?;
        Ok(())
    }

    /// Run health check loop.
    async fn run_health_checks(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(self.config.health_interval));

        while self.running {
            interval.tick().await;

            let services = self.services.read().await;
            let names: Vec<String> = services.keys().cloned().collect();
            drop(services);

            for name in names {
                if let Err(e) = self.check_service_health(&name).await {
                    debug!("Health check failed for {}: {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Check health of a service.
    async fn check_service_health(&self, name: &str) -> Result<()> {
        let state = {
            let services = self.services.read().await;
            services.get(name).cloned()
        };

        if let Some(state) = state {
            if let Some(container_id) = &state.container_id {
                // Check container status
                let inspect = self.docker.inspect_container(container_id, None).await?;

                if let Some(state) = inspect.state {
                    let running = state.running.unwrap_or(false);

                    if !running {
                        warn!("Service {} is not running", name);

                        // Handle restart policy
                        self.handle_service_failure(name).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle service failure.
    async fn handle_service_failure(&self, name: &str) -> Result<()> {
        let should_restart = {
            let mut services = self.services.write().await;
            if let Some(state) = services.get_mut(name) {
                state.status = ServiceStatus::Failed;

                match self.config.restart_policy {
                    RestartPolicy::Never => false,
                    RestartPolicy::OnFailure | RestartPolicy::Always => {
                        if state.restart_count < self.config.max_restarts {
                            state.restart_count += 1;
                            true
                        } else {
                            error!(
                                "Service {} exceeded max restarts ({})",
                                name, self.config.max_restarts
                            );
                            false
                        }
                    }
                }
            } else {
                false
            }
        };

        if should_restart {
            info!("Restarting failed service: {}", name);
            self.restart_service(name).await?;
        }

        Ok(())
    }

    /// Get service status.
    pub async fn get_service_status(&self, name: &str) -> Option<ServiceStatus> {
        let services = self.services.read().await;
        services.get(name).map(|s| s.status)
    }

    /// List all services.
    pub async fn list_services(&self) -> Vec<(String, ServiceStatus)> {
        let services = self.services.read().await;
        services
            .iter()
            .map(|(name, state)| (name.clone(), state.status))
            .collect()
    }
}
