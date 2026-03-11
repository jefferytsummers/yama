//! Yama Host Library
//!
//! Shared components for the Yama host application.
//! This module is used by both the GUI (`main.rs`) and headless (`bin/headless.rs`) binaries.

pub mod agent;
pub mod artifact;
pub mod db;
pub mod embedding;
pub mod event_bus;
pub mod indexer;
pub mod inference;
pub mod models;
pub mod transcription;

// Re-export commonly used types
pub use db::{Database, DatabaseStats};
pub use event_bus::{EventBus, EventBusConfig};
pub use inference::{
    BackendType, InferenceBackend, InferenceChunk, InferenceJob, JobStatus, MockBackend,
    UploadInfo, VlmInferenceConfig, VlmInferenceService, VlmModelInfo,
};
pub use models::{ModelManager, ModelManagerConfig, ModelRegistry, ModelType};

// Embedding re-exports
pub use embedding::{
    ClipConfig, ClipEmbedder, ClipModel, ClipTextEmbedder, ImagePreprocessor, TextEmbedder,
    TextEmbedderConfig, CLIP_EMBEDDING_DIM, TEXT_EMBEDDING_DIM,
};

// Transcription re-exports
pub use transcription::{
    AudioExtractor, ExtractedAudio, ModelSize, TranscriptionResult, TranscriptionSegment,
    WhisperConfig, WhisperTranscriber, Word,
};

// Agent re-exports
pub use agent::{
    AgentPreset, AuditEntry, ChatChunk, ChatSession, PresetId, PresetRegistry, RateLimitConfig,
    SessionConfig, Tool, ToolConfig, ToolContext, ToolDefinition, ToolExecutor, ToolRegistry,
    ToolResult, ToolValidator, ValidationError, ValidationResult,
};

// Chat history re-exports
pub use db::{ChatHistory, ChatMessageRow, ChatSessionRow, MessageRole, StoredToolCall};

// Artifact re-exports
pub use artifact::{
    Artifact, ArtifactMetadata, ArtifactQuery, ArtifactStore, ArtifactStoreConfig, ArtifactType,
    NewArtifact,
};
