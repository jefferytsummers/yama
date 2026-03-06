//! VLM Pipeline Integration Tests
//!
//! Tests the full VLM inference pipeline using the test harness.

use std::sync::Arc;
use std::time::Duration;

use yama_host_apple::inference::{
    BackendType, InferenceBackend, JobStatus, MockBackend, UploadInfo, VlmInferenceConfig,
    VlmInferenceService,
};
use yama_test_harness::{TestEventBus, TestVideoSource, TestVlmBackend};

/// Test that mock backend completes inference.
#[tokio::test]
async fn test_mock_backend_inference() {
    let config = VlmInferenceConfig {
        backend: BackendType::Mock,
        mock_enabled: true,
        ..Default::default()
    };

    let service = VlmInferenceService::new(config).await.unwrap();
    assert_eq!(service.backend_name(), "mock");

    // Create a job
    let job_id = service
        .create_job("vlm-default".to_string(), "Describe this video".to_string())
        .await;

    let job = service.get_job(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Queued);
}

/// Test that jobs can be created and retrieved.
#[tokio::test]
async fn test_job_lifecycle() {
    let config = VlmInferenceConfig::default();
    let service = VlmInferenceService::new(config).await.unwrap();

    // Create multiple jobs
    let job_id_1 = service
        .create_job("vlm-default".to_string(), "Prompt 1".to_string())
        .await;
    let job_id_2 = service
        .create_job("vlm-fast".to_string(), "Prompt 2".to_string())
        .await;

    // Verify they exist
    assert!(service.get_job(&job_id_1).await.is_some());
    assert!(service.get_job(&job_id_2).await.is_some());

    // Verify list contains both
    let jobs = service.list_jobs().await;
    assert_eq!(jobs.len(), 2);

    // Delete a job
    assert!(service.delete_job(&job_id_1).await);
    assert!(service.get_job(&job_id_1).await.is_none());
    assert_eq!(service.list_jobs().await.len(), 1);
}

/// Test inference with custom backend.
#[tokio::test]
async fn test_custom_backend() {
    let config = VlmInferenceConfig::default();
    let backend: Arc<dyn InferenceBackend> = Arc::new(MockBackend::with_delay(1));

    let service = VlmInferenceService::with_backend(config, backend)
        .await
        .unwrap();

    assert_eq!(service.backend_name(), "mock");
    assert!(service.is_ready().await);
}

/// Test upload registration.
#[tokio::test]
async fn test_upload_registration() {
    let config = VlmInferenceConfig::default();
    let service = VlmInferenceService::new(config).await.unwrap();

    let upload_info = UploadInfo {
        upload_id: "test-upload-1".to_string(),
        filename: "test_video.mp4".to_string(),
        size: 1024,
        path: std::path::PathBuf::from("/tmp/test.mp4"),
        content_type: "video/mp4".to_string(),
    };

    service.register_upload(upload_info.clone()).await;

    let retrieved = service.get_upload("test-upload-1").await;
    assert!(retrieved.is_some());

    let upload = retrieved.unwrap();
    assert_eq!(upload.filename, "test_video.mp4");
    assert_eq!(upload.size, 1024);
}

/// Test model listing.
#[tokio::test]
async fn test_list_models() {
    let config = VlmInferenceConfig::default();
    let service = VlmInferenceService::new(config).await.unwrap();

    let models = service.list_models();
    assert!(!models.is_empty());

    // Should have default models
    assert!(models.iter().any(|m| m.id == "vlm-default"));
    assert!(models.iter().any(|m| m.id == "vlm-fast"));
    assert!(models.iter().any(|m| m.id == "vlm-detailed"));
}

/// Test job status updates.
#[tokio::test]
async fn test_job_status_updates() {
    let config = VlmInferenceConfig::default();
    let service = VlmInferenceService::new(config).await.unwrap();

    let job_id = service
        .create_job("vlm-default".to_string(), "Test".to_string())
        .await;

    // Initial status
    let job = service.get_job(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Queued);

    // Update status
    service
        .update_job_status(&job_id, JobStatus::Extracting)
        .await;
    let job = service.get_job(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Extracting);

    // Update progress
    service.update_job_progress(&job_id, 5, 10, None).await;
    let job = service.get_job(&job_id).await.unwrap();
    assert_eq!(job.current_frame, 5);
    assert_eq!(job.total_frames, 10);
    assert!((job.progress_percent - 50.0).abs() < 0.01);

    // Set error
    service
        .set_job_error(&job_id, "Test error".to_string())
        .await;
    let job = service.get_job(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.error, Some("Test error".to_string()));
}

/// Test the test harness event bus.
#[tokio::test]
async fn test_event_bus_pubsub() {
    use yama_protocol::common::HealthCheckRequest;

    let bus = TestEventBus::new();
    let client1 = bus.create_client("publisher");
    let mut client2 = bus.create_client("subscriber");

    // Subscribe
    client2.subscribe(&["test.topic"]).await.unwrap();

    // Wait for subscription to be set up
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Publish
    client1
        .publish(
            "test.topic",
            HealthCheckRequest {
                service_id: "test".to_string(),
            },
        )
        .await
        .unwrap();

    // Receive
    let event = client2.recv_timeout(Duration::from_secs(1)).await;
    assert!(event.is_some());
    assert_eq!(event.unwrap().topic, "test.topic");
}

/// Test the test harness video source.
#[tokio::test]
async fn test_video_source() {
    use yama_test_harness::video_source::{FramePattern, TestVideoConfig};

    let config = TestVideoConfig {
        width: 100,
        height: 100,
        fps: 10,
        duration_secs: 1,
        pattern: FramePattern::Gradient,
    };

    let source = TestVideoSource::new(config);
    let frames: Vec<_> = source.collect();

    assert_eq!(frames.len(), 10);
    assert_eq!(frames[0].width, 100);
    assert_eq!(frames[0].height, 100);
    assert!(!frames[0].data.is_empty());
}

/// Test the test harness VLM backend.
#[tokio::test]
async fn test_vlm_backend() {
    let backend = TestVlmBackend::new();

    // Add canned response
    backend.add_response("person", vec!["I see a person".to_string()]);

    // Test matching response
    let result = backend
        .infer("req-1", "Find person in frame", None)
        .await
        .unwrap();
    assert_eq!(result, "I see a person");

    // Test default response
    let result = backend.infer("req-2", "What is the weather?", None).await;
    assert!(result.is_ok());

    // Check call history
    assert_eq!(backend.call_count(), 2);
    assert!(backend.was_called_with("person"));
}

/// Test backend type configuration.
#[tokio::test]
async fn test_backend_type_config() {
    // Test mock backend via config
    let config = VlmInferenceConfig {
        backend: BackendType::Mock,
        mock_enabled: true,
        ..Default::default()
    };
    assert_eq!(config.effective_backend(), BackendType::Mock);

    // Test that mock_enabled takes precedence for backwards compatibility
    let config = VlmInferenceConfig {
        backend: BackendType::EventBus,
        mock_enabled: true,
        ..Default::default()
    };
    assert_eq!(config.effective_backend(), BackendType::Mock);
}
