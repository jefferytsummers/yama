//! Health reporting for containerized services.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, instrument};

use yama_protocol::common::{ComponentHealth, HealthCheckResponse, ServiceStatus};

use crate::client::EventBusClient;

/// Health status of a component.
#[derive(Debug, Clone)]
pub struct ComponentStatus {
    /// Whether the component is healthy.
    pub healthy: bool,
    /// Status message.
    pub message: String,
}

impl ComponentStatus {
    /// Create a healthy status.
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: "OK".to_string(),
        }
    }

    /// Create an unhealthy status.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: message.into(),
        }
    }
}

/// Health reporter for a service.
pub struct HealthReporter {
    /// Service ID.
    service_id: String,
    /// Event bus client.
    client: Arc<EventBusClient>,
    /// Component health statuses.
    components: Arc<RwLock<HashMap<String, ComponentStatus>>>,
    /// Overall service status.
    status: Arc<RwLock<ServiceStatus>>,
    /// Report interval.
    interval: Duration,
}

impl HealthReporter {
    /// Create a new health reporter.
    pub fn new(client: Arc<EventBusClient>, service_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            client,
            components: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(ServiceStatus::Starting)),
            interval: Duration::from_secs(5),
        }
    }

    /// Set the report interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set the overall service status.
    pub async fn set_status(&self, status: ServiceStatus) {
        *self.status.write().await = status;
    }

    /// Update a component's health status.
    pub async fn set_component(&self, name: impl Into<String>, status: ComponentStatus) {
        self.components.write().await.insert(name.into(), status);
    }

    /// Remove a component from health tracking.
    pub async fn remove_component(&self, name: &str) {
        self.components.write().await.remove(name);
    }

    /// Mark the service as healthy.
    pub async fn healthy(&self) {
        self.set_status(ServiceStatus::Healthy).await;
    }

    /// Mark the service as unhealthy.
    pub async fn unhealthy(&self) {
        self.set_status(ServiceStatus::Unhealthy).await;
    }

    /// Mark the service as degraded.
    pub async fn degraded(&self) {
        self.set_status(ServiceStatus::Degraded).await;
    }

    /// Start the health reporter background task.
    #[instrument(skip(self), fields(service_id = %self.service_id))]
    pub async fn start(&self) -> Result<()> {
        info!("Starting health reporter");

        let service_id = self.service_id.clone();
        let client = self.client.clone();
        let components = self.components.clone();
        let status = self.status.clone();
        let interval_duration = self.interval;

        tokio::spawn(async move {
            let mut interval = interval(interval_duration);

            loop {
                interval.tick().await;

                let current_status = *status.read().await;
                let current_components = components.read().await;

                // Build component health map
                let component_health: HashMap<String, ComponentHealth> = current_components
                    .iter()
                    .map(|(name, status)| {
                        (
                            name.clone(),
                            ComponentHealth {
                                name: name.clone(),
                                healthy: status.healthy,
                                message: status.message.clone(),
                            },
                        )
                    })
                    .collect();

                // Determine overall status based on components
                let effective_status = if current_status == ServiceStatus::Starting {
                    current_status
                } else if current_components.is_empty() {
                    current_status
                } else {
                    let all_healthy = current_components.values().all(|c| c.healthy);
                    let any_healthy = current_components.values().any(|c| c.healthy);

                    if all_healthy {
                        ServiceStatus::Healthy
                    } else if any_healthy {
                        ServiceStatus::Degraded
                    } else {
                        ServiceStatus::Unhealthy
                    }
                };

                let response = HealthCheckResponse {
                    service_id: service_id.clone(),
                    status: effective_status.into(),
                    message: format!("{:?}", effective_status),
                    components: component_health,
                    checked_at: Some(prost_types::Timestamp {
                        seconds: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        nanos: 0,
                    }),
                };

                debug!("Reporting health: {:?}", effective_status);

                if let Err(e) = client.publish("system.health", response).await {
                    error!("Failed to publish health: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Build a health response for HTTP endpoints.
    pub async fn build_response(&self) -> HealthCheckResponse {
        let current_status = *self.status.read().await;
        let current_components = self.components.read().await;

        let component_health: HashMap<String, ComponentHealth> = current_components
            .iter()
            .map(|(name, status)| {
                (
                    name.clone(),
                    ComponentHealth {
                        name: name.clone(),
                        healthy: status.healthy,
                        message: status.message.clone(),
                    },
                )
            })
            .collect();

        HealthCheckResponse {
            service_id: self.service_id.clone(),
            status: current_status.into(),
            message: format!("{:?}", current_status),
            components: component_health,
            checked_at: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                nanos: 0,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_status() {
        let healthy = ComponentStatus::healthy();
        assert!(healthy.healthy);

        let unhealthy = ComponentStatus::unhealthy("error");
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.message, "error");
    }
}
