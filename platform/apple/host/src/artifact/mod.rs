//! Artifact storage for agent-generated content.
//!
//! Artifacts are files generated during agent conversations, such as:
//! - Video clips extracted from source videos
//! - Screenshots and keyframe exports
//! - Analysis reports and summaries
//! - Comparison visualizations
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::artifact::{ArtifactStore, ArtifactType, NewArtifact};
//!
//! let store = ArtifactStore::new(db_pool).await?;
//!
//! // Create an artifact from a file
//! let artifact = store.create(NewArtifact {
//!     artifact_type: ArtifactType::VideoClip,
//!     name: "highlight_clip.mp4".to_string(),
//!     data: file_bytes,
//!     ..Default::default()
//! }).await?;
//!
//! // List artifacts for a session
//! let artifacts = store.list_by_session("session-123").await?;
//! ```

mod store;

pub use store::{
    Artifact, ArtifactMetadata, ArtifactQuery, ArtifactStore, ArtifactStoreConfig, ArtifactType,
    NewArtifact,
};
