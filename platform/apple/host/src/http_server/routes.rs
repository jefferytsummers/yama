//! API route definitions for the HTTP server.
//!
//! Routes are organized into three categories:
//!
//! ## Admin Routes (Control Plane)
//! For the web dashboard - system operators monitoring and managing services.
//! Mounted at `/api/` and include:
//! - `/api/health` - Health check
//! - `/api/status` - Host status with all subsystem info
//! - `/api/services/*` - Service management
//! - `/api/config` - Configuration
//! - `/api/metrics` - System metrics
//! - `/api/video-sources` - Video source management
//!
//! ## Project Routes (Data Management)
//! For project and library management.
//! Mounted at `/api/` and include:
//! - `/api/projects` - List/create projects
//! - `/api/projects/{id}` - Get/delete project
//! - `/api/projects/{id}/libraries` - List/create libraries
//!
//! ## Inference API Routes (Data Plane)
//! For external tools and scripts that need programmatic access to VLM inference.
//! The web UI does NOT use these - they exist for CLI tools, automation, etc.
//! Mounted at `/api/inference/` and include:
//! - `/api/inference/upload` - Upload video for analysis
//! - `/api/inference/start` - Start inference job
//! - `/api/inference/jobs/{id}` - Get job status
//! - `/api/inference/models` - List available models

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::ServiceState;

use super::handlers;
use super::inference_handlers;
use super::project_handlers;

/// Build the complete API router with all endpoints.
///
/// This combines admin routes (for web dashboard), project routes (for project management),
/// and inference API routes (for external tools/scripts).
pub fn api_routes(state: Arc<ServiceState>) -> Router {
    admin_routes(state.clone())
        .merge(project_routes(state.clone()))
        .merge(inference_api_routes(state))
}

/// Admin routes for the web dashboard (control plane).
///
/// These endpoints are used by the Svelte web UI for:
/// - Monitoring system health and status
/// - Managing containerized services
/// - Viewing configuration and metrics
///
/// The web dashboard is for system operators/engineers, not end users.
pub fn admin_routes(state: Arc<ServiceState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Host status (comprehensive system status for dashboard)
        .route("/status", get(handlers::get_status))
        // Services management
        .route("/services", get(handlers::list_services))
        .route("/services/{id}/start", post(handlers::start_service))
        .route("/services/{id}/stop", post(handlers::stop_service))
        // Configuration
        .route("/config", get(handlers::get_config))
        .route("/config", put(handlers::update_config))
        // Metrics
        .route("/metrics", get(handlers::get_metrics))
        // Video sources
        .route("/video-sources", get(handlers::list_video_sources))
        .with_state(state)
}

/// Project management routes.
///
/// These endpoints manage projects and libraries for the web UI:
/// - `/projects` - List/create projects
/// - `/projects/:id` - Get/delete project
/// - `/projects/:id/libraries` - List/create libraries
pub fn project_routes(state: Arc<ServiceState>) -> Router {
    Router::new()
        // Projects
        .route("/projects", get(project_handlers::list_projects_handler))
        .route("/projects", post(project_handlers::create_project_handler))
        .route("/projects/{id}", get(project_handlers::get_project_handler))
        .route(
            "/projects/{id}",
            delete(project_handlers::delete_project_handler),
        )
        // Libraries
        .route(
            "/projects/{id}/libraries",
            get(project_handlers::list_libraries_handler),
        )
        .route(
            "/projects/{id}/libraries",
            post(project_handlers::create_library_handler),
        )
        .with_state(state)
}

/// Inference API routes for external tools (data plane).
///
/// These endpoints provide programmatic access to VLM inference for:
/// - CLI tools and scripts
/// - Automation pipelines
/// - Integration with other systems
///
/// NOTE: The web dashboard does NOT consume these endpoints. The egui
/// native app handles user-facing video analysis. These are exposed for
/// external tool integration only.
pub fn inference_api_routes(state: Arc<ServiceState>) -> Router {
    Router::new()
        // Video upload (500MB limit)
        .route(
            "/inference/upload",
            post(inference_handlers::upload_video)
                .layer(DefaultBodyLimit::max(500 * 1024 * 1024)),
        )
        // Start inference job
        .route("/inference/start", post(inference_handlers::start_inference))
        // Get job status
        .route("/inference/jobs/{id}", get(inference_handlers::get_inference_job))
        // List available models
        .route("/inference/models", get(inference_handlers::list_vlm_models))
        .with_state(state)
}
