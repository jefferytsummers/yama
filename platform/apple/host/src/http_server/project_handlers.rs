//! Project API handlers.
//!
//! CRUD operations for projects and libraries.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::db::{
    create_library, create_project, delete_project, get_project, list_libraries, list_projects,
    NewLibrary, NewProject,
};

use crate::ServiceState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new project.
#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Tags for the project (stored as JSON in description for now).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Project configuration.
    #[serde(default)]
    pub config: Option<crate::db::models::ProjectConfig>,
}

/// Request to create a new library.
#[derive(Debug, Deserialize)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub root_path: String,
}

/// Response for a project.
#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: Option<crate::db::models::ProjectConfig>,
    pub created_at: String,
    pub updated_at: String,
    pub library_count: i64,
}

/// Response for a library.
#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_indexed_at: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// List all projects.
///
/// GET /api/projects
pub async fn list_projects_handler(
    State(state): State<Arc<ServiceState>>,
) -> impl IntoResponse {
    match list_projects(&state.database.pool).await {
        Ok(projects) => {
            // Get library counts for each project
            let mut responses = Vec::with_capacity(projects.len());
            for p in projects {
                let library_count = match list_libraries(&state.database.pool, &p.id).await {
                    Ok(libs) => libs.len() as i64,
                    Err(_) => 0,
                };
                responses.push(ProjectResponse {
                    id: p.id,
                    name: p.name,
                    description: p.description,
                    config: p.config,
                    created_at: p.created_at,
                    updated_at: p.updated_at,
                    library_count,
                });
            }
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list projects: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list projects"
            }))).into_response()
        }
    }
}

/// Get a single project.
///
/// GET /api/projects/:id
pub async fn get_project_handler(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match get_project(&state.database.pool, &id).await {
        Ok(Some(p)) => {
            let library_count = match list_libraries(&state.database.pool, &p.id).await {
                Ok(libs) => libs.len() as i64,
                Err(_) => 0,
            };
            (StatusCode::OK, Json(ProjectResponse {
                id: p.id,
                name: p.name,
                description: p.description,
                config: p.config,
                created_at: p.created_at,
                updated_at: p.updated_at,
                library_count,
            })).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Project not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get project {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get project"
            }))).into_response()
        }
    }
}

/// Create a new project.
///
/// POST /api/projects
pub async fn create_project_handler(
    State(state): State<Arc<ServiceState>>,
    Json(req): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    // Build description with tags if provided
    let description = if !req.tags.is_empty() {
        let tags_json = serde_json::json!({
            "text": req.description.unwrap_or_default(),
            "tags": req.tags,
        });
        Some(tags_json.to_string())
    } else {
        req.description
    };

    match create_project(
        &state.database.pool,
        NewProject {
            name: req.name.clone(),
            description,
            config: req.config.clone(),
        },
    )
    .await
    {
        Ok(p) => {
            info!("Created project: {} ({})", p.name, p.id);
            (StatusCode::CREATED, Json(ProjectResponse {
                id: p.id,
                name: p.name,
                description: p.description,
                config: p.config,
                created_at: p.created_at,
                updated_at: p.updated_at,
                library_count: 0,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create project: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to create project"
            }))).into_response()
        }
    }
}

/// Delete a project.
///
/// DELETE /api/projects/:id
pub async fn delete_project_handler(
    State(state): State<Arc<ServiceState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match delete_project(&state.database.pool, &id).await {
        Ok(true) => {
            info!("Deleted project: {}", id);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Project not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete project {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to delete project"
            }))).into_response()
        }
    }
}

/// List libraries for a project.
///
/// GET /api/projects/:id/libraries
pub async fn list_libraries_handler(
    State(state): State<Arc<ServiceState>>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    // Verify project exists
    match get_project(&state.database.pool, &project_id).await {
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Project not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get project {}: {}", project_id, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get project"
            }))).into_response();
        }
        Ok(Some(_)) => {}
    }

    match list_libraries(&state.database.pool, &project_id).await {
        Ok(libraries) => {
            let responses: Vec<LibraryResponse> = libraries
                .into_iter()
                .map(|l| LibraryResponse {
                    id: l.id,
                    project_id: l.project_id,
                    name: l.name,
                    root_path: l.root_path,
                    created_at: l.created_at,
                    updated_at: l.updated_at,
                    last_indexed_at: l.last_indexed_at,
                })
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list libraries for project {}: {}", project_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to list libraries"
            }))).into_response()
        }
    }
}

/// Create a library for a project.
///
/// POST /api/projects/:id/libraries
pub async fn create_library_handler(
    State(state): State<Arc<ServiceState>>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateLibraryRequest>,
) -> impl IntoResponse {
    // Verify project exists
    match get_project(&state.database.pool, &project_id).await {
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Project not found"
            }))).into_response();
        }
        Err(e) => {
            error!("Failed to get project {}: {}", project_id, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to get project"
            }))).into_response();
        }
        Ok(Some(_)) => {}
    }

    match create_library(
        &state.database.pool,
        NewLibrary {
            project_id: project_id.clone(),
            name: req.name.clone(),
            root_path: req.root_path,
        },
    )
    .await
    {
        Ok(l) => {
            info!("Created library: {} ({}) in project {}", l.name, l.id, project_id);
            (StatusCode::CREATED, Json(LibraryResponse {
                id: l.id,
                project_id: l.project_id,
                name: l.name,
                root_path: l.root_path,
                created_at: l.created_at,
                updated_at: l.updated_at,
                last_indexed_at: l.last_indexed_at,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create library: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "Failed to create library"
            }))).into_response()
        }
    }
}
