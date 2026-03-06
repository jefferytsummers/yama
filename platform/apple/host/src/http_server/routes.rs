//! API route definitions for the HTTP server.

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};
use axum::Router;

use crate::AppState;

use super::handlers;
use super::inference_handlers;

/// Build the API router with all endpoints.
pub fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        // Host status (for console)
        .route("/status", get(handlers::get_status))
        // Services
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
        // VLM Inference endpoints
        .route(
            "/inference/upload",
            post(inference_handlers::upload_video)
                .layer(DefaultBodyLimit::max(500 * 1024 * 1024)), // 500MB limit
        )
        .route("/inference/start", post(inference_handlers::start_inference))
        .route("/inference/jobs/{id}", get(inference_handlers::get_inference_job))
        .route("/inference/models", get(inference_handlers::list_vlm_models))
        .with_state(state)
}
