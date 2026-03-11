//! API integration tests.
//!
//! These tests validate all HTTP API endpoints.
//! Run with: cargo test -p yama-host-apple --test api_integration
//!
//! Prerequisites:
//! - The host service must be running on localhost:8080
//! - Run `cargo run -p yama-host-apple --bin yama` before running tests

use reqwest::Client;
use serde_json::{json, Value};

const BASE_URL: &str = "http://localhost:8080/api";

/// Check if server is available.
async fn server_available() -> bool {
    let client = Client::new();
    client
        .get(format!("{}/health", BASE_URL))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok()
}

// ============================================================================
// Phase 1: Project CRUD Tests
// ============================================================================

#[tokio::test]
async fn test_health_check() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/health", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_project_crud() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();

    // Create project
    let resp = client
        .post(format!("{}/projects", BASE_URL))
        .json(&json!({
            "name": "Integration Test Project",
            "description": "Created by integration tests",
            "config": {
                "default_model": "test-model",
                "enabled_tools": ["search_videos"]
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let project: Value = resp.json().await.unwrap();
    let id = project["id"].as_str().unwrap();
    assert_eq!(project["name"], "Integration Test Project");
    assert!(project["config"].is_object());

    // Read project
    let resp = client
        .get(format!("{}/projects/{}", BASE_URL, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let fetched: Value = resp.json().await.unwrap();
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["name"], "Integration Test Project");

    // List projects
    let resp = client
        .get(format!("{}/projects", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let projects: Vec<Value> = resp.json().await.unwrap();
    assert!(projects.iter().any(|p| p["id"] == id));

    // Delete project
    let resp = client
        .delete(format!("{}/projects/{}", BASE_URL, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Verify deletion
    let resp = client
        .get(format!("{}/projects/{}", BASE_URL, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_project_libraries() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();

    // Create project
    let resp = client
        .post(format!("{}/projects", BASE_URL))
        .json(&json!({
            "name": "Library Test Project"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let project: Value = resp.json().await.unwrap();
    let project_id = project["id"].as_str().unwrap();

    // Create library
    let resp = client
        .post(format!("{}/projects/{}/libraries", BASE_URL, project_id))
        .json(&json!({
            "name": "Test Library",
            "root_path": "/tmp/test-videos"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let library: Value = resp.json().await.unwrap();
    assert_eq!(library["name"], "Test Library");
    assert_eq!(library["project_id"], project_id);

    // List libraries
    let resp = client
        .get(format!("{}/projects/{}/libraries", BASE_URL, project_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let libraries: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(libraries.len(), 1);

    // Cleanup
    client
        .delete(format!("{}/projects/{}", BASE_URL, project_id))
        .send()
        .await
        .unwrap();
}

// ============================================================================
// Phase 2: Inference API Tests
// ============================================================================

#[tokio::test]
async fn test_inference_models() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/inference/models", BASE_URL))
        .send()
        .await
        .unwrap();

    // May be 200 or 503 depending on service state
    let status = resp.status().as_u16();
    assert!(status == 200 || status == 503);

    if status == 200 {
        let body: Value = resp.json().await.unwrap();
        assert!(body["models"].is_array());
    }
}

#[tokio::test]
async fn test_inference_invalid_model() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .post(format!("{}/inference/start", BASE_URL))
        .json(&json!({
            "source": "upload",
            "upload_id": "nonexistent",
            "model": "invalid-model",
            "prompt": "test"
        }))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    // Should be 400 (invalid model) or 404 (upload not found)
    assert!(status == 400 || status == 404);
}

// ============================================================================
// Phase 3: Chat Session Tests
// ============================================================================

#[tokio::test]
async fn test_chat_presets() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/chat/presets", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let presets: Vec<Value> = resp.json().await.unwrap();
    assert!(!presets.is_empty());

    // Verify preset structure
    let first = &presets[0];
    assert!(first["id"].is_string());
    assert!(first["name"].is_string());
    assert!(first["tools"].is_array());
}

#[tokio::test]
async fn test_chat_session_lifecycle() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();

    // Create session
    let resp = client
        .post(format!("{}/chat/sessions", BASE_URL))
        .json(&json!({
            "preset_id": "quick-search",
            "title": "Integration Test Session"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let session: Value = resp.json().await.unwrap();
    let session_id = session["id"].as_str().unwrap();
    assert_eq!(session["preset_id"], "quick-search");

    // Get session
    let resp = client
        .get(format!("{}/chat/sessions/{}", BASE_URL, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Get messages (should have system prompt)
    let resp = client
        .get(format!("{}/chat/sessions/{}/messages", BASE_URL, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let messages: Vec<Value> = resp.json().await.unwrap();
    assert!(!messages.is_empty());

    // Delete session
    let resp = client
        .delete(format!("{}/chat/sessions/{}", BASE_URL, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Verify deletion
    let resp = client
        .get(format!("{}/chat/sessions/{}", BASE_URL, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_list_sessions() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();

    // List sessions (may be empty or have sessions from other tests)
    let resp = client
        .get(format!("{}/chat/sessions?limit=10", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let sessions: Vec<Value> = resp.json().await.unwrap();
    // Just verify it's an array
    assert!(sessions.len() <= 10);
}

// ============================================================================
// Phase 5: Admin Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_config_endpoint() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/config", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let config: Value = resp.json().await.unwrap();
    assert!(config["compositor"].is_object());
    assert!(config["event_bus"].is_object());
    assert!(config["http_server"].is_object());
}

#[tokio::test]
async fn test_metrics_endpoint() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/metrics", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let metrics: Value = resp.json().await.unwrap();
    assert!(metrics["cpu_usage_percent"].is_number());
    assert!(metrics["uptime_seconds"].is_number());
}

#[tokio::test]
async fn test_video_sources_endpoint() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/video-sources", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let sources: Vec<Value> = resp.json().await.unwrap();
    // Verify array (may be empty or have placeholder data)
    assert!(sources.is_empty() || sources[0]["name"].is_string());
}

#[tokio::test]
async fn test_services_endpoint() {
    if !server_available().await {
        eprintln!("Skipping test: server not available at {}", BASE_URL);
        return;
    }

    let client = Client::new();
    let resp = client
        .get(format!("{}/services", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let services: Vec<Value> = resp.json().await.unwrap();
    // Verify array (may be empty)
    for service in &services {
        assert!(service["name"].is_string());
        assert!(service["status"].is_string());
    }
}
