//! Phase 3 Demo - Agent System Components
//!
//! Run with: cargo run -p yama-host-apple --example phase3_demo
//!
//! Demonstrates:
//! - Agent presets
//! - Tool definitions and execution
//! - Chat session with history
//! - Artifact storage

use std::sync::Arc;

use anyhow::Result;
use yama_host_apple::{
    AgentPreset, ArtifactStore, ArtifactStoreConfig, ArtifactType, ChatHistory, ChatSession,
    NewArtifact, PresetRegistry, SessionConfig, ToolContext, ToolExecutor, ToolRegistry,
    VlmInferenceService,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for nice output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("\n========================================");
    println!("  Yama Phase 3 Demo - Agent System");
    println!("========================================\n");

    // ========================================
    // 1. Agent Presets
    // ========================================
    println!("1. AGENT PRESETS");
    println!("   Loading built-in presets...\n");

    let preset_registry = PresetRegistry::load_builtin()?;

    for preset_id in ["video-analyst", "quick-search", "document-reporter", "custom"] {
        if let Some(preset) = preset_registry.get(preset_id) {
            println!("   [{}] {}", preset.id.as_str(), preset.name);
            println!("   Description: {}", preset.description);
            println!("   Tools: {:?}", preset.tools.iter().map(|t| &t.name).collect::<Vec<_>>());
            println!();
        }
    }

    // ========================================
    // 2. Tool Registry
    // ========================================
    println!("2. TOOL REGISTRY");
    println!("   Available tools for agent use:\n");

    let tool_registry = ToolRegistry::with_builtins();

    for tool_name in tool_registry.list() {
        if let Some(tool) = tool_registry.get(tool_name) {
            let def = tool.definition();
            println!("   [{}]", def.name);
            println!("   Description: {}", def.description);
            println!("   Parameters: {}", def.parameters.len());
            println!();
        }
    }

    // ========================================
    // 3. Tool Executor with Validation
    // ========================================
    println!("3. TOOL EXECUTOR");
    println!("   Demonstrating input validation...\n");

    let executor = ToolExecutor::new(ToolRegistry::with_builtins())?;

    // Show validation failure
    let ctx = ToolContext::without_db();
    let result = executor
        .execute("search_videos", serde_json::json!({}), ctx.clone())
        .await;

    println!("   Executing search_videos with empty input:");
    match result {
        Ok(_) => println!("   Result: Success (unexpected)"),
        Err(e) => println!("   Result: Validation failed - {}", e),
    }
    println!();

    // Show valid execution
    let result = executor
        .execute(
            "search_videos",
            serde_json::json!({"query": "person walking", "limit": 5}),
            ctx,
        )
        .await;

    println!("   Executing search_videos with valid input:");
    match result {
        Ok(r) => println!("   Result: Success - {} ({}ms)", r.success, r.execution_time_ms),
        Err(e) => println!("   Result: Error - {}", e),
    }
    println!();

    // ========================================
    // 4. In-Memory Database Demo
    // ========================================
    println!("4. CHAT HISTORY & ARTIFACTS");
    println!("   Using in-memory database...\n");

    let pool = sqlx::SqlitePool::connect(":memory:").await?;

    // Run migrations
    sqlx::query("CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY)")
        .execute(&pool)
        .await?;
    sqlx::query(include_str!("../src/db/migrations/001_initial.sql"))
        .execute(&pool)
        .await
        .ok();

    let chat_history = Arc::new(ChatHistory::new(pool.clone()));
    chat_history.migrate().await?;

    // Create a chat session
    let session = chat_history
        .create_session("video-analyst", None, Some("Demo Session"))
        .await?;

    println!("   Created chat session: {}", session.id);

    // Add some messages
    chat_history
        .add_message(&session.id, yama_host_apple::MessageRole::User, "What's in this video?", None, None, None, None)
        .await?;

    chat_history
        .add_message(
            &session.id,
            yama_host_apple::MessageRole::Assistant,
            "I can see a person walking through a park...",
            None,
            None,
            Some("vlm-default"),
            Some(42),
        )
        .await?;

    let messages = chat_history.get_messages(&session.id).await?;
    println!("   Messages in session: {}", messages.len());

    for msg in &messages {
        println!("   - [{}] {:.50}...", format!("{:?}", msg.role).to_lowercase(), msg.content);
    }
    println!();

    // ========================================
    // 5. Artifact Store
    // ========================================
    println!("5. ARTIFACT STORE");

    let temp_dir = tempfile::TempDir::new()?;
    let config = ArtifactStoreConfig {
        storage_dir: temp_dir.path().join("artifacts"),
        max_artifact_size: 10 * 1024 * 1024,
        auto_cleanup: false,
    };

    // Need to run artifact migration
    sqlx::query(include_str!("../src/db/migrations/003_chat_history.sql"))
        .execute(&pool)
        .await
        .ok();

    let artifact_store = ArtifactStore::new(pool.clone(), config).await?;

    // Create a demo artifact
    let artifact = artifact_store
        .create(NewArtifact {
            session_id: Some(session.id.clone()),
            artifact_type: ArtifactType::Summary,
            name: "video_summary.txt".to_string(),
            description: Some("AI-generated summary".to_string()),
            data: b"This video shows a person walking through a scenic park...".to_vec(),
            tags: vec!["demo".to_string(), "summary".to_string()],
            ..Default::default()
        })
        .await?;

    println!("   Created artifact: {} ({})", artifact.name, artifact.id);
    println!("   Type: {:?}", artifact.artifact_type);
    println!("   Size: {} bytes", artifact.size_bytes);
    println!("   Tags: {:?}", artifact.tags);
    println!("   Path: {}", artifact.path);
    println!();

    // Storage stats
    let storage = artifact_store.storage_used().await?;
    let counts = artifact_store.count_by_type().await?;
    println!("   Storage used: {} bytes", storage);
    println!("   Counts by type: {:?}", counts);

    println!("\n========================================");
    println!("  Demo Complete!");
    println!("========================================\n");

    Ok(())
}
