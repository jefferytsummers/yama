//! Request handlers for the HTTP API.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::orchestrator::ServiceStatus;
use crate::ServiceState;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Health check endpoint.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Service info for API responses.
#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: String,
    pub container_id: Option<String>,
}

/// Convert ServiceStatus to string.
fn status_to_string(status: ServiceStatus) -> String {
    match status {
        ServiceStatus::Stopped => "stopped".to_string(),
        ServiceStatus::Starting => "starting".to_string(),
        ServiceStatus::Running => "running".to_string(),
        ServiceStatus::Unhealthy => "unhealthy".to_string(),
        ServiceStatus::Stopping => "stopping".to_string(),
        ServiceStatus::Failed => "failed".to_string(),
    }
}

/// List all services.
pub async fn list_services(State(state): State<Arc<ServiceState>>) -> Json<Vec<ServiceInfo>> {
    let orchestrator = state.orchestrator.read().await;
    let services = orchestrator.list_services().await;

    let service_infos: Vec<ServiceInfo> = services
        .into_iter()
        .map(|(name, status)| ServiceInfo {
            name,
            status: status_to_string(status),
            container_id: None, // Could be expanded to include this
        })
        .collect();

    Json(service_infos)
}

/// Generic API response.
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// Start a service.
pub async fn start_service(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let orchestrator = state.orchestrator.read().await;

    match orchestrator.start_service(&id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                message: format!("Service '{}' started", id),
            }),
        ),
        Err(e) => {
            error!("Failed to start service '{}': {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to start service: {}", e),
                }),
            )
        }
    }
}

/// Stop a service.
pub async fn stop_service(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let orchestrator = state.orchestrator.read().await;

    match orchestrator.stop_service(&id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                message: format!("Service '{}' stopped", id),
            }),
        ),
        Err(e) => {
            error!("Failed to stop service '{}': {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to stop service: {}", e),
                }),
            )
        }
    }
}

/// Configuration response (sanitized for API).
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub compositor: CompositorConfigInfo,
    pub event_bus: EventBusConfigInfo,
    pub orchestrator: OrchestratorConfigInfo,
    pub http_server: HttpServerConfigInfo,
}

#[derive(Debug, Serialize)]
pub struct CompositorConfigInfo {
    pub backend: String,
    pub renderer: String,
    pub fps: u32,
    pub vsync: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct EventBusConfigInfo {
    pub websocket_bind: String,
    pub unix_socket: String,
}

#[derive(Debug, Serialize)]
pub struct OrchestratorConfigInfo {
    pub runtime: String,
    pub health_interval: u64,
    pub max_restarts: u32,
    pub service_count: usize,
}

#[derive(Debug, Serialize)]
pub struct HttpServerConfigInfo {
    pub bind: String,
    pub port: u16,
    pub cors_enabled: bool,
}

/// Get current configuration.
pub async fn get_config(State(state): State<Arc<ServiceState>>) -> Json<ConfigResponse> {
    let config = &state.config;

    Json(ConfigResponse {
        compositor: CompositorConfigInfo {
            backend: config.compositor.backend.clone(),
            renderer: config.compositor.renderer.clone(),
            fps: config.compositor.fps,
            vsync: config.compositor.vsync,
            width: config.compositor.width,
            height: config.compositor.height,
        },
        event_bus: EventBusConfigInfo {
            websocket_bind: config.event_bus.websocket_bind.clone(),
            unix_socket: config.event_bus.unix_socket.display().to_string(),
        },
        orchestrator: OrchestratorConfigInfo {
            runtime: config.orchestrator.runtime.clone(),
            health_interval: config.orchestrator.health_interval,
            max_restarts: config.orchestrator.max_restarts,
            service_count: config.orchestrator.services.len(),
        },
        http_server: HttpServerConfigInfo {
            bind: config.http_server.bind.clone(),
            port: config.http_server.port,
            cors_enabled: config.http_server.cors_enabled,
        },
    })
}

/// Config update request.
#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    // For now, only allow updating select fields
    pub compositor_fps: Option<u32>,
    pub health_interval: Option<u64>,
}

/// Update configuration.
pub async fn update_config(
    State(_state): State<Arc<ServiceState>>,
    Json(_request): Json<ConfigUpdateRequest>,
) -> impl IntoResponse {
    // TODO: Implement config updates
    // This would require making Config mutable and potentially restarting services
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse {
            success: false,
            message: "Configuration updates not yet implemented".to_string(),
        }),
    )
}

/// System metrics.
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub gpu_usage_percent: Option<f64>,
    pub gpu_memory_used_mb: Option<u64>,
    pub uptime_seconds: u64,
}

/// Get system metrics.
pub async fn get_metrics() -> Json<MetricsResponse> {
    // TODO: Implement actual system metrics collection
    // This would use sysinfo crate or platform-specific APIs
    Json(MetricsResponse {
        cpu_usage_percent: 0.0,
        memory_used_mb: 0,
        memory_total_mb: 0,
        gpu_usage_percent: None,
        gpu_memory_used_mb: None,
        uptime_seconds: 0,
    })
}

/// Video source info.
#[derive(Debug, Serialize)]
pub struct VideoSourceInfo {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub status: String,
}

/// List video sources.
pub async fn list_video_sources(State(_state): State<Arc<ServiceState>>) -> Json<Vec<VideoSourceInfo>> {
    // TODO: Read video sources from config
    // For now, return placeholder data
    Json(vec![
        VideoSourceInfo {
            name: "EAGLE-1".to_string(),
            url: "rtsp://192.168.1.100:554/stream1".to_string(),
            enabled: true,
            status: "disconnected".to_string(),
        },
        VideoSourceInfo {
            name: "EAGLE-2".to_string(),
            url: "rtsp://192.168.1.101:554/stream1".to_string(),
            enabled: false,
            status: "disabled".to_string(),
        },
    ])
}

/// Host status response for the console.
#[derive(Debug, Serialize)]
pub struct HostStatusResponse {
    pub version: &'static str,
    pub platform: &'static str,
    pub arch: &'static str,
    pub event_bus: SubsystemStatus,
    pub http_server: SubsystemStatus,
    pub orchestrator: SubsystemStatus,
    pub compositor: SubsystemStatus,
    pub inference: SubsystemStatus,
}

#[derive(Debug, Serialize)]
pub struct SubsystemStatus {
    pub name: &'static str,
    pub status: &'static str,
    pub details: String,
}

/// Get host status for console display.
pub async fn get_status(State(state): State<Arc<ServiceState>>) -> Json<HostStatusResponse> {
    let orchestrator = state.orchestrator.read().await;
    let services = orchestrator.list_services().await;
    let running_count = services.iter().filter(|(_, s)| matches!(s, ServiceStatus::Running)).count();

    // Get inference subsystem status
    let inference_status = match &state.inference_service {
        Some(service) => {
            let backend_name = service.backend_name();
            let is_ready = service.is_ready().await;
            let status_str = if is_ready { "running" } else { "degraded" };
            let details = match backend_name {
                "mock" => "Mock backend (testing mode)".to_string(),
                "event_bus" => {
                    if is_ready {
                        "VLM Container connected".to_string()
                    } else {
                        "VLM Container not connected".to_string()
                    }
                }
                _ => format!("Unknown backend: {}", backend_name),
            };
            SubsystemStatus {
                name: "Inference",
                status: status_str,
                details,
            }
        }
        None => SubsystemStatus {
            name: "Inference",
            status: "stopped",
            details: "Service not initialized".to_string(),
        },
    };

    Json(HostStatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        event_bus: SubsystemStatus {
            name: "Event Bus",
            status: "running",
            details: format!("ws://{}", state.config.event_bus.websocket_bind),
        },
        http_server: SubsystemStatus {
            name: "HTTP Server",
            status: "running",
            details: format!("http://{}:{}", state.config.http_server.bind, state.config.http_server.port),
        },
        orchestrator: SubsystemStatus {
            name: "Orchestrator",
            status: "running",
            details: format!("{}/{} services running", running_count, services.len()),
        },
        compositor: SubsystemStatus {
            name: "Compositor",
            status: "running",
            details: format!("{} @ {}fps", state.config.compositor.renderer, state.config.compositor.fps),
        },
        inference: inference_status,
    })
}
