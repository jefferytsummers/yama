//! HTTP handlers for chat sessions with SSE streaming.
//!
//! Provides REST API endpoints for chat session management and
//! Server-Sent Events (SSE) streaming for real-time message responses.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
    Json,
};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::agent::{ChatChunk, ChatSession, PresetRegistry, SessionConfig, ToolContext, ToolExecutor};
use crate::db::chat_history::ChatHistory;
use crate::inference::VlmInferenceService;
use crate::ServiceState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new chat session.
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// Preset ID to use for this session.
    pub preset_id: String,
    /// Optional project ID to associate with the session.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Optional title for the session.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional model override.
    #[serde(default)]
    pub model_override: Option<String>,
}

/// Request to send a message in a chat session.
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    /// Message content.
    pub content: String,
    /// Optional attachments (video/image references).
    #[serde(default)]
    pub attachments: Vec<MessageAttachment>,
}

/// Attachment for a chat message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageAttachment {
    /// Attachment type.
    #[serde(rename = "type")]
    pub attachment_type: AttachmentType,
    /// Upload ID (reference to previously uploaded file).
    #[serde(default)]
    pub upload_id: Option<String>,
    /// Inline base64-encoded data.
    #[serde(default)]
    pub base64_data: Option<String>,
}

/// Type of attachment.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentType {
    Video,
    Image,
}

/// Query parameters for listing sessions.
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    /// Filter by project ID.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Include archived sessions.
    #[serde(default)]
    pub include_archived: bool,
    /// Maximum number of sessions to return.
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination.
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// Response for a chat session.
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub preset_id: String,
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_archived: bool,
    pub message_count: i64,
}

/// Response for a chat message.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
    pub created_at: String,
    pub token_count: Option<i64>,
    pub model_id: Option<String>,
}

/// Response for an agent preset.
#[derive(Debug, Serialize)]
pub struct PresetResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub tags: Vec<String>,
    pub builtin: bool,
    pub tools: Vec<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new chat session.
///
/// POST /api/chat/sessions
pub async fn create_session(
    State(state): State<Arc<ServiceState>>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    // Validate preset exists
    let preset_registry = match get_preset_registry() {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to load preset registry: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to load presets" })),
            )
                .into_response();
        }
    };

    let preset = match preset_registry.get(&req.preset_id) {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Preset not found: {}", req.preset_id)
                })),
            )
                .into_response();
        }
    };

    // Get required services
    let chat_history = get_chat_history(&state);
    let vlm_service = match &state.inference_service {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Inference service not available" })),
            )
                .into_response();
        }
    };

    let executor = get_tool_executor();
    let tool_context = ToolContext::new(state.database.pool.clone());

    // Build session config
    let config = SessionConfig {
        model_override: req.model_override,
        ..SessionConfig::default()
    };

    // Create the session
    match ChatSession::new_with_details(
        preset,
        chat_history.clone(),
        vlm_service,
        executor,
        tool_context,
        config,
        req.project_id.as_deref(),
        req.title.as_deref(),
    )
    .await
    {
        Ok(session) => {
            let session_id = session.id().to_string();
            info!("Created chat session: {} (preset: {})", session_id, req.preset_id);

            // Get message count (should be 1 for system prompt)
            let message_count = chat_history
                .count_messages(&session_id)
                .await
                .unwrap_or(0);

            (
                StatusCode::CREATED,
                Json(SessionResponse {
                    id: session_id,
                    preset_id: req.preset_id,
                    project_id: req.project_id,
                    title: req.title,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    is_archived: false,
                    message_count,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to create chat session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create session: {}", e) })),
            )
                .into_response()
        }
    }
}

/// List chat sessions.
///
/// GET /api/chat/sessions
pub async fn list_sessions(
    State(state): State<Arc<ServiceState>>,
    Query(query): Query<ListSessionsQuery>,
) -> impl IntoResponse {
    let chat_history = get_chat_history(&state);

    match chat_history
        .list_sessions(
            query.project_id.as_deref(),
            query.include_archived,
            query.limit,
            query.offset,
        )
        .await
    {
        Ok(sessions) => {
            let mut responses = Vec::with_capacity(sessions.len());
            for s in sessions {
                let message_count = chat_history
                    .count_messages(&s.id)
                    .await
                    .unwrap_or(0);
                responses.push(SessionResponse {
                    id: s.id,
                    preset_id: s.preset_id,
                    project_id: s.project_id,
                    title: s.title,
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                    is_archived: s.is_archived,
                    message_count,
                });
            }
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to list sessions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to list sessions" })),
            )
                .into_response()
        }
    }
}

/// Get a specific chat session.
///
/// GET /api/chat/sessions/{id}
pub async fn get_session(
    State(state): State<Arc<ServiceState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let chat_history = get_chat_history(&state);

    match chat_history.get_session(&session_id).await {
        Ok(Some(s)) => {
            let message_count = chat_history
                .count_messages(&s.id)
                .await
                .unwrap_or(0);
            (
                StatusCode::OK,
                Json(SessionResponse {
                    id: s.id,
                    preset_id: s.preset_id,
                    project_id: s.project_id,
                    title: s.title,
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                    is_archived: s.is_archived,
                    message_count,
                }),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to get session {}: {}", session_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to get session" })),
            )
                .into_response()
        }
    }
}

/// Delete a chat session.
///
/// DELETE /api/chat/sessions/{id}
pub async fn delete_session(
    State(state): State<Arc<ServiceState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let chat_history = get_chat_history(&state);

    // Check if session exists first
    match chat_history.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Session not found" })),
            )
                .into_response();
        }
        Err(e) => {
            error!("Failed to check session {}: {}", session_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to check session" })),
            )
                .into_response();
        }
        Ok(Some(_)) => {}
    }

    match chat_history.delete_session(&session_id).await {
        Ok(()) => {
            info!("Deleted chat session: {}", session_id);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            error!("Failed to delete session {}: {}", session_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to delete session" })),
            )
                .into_response()
        }
    }
}

/// Get messages for a chat session.
///
/// GET /api/chat/sessions/{id}/messages
pub async fn get_messages(
    State(state): State<Arc<ServiceState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let chat_history = get_chat_history(&state);

    // Check if session exists first
    match chat_history.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Session not found" })),
            )
                .into_response();
        }
        Err(e) => {
            error!("Failed to check session {}: {}", session_id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to check session" })),
            )
                .into_response();
        }
        Ok(Some(_)) => {}
    }

    match chat_history.get_messages(&session_id).await {
        Ok(messages) => {
            let responses: Vec<MessageResponse> = messages
                .into_iter()
                .map(|m| MessageResponse {
                    id: m.id,
                    session_id: m.session_id,
                    role: format!("{:?}", m.role).to_lowercase(),
                    content: m.content,
                    tool_calls: m.tool_calls.map(|tc| serde_json::to_value(tc).unwrap_or_default()),
                    tool_call_id: m.tool_call_id,
                    created_at: m.created_at.to_rfc3339(),
                    token_count: m.token_count,
                    model_id: m.model_id,
                })
                .collect();
            (StatusCode::OK, Json(responses)).into_response()
        }
        Err(e) => {
            error!("Failed to get messages for session {}: {}", session_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to get messages" })),
            )
                .into_response()
        }
    }
}

/// Send a message to a chat session (SSE streaming response).
///
/// POST /api/chat/sessions/{id}/messages
///
/// Returns Server-Sent Events stream with the following event types:
/// - `text`: Text content chunk
/// - `tool_call`: Tool being invoked
/// - `tool_result`: Tool execution result
/// - `done`: Message complete
/// - `error`: Error occurred
pub async fn send_message(
    State(state): State<Arc<ServiceState>>,
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    // Get required services
    let chat_history = get_chat_history(&state);
    let preset_registry = match get_preset_registry() {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to load preset registry: {}", e);
            return error_sse_response("Failed to load presets").into_response();
        }
    };

    // Load the session
    let session_row = match chat_history.get_session(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return error_sse_response("Session not found").into_response();
        }
        Err(e) => {
            error!("Failed to get session {}: {}", session_id, e);
            return error_sse_response("Failed to get session").into_response();
        }
    };

    let vlm_service = match &state.inference_service {
        Some(s) => s.clone(),
        None => {
            return error_sse_response("Inference service not available").into_response();
        }
    };

    let executor = get_tool_executor();
    let tool_context = ToolContext::new(state.database.pool.clone());

    // Load the chat session
    let session = match ChatSession::load(
        &session_id,
        &preset_registry,
        chat_history,
        vlm_service,
        executor,
        tool_context,
        SessionConfig::default(),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load session {}: {}", session_id, e);
            return error_sse_response(format!("Failed to load session: {}", e)).into_response();
        }
    };

    // Send the message and get the streaming response
    let rx = session.send_message(req.content);

    // Convert mpsc::Receiver to SSE stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|chunk| {
        let event = match &chunk {
            ChatChunk::Text(text) => Event::default()
                .event("text")
                .json_data(&serde_json::json!({ "content": text }))
                .unwrap_or_else(|_| Event::default().data("error")),
            ChatChunk::ToolCall { id, name, arguments } => Event::default()
                .event("tool_call")
                .json_data(&serde_json::json!({
                    "id": id,
                    "name": name,
                    "arguments": arguments
                }))
                .unwrap_or_else(|_| Event::default().data("error")),
            ChatChunk::ToolResult { tool_call_id, success, content } => Event::default()
                .event("tool_result")
                .json_data(&serde_json::json!({
                    "tool_call_id": tool_call_id,
                    "success": success,
                    "content": content
                }))
                .unwrap_or_else(|_| Event::default().data("error")),
            ChatChunk::Done { token_count } => Event::default()
                .event("done")
                .json_data(&serde_json::json!({ "token_count": token_count }))
                .unwrap_or_else(|_| Event::default().data("done")),
            ChatChunk::Error(error) => Event::default()
                .event("error")
                .json_data(&serde_json::json!({ "error": error }))
                .unwrap_or_else(|_| Event::default().data("error")),
        };
        Ok::<_, Infallible>(event)
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("ping"))
        .into_response()
}

/// List available agent presets.
///
/// GET /api/chat/presets
pub async fn list_presets() -> impl IntoResponse {
    let preset_registry = match get_preset_registry() {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to load preset registry: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to load presets" })),
            )
                .into_response();
        }
    };

    let presets: Vec<PresetResponse> = preset_registry
        .all()
        .into_iter()
        .map(|p| PresetResponse {
            id: p.id.as_str().to_string(),
            name: p.name.clone(),
            description: p.description.clone(),
            icon: p.icon.clone(),
            tags: p.tags.clone(),
            builtin: p.builtin,
            tools: p.enabled_tools().into_iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    (StatusCode::OK, Json(presets)).into_response()
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get or create a ChatHistory instance from state.
fn get_chat_history(state: &ServiceState) -> Arc<ChatHistory> {
    Arc::new(ChatHistory::new(state.database.pool.clone()))
}

/// Get or create a PresetRegistry.
fn get_preset_registry() -> anyhow::Result<PresetRegistry> {
    PresetRegistry::load_builtin()
}

/// Get or create a ToolExecutor.
fn get_tool_executor() -> Arc<ToolExecutor> {
    use crate::agent::tools::ToolRegistry;
    let registry = ToolRegistry::with_builtins();
    Arc::new(ToolExecutor::new(registry).unwrap_or_else(|_| {
        ToolExecutor::new(ToolRegistry::new()).unwrap()
    }))
}

/// Create an error SSE response with a single error event.
fn error_sse_response(error: impl Into<String>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let error_msg = error.into();
    let stream = futures_util::stream::once(async move {
        Ok::<_, Infallible>(
            Event::default()
                .event("error")
                .json_data(&serde_json::json!({ "error": error_msg }))
                .unwrap_or_else(|_| Event::default().data("error")),
        )
    });
    Sse::new(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_create_session_request() {
        let json = r#"{"preset_id": "video-analyst"}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.preset_id, "video-analyst");
        assert!(req.project_id.is_none());
    }

    #[test]
    fn test_deserialize_send_message_request() {
        let json = r#"{"content": "Hello, world!"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Hello, world!");
        assert!(req.attachments.is_empty());
    }

    #[test]
    fn test_deserialize_send_message_with_attachments() {
        let json = r#"{
            "content": "Analyze this",
            "attachments": [
                {"type": "video", "upload_id": "abc123"}
            ]
        }"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Analyze this");
        assert_eq!(req.attachments.len(), 1);
        assert_eq!(req.attachments[0].attachment_type, AttachmentType::Video);
        assert_eq!(req.attachments[0].upload_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_list_sessions_query_defaults() {
        let json = r#"{}"#;
        let query: ListSessionsQuery = serde_json::from_str(json).unwrap();
        assert!(query.project_id.is_none());
        assert!(!query.include_archived);
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
    }
}
