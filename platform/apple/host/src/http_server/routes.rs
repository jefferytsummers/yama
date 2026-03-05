//! API route definitions for the HTTP server.

use std::sync::Arc;

use axum::routing::{get, post, put};
use axum::Router;

use crate::AppState;

use super::handlers;

/// Build the API router with all endpoints.
pub fn api_routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
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
        .with_state(state)
}
