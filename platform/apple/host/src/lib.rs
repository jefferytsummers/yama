//! Yama Host Library
//!
//! Shared components for the Yama host application.
//! This module is used by both the GUI (`main.rs`) and headless (`bin/headless.rs`) binaries.

pub mod event_bus;
pub mod inference;

// Re-export commonly used types
pub use event_bus::{EventBus, EventBusConfig};
pub use inference::{
    BackendType, InferenceBackend, InferenceChunk, InferenceJob, JobStatus, MockBackend,
    UploadInfo, VlmInferenceConfig, VlmInferenceService, VlmModelInfo,
};
