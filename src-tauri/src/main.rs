//! Yama Desktop Application
//!
//! Tauri wrapper that runs the SvelteKit frontend in a native window
//! with the Rust backend services running headlessly.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::thread;

use tauri::Manager;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod commands;

/// Application state shared with Tauri commands.
pub struct AppState {
    /// Flag indicating if backend services are ready.
    pub backend_ready: std::sync::atomic::AtomicBool,
}

fn main() {
    // Initialize logging
    let _guard = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .try_init();

    info!("Starting Yama (Tauri)");
    info!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Create shared state
    let app_state = Arc::new(AppState {
        backend_ready: std::sync::atomic::AtomicBool::new(false),
    });

    // Start backend services in a background thread
    let state_clone = app_state.clone();
    thread::spawn(move || {
        // Create a dedicated runtime for backend services
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        runtime.block_on(async move {
            info!("Starting backend services...");

            match yama_host_apple::run_headless().await {
                Ok(()) => {
                    info!("Backend services started successfully");
                    state_clone
                        .backend_ready
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Err(e) => {
                    error!("Failed to start backend services: {}", e);
                }
            }
        });
    });

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::pick_video_files,
            commands::get_backend_status,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Focus the window on startup
            window.set_focus().ok();

            info!("Tauri window created");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
