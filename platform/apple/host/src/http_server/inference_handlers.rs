//! HTTP handlers for VLM video inference.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::inference::{InferenceChunk, JobStatus, UploadInfo, VlmModelInfo};
use crate::AppState;

/// Upload response.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub upload_id: Option<String>,
    pub filename: Option<String>,
    pub size: Option<u64>,
    pub message: String,
}

/// Upload a video file for inference.
pub async fn upload_video(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let inference_service = match &state.inference_service {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(UploadResponse {
                    success: false,
                    upload_id: None,
                    filename: None,
                    size: None,
                    message: "Inference service not available".to_string(),
                }),
            );
        }
    };

    let config = inference_service.config();
    let max_size = config.max_upload_size;
    let upload_dir = config.upload_dir.clone();

    // Process multipart form
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "video.mp4".to_string());

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "video/mp4".to_string());

        // Validate content type
        let valid_types = ["video/mp4", "video/webm", "video/quicktime", "video/x-msvideo"];
        if !valid_types.iter().any(|t| content_type.starts_with(t)) {
            return (
                StatusCode::BAD_REQUEST,
                Json(UploadResponse {
                    success: false,
                    upload_id: None,
                    filename: Some(filename),
                    size: None,
                    message: format!("Invalid content type: {}. Supported: mp4, webm, mov", content_type),
                }),
            );
        }

        // Read the file data
        let data: Bytes = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to read upload: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(UploadResponse {
                        success: false,
                        upload_id: None,
                        filename: Some(filename),
                        size: None,
                        message: format!("Failed to read upload: {}", e),
                    }),
                );
            }
        };

        let size = data.len() as u64;

        // Check size limit
        if size > max_size {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(UploadResponse {
                    success: false,
                    upload_id: None,
                    filename: Some(filename),
                    size: Some(size),
                    message: format!(
                        "File too large: {} bytes (max: {} bytes)",
                        size, max_size
                    ),
                }),
            );
        }

        // Generate upload ID and save file
        let upload_id = Uuid::new_v4().to_string();
        let extension = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        let save_filename = format!("{}.{}", upload_id, extension);
        let save_path = upload_dir.join(&save_filename);

        // Save the file
        match tokio::fs::File::create(&save_path).await {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&data).await {
                    error!("Failed to write upload: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(UploadResponse {
                            success: false,
                            upload_id: None,
                            filename: Some(filename),
                            size: Some(size),
                            message: format!("Failed to save file: {}", e),
                        }),
                    );
                }
            }
            Err(e) => {
                error!("Failed to create file: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(UploadResponse {
                        success: false,
                        upload_id: None,
                        filename: Some(filename),
                        size: Some(size),
                        message: format!("Failed to create file: {}", e),
                    }),
                );
            }
        }

        // Register the upload
        let upload_info = UploadInfo {
            upload_id: upload_id.clone(),
            filename: filename.clone(),
            size,
            path: save_path,
            content_type,
        };

        inference_service.register_upload(upload_info).await;

        info!("Video uploaded: {} ({} bytes)", filename, size);

        return (
            StatusCode::OK,
            Json(UploadResponse {
                success: true,
                upload_id: Some(upload_id),
                filename: Some(filename),
                size: Some(size),
                message: "Upload successful".to_string(),
            }),
        );
    }

    // No file found in multipart
    (
        StatusCode::BAD_REQUEST,
        Json(UploadResponse {
            success: false,
            upload_id: None,
            filename: None,
            size: None,
            message: "No file provided in request".to_string(),
        }),
    )
}

/// Start inference request.
#[derive(Debug, Deserialize)]
pub struct StartInferenceRequest {
    pub source: String, // "upload" or "stream"
    pub upload_id: Option<String>,
    pub stream_source: Option<String>,
    pub model: String,
    pub prompt: String,
}

/// Start inference response.
#[derive(Debug, Serialize)]
pub struct StartInferenceResponse {
    pub success: bool,
    pub job_id: Option<String>,
    pub message: String,
}

/// Start inference on a video.
pub async fn start_inference(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StartInferenceRequest>,
) -> impl IntoResponse {
    let inference_service = match &state.inference_service {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(StartInferenceResponse {
                    success: false,
                    job_id: None,
                    message: "Inference service not available".to_string(),
                }),
            );
        }
    };

    // Validate source
    if request.source != "upload" {
        return (
            StatusCode::BAD_REQUEST,
            Json(StartInferenceResponse {
                success: false,
                job_id: None,
                message: "Only 'upload' source is currently supported".to_string(),
            }),
        );
    }

    // Validate upload_id
    let upload_id = match &request.upload_id {
        Some(id) => id.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(StartInferenceResponse {
                    success: false,
                    job_id: None,
                    message: "upload_id is required for 'upload' source".to_string(),
                }),
            );
        }
    };

    // Verify upload exists
    if inference_service.get_upload(&upload_id).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(StartInferenceResponse {
                success: false,
                job_id: None,
                message: format!("Upload not found: {}", upload_id),
            }),
        );
    }

    // Validate model
    let valid_models: Vec<_> = inference_service.list_models().iter().map(|m| &m.id).collect();
    if !valid_models.contains(&&request.model) {
        return (
            StatusCode::BAD_REQUEST,
            Json(StartInferenceResponse {
                success: false,
                job_id: None,
                message: format!(
                    "Invalid model: {}. Available: {:?}",
                    request.model, valid_models
                ),
            }),
        );
    }

    // Create the job
    let job_id = inference_service
        .create_job(request.model.clone(), request.prompt.clone())
        .await;

    // Start inference
    match inference_service.start_inference(job_id.clone(), upload_id).await {
        Ok(()) => {
            info!("Started inference job: {}", job_id);
            (
                StatusCode::OK,
                Json(StartInferenceResponse {
                    success: true,
                    job_id: Some(job_id),
                    message: "Inference started".to_string(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to start inference: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(StartInferenceResponse {
                    success: false,
                    job_id: Some(job_id),
                    message: format!("Failed to start inference: {}", e),
                }),
            )
        }
    }
}

/// Inference job response.
#[derive(Debug, Serialize)]
pub struct InferenceJobResponse {
    pub job_id: String,
    pub status: String,
    pub progress_percent: f32,
    pub current_frame: u64,
    pub total_frames: u64,
    pub results: Vec<InferenceChunk>,
    pub error: Option<String>,
    pub model: String,
    pub prompt: String,
}

/// Get inference job status and results.
pub async fn get_inference_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let inference_service = match &state.inference_service {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Inference service not available"
                })),
            );
        }
    };

    match inference_service.get_job(&job_id).await {
        Some(job) => (
            StatusCode::OK,
            Json(serde_json::json!(InferenceJobResponse {
                job_id: job.job_id,
                status: job.status.to_string(),
                progress_percent: job.progress_percent,
                current_frame: job.current_frame,
                total_frames: job.total_frames,
                results: job.results,
                error: job.error,
                model: job.model,
                prompt: job.prompt,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Job not found: {}", job_id)
            })),
        ),
    }
}

/// VLM models response.
#[derive(Debug, Serialize)]
pub struct VlmModelsResponse {
    pub models: Vec<VlmModelInfo>,
}

/// List available VLM models.
pub async fn list_vlm_models(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let inference_service = match &state.inference_service {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(VlmModelsResponse { models: vec![] }),
            );
        }
    };

    let models = inference_service.list_models().to_vec();

    (StatusCode::OK, Json(VlmModelsResponse { models }))
}
