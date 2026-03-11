//! Tauri IPC commands.
//!
//! These commands are exposed to the frontend via Tauri's invoke mechanism.
//! They provide native OS functionality that isn't available in the browser.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tracing::info;

use crate::AppState;

/// Video file information returned from the file picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    /// Full path to the video file.
    pub path: String,
    /// File name without directory.
    pub filename: String,
    /// File size in bytes.
    pub size: u64,
}

/// Open the native file picker and select video files.
///
/// Returns a list of selected video files with their metadata.
#[tauri::command]
pub async fn pick_video_files(app: tauri::AppHandle) -> Result<Vec<VideoFile>, String> {
    info!("Opening native file picker for video files");

    // Use the dialog plugin to open a file picker
    let file_paths = app
        .dialog()
        .file()
        .add_filter("Video Files", &["mp4", "webm", "mov", "avi", "mkv", "m4v"])
        .set_title("Select Video Files")
        .blocking_pick_files();

    let Some(paths) = file_paths else {
        // User cancelled
        info!("File picker cancelled");
        return Ok(Vec::new());
    };

    let mut videos = Vec::new();

    for file_path in paths {
        // In Tauri 2.0, FilePath is an enum - extract the path
        let Some(path) = file_path.as_path() else {
            continue; // Skip URLs, we only handle local files
        };
        let path = path.to_path_buf();
        let path_str = path.to_string_lossy().to_string();

        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        info!("Selected video: {} ({} bytes)", filename, size);

        videos.push(VideoFile {
            path: path_str,
            filename,
            size,
        });
    }

    Ok(videos)
}

/// Get the current status of backend services.
#[tauri::command]
pub fn get_backend_status(state: State<'_, Arc<AppState>>) -> BackendStatus {
    let ready = state
        .backend_ready
        .load(std::sync::atomic::Ordering::SeqCst);

    BackendStatus {
        ready,
        http_port: 8080,
        event_bus_port: 8765,
    }
}

/// Backend service status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    /// Whether backend services are ready.
    pub ready: bool,
    /// HTTP server port.
    pub http_port: u16,
    /// Event bus WebSocket port.
    pub event_bus_port: u16,
}
