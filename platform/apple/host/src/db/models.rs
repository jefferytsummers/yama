//! Database row models for Yama.
//!
//! These structs map directly to database tables and are used with sqlx.

use serde::{Deserialize, Serialize};

/// Project-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Default preset ID for new chat sessions.
    #[serde(default)]
    pub default_preset_id: Option<String>,
    /// Default model to use.
    #[serde(default)]
    pub default_model: Option<String>,
    /// Enabled tools for this project.
    #[serde(default)]
    pub enabled_tools: Vec<String>,
}

/// A project organizes video libraries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: Option<ProjectConfig>,
    pub created_at: String,
    pub updated_at: String,
}

/// A project row without config parsing (for direct DB queries).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ProjectRow {
    /// Convert to Project with parsed config.
    pub fn into_project(self) -> Project {
        let config = self.config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());
        Project {
            id: self.id,
            name: self.name,
            description: self.description,
            config,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// A library is a directory containing videos.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Library {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_indexed_at: Option<String>,
}

/// A video is an individual media file.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Video {
    pub id: String,
    pub library_id: String,
    pub path: String,
    pub filename: String,
    pub duration_ms: i64,
    pub width: i32,
    pub height: i32,
    pub fps: f64,
    pub codec: String,
    pub container: Option<String>,
    pub file_size_bytes: i64,
    pub file_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub indexed_at: Option<String>,
}

/// Segment types for video portions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentType {
    Scene,
    Chapter,
    Manual,
    Silence,
    Speech,
}

impl SegmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Chapter => "chapter",
            Self::Manual => "manual",
            Self::Silence => "silence",
            Self::Speech => "speech",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scene" => Some(Self::Scene),
            "chapter" => Some(Self::Chapter),
            "manual" => Some(Self::Manual),
            "silence" => Some(Self::Silence),
            "speech" => Some(Self::Speech),
            _ => None,
        }
    }
}

/// A segment is a contiguous portion of video.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Segment {
    pub id: String,
    pub video_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub segment_type: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_path: Option<String>,
    pub created_at: String,
}

impl Segment {
    /// Get the segment type as an enum.
    pub fn segment_type_enum(&self) -> Option<SegmentType> {
        SegmentType::from_str(&self.segment_type)
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        self.end_ms - self.start_ms
    }
}

/// Keyframe extraction methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    Interval,
    SceneChange,
    Iframe,
    Manual,
}

impl ExtractionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Interval => "interval",
            Self::SceneChange => "scene_change",
            Self::Iframe => "iframe",
            Self::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "interval" => Some(Self::Interval),
            "scene_change" => Some(Self::SceneChange),
            "iframe" => Some(Self::Iframe),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }
}

/// A keyframe is an individual frame extracted for analysis.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Keyframe {
    pub id: String,
    pub video_id: String,
    pub timestamp_ms: i64,
    pub frame_number: i64,
    pub image_path: String,
    pub width: i32,
    pub height: i32,
    pub extraction_method: String,
    pub scene_score: Option<f64>,
    pub created_at: String,
}

impl Keyframe {
    /// Get the extraction method as an enum.
    pub fn extraction_method_enum(&self) -> Option<ExtractionMethod> {
        ExtractionMethod::from_str(&self.extraction_method)
    }
}

/// Embedding source types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingSourceType {
    Keyframe,
    Segment,
    Transcript,
}

impl EmbeddingSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keyframe => "keyframe",
            Self::Segment => "segment",
            Self::Transcript => "transcript",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "keyframe" => Some(Self::Keyframe),
            "segment" => Some(Self::Segment),
            "transcript" => Some(Self::Transcript),
            _ => None,
        }
    }
}

/// An embedding stores a vector representation for similarity search.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Embedding {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub model_id: String,
    pub embedding: Vec<u8>,
    pub dimensions: i32,
    pub created_at: String,
}

impl Embedding {
    /// Get the source type as an enum.
    pub fn source_type_enum(&self) -> Option<EmbeddingSourceType> {
        EmbeddingSourceType::from_str(&self.source_type)
    }

    /// Decode the embedding from bytes to f32 vector.
    pub fn to_f32_vec(&self) -> Vec<f32> {
        self.embedding
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    /// Encode an f32 vector to bytes.
    pub fn from_f32_vec(vec: &[f32]) -> Vec<u8> {
        vec.iter().flat_map(|f| f.to_le_bytes()).collect()
    }
}

/// A transcript stores audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Transcript {
    pub id: String,
    pub video_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub language: Option<String>,
    pub confidence: Option<f64>,
    pub speaker_id: Option<String>,
    pub word_timestamps: Option<String>, // JSON array
    pub model_id: String,
    pub created_at: String,
}

impl Transcript {
    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        self.end_ms - self.start_ms
    }
}

/// Word-level timestamp information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub confidence: Option<f64>,
}

// Input types for creating new records

/// Input for creating a new project.
#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
    pub config: Option<ProjectConfig>,
}

/// Input for creating a new library.
#[derive(Debug, Clone)]
pub struct NewLibrary {
    pub project_id: String,
    pub name: String,
    pub root_path: String,
}

/// Input for creating a new video.
#[derive(Debug, Clone)]
pub struct NewVideo {
    pub library_id: String,
    pub path: String,
    pub filename: String,
    pub duration_ms: i64,
    pub width: i32,
    pub height: i32,
    pub fps: f64,
    pub codec: String,
    pub container: Option<String>,
    pub file_size_bytes: i64,
    pub file_hash: String,
}

/// Input for creating a new segment.
#[derive(Debug, Clone)]
pub struct NewSegment {
    pub video_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub segment_type: SegmentType,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_path: Option<String>,
}

/// Input for creating a new keyframe.
#[derive(Debug, Clone)]
pub struct NewKeyframe {
    pub video_id: String,
    pub timestamp_ms: i64,
    pub frame_number: i64,
    pub image_path: String,
    pub width: i32,
    pub height: i32,
    pub extraction_method: ExtractionMethod,
    pub scene_score: Option<f64>,
}

/// Input for creating a new embedding.
#[derive(Debug, Clone)]
pub struct NewEmbedding {
    pub source_type: EmbeddingSourceType,
    pub source_id: String,
    pub model_id: String,
    pub embedding: Vec<f32>,
}

/// Input for creating a new transcript.
#[derive(Debug, Clone)]
pub struct NewTranscript {
    pub video_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub language: Option<String>,
    pub confidence: Option<f64>,
    pub speaker_id: Option<String>,
    pub word_timestamps: Option<Vec<WordTimestamp>>,
    pub model_id: String,
}
