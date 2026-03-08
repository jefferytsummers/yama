//! Chat-centric UI for Yama video analysis.
//!
//! This module provides a conversational interface for video inference,
//! allowing users to drop videos into chat and provide prompts for analysis.

pub mod app;
pub mod attachments;
pub mod chat;

pub use app::ChatApp;

use std::path::PathBuf;

use eframe::egui;

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// User-sent message
    User,
    /// Assistant response
    Assistant,
    /// System notification
    System,
}

impl From<MessageRole> for yama_theme::components::MessageRole {
    fn from(role: MessageRole) -> Self {
        match role {
            MessageRole::User => yama_theme::components::MessageRole::User,
            MessageRole::Assistant => yama_theme::components::MessageRole::Assistant,
            MessageRole::System => yama_theme::components::MessageRole::System,
        }
    }
}

/// Status of a video attachment in the processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttachmentStatus {
    /// Attachment added but not yet started
    #[default]
    Pending,
    /// Ready for processing
    Ready,
    /// Currently being processed
    Processing,
    /// Processing completed
    Completed,
    /// Processing failed
    Failed,
}

impl From<AttachmentStatus> for yama_theme::components::AttachmentStatus {
    fn from(status: AttachmentStatus) -> Self {
        match status {
            AttachmentStatus::Pending => yama_theme::components::AttachmentStatus::Pending,
            AttachmentStatus::Ready => yama_theme::components::AttachmentStatus::Ready,
            AttachmentStatus::Processing => yama_theme::components::AttachmentStatus::Processing,
            AttachmentStatus::Completed => yama_theme::components::AttachmentStatus::Completed,
            AttachmentStatus::Failed => yama_theme::components::AttachmentStatus::Failed,
        }
    }
}

/// Video attachment data.
#[derive(Clone)]
pub struct VideoAttachmentData {
    /// Unique identifier
    pub id: String,
    /// Path to the video file
    pub path: PathBuf,
    /// Display filename
    pub filename: String,
    /// File size in bytes
    pub size: u64,
    /// Thumbnail texture (loaded asynchronously)
    pub thumbnail: Option<egui::TextureHandle>,
    /// Current processing status
    pub status: AttachmentStatus,
    /// Video duration in milliseconds (if known)
    pub duration_ms: Option<u64>,
    /// Video dimensions (if known)
    pub dimensions: Option<(u32, u32)>,
}

impl std::fmt::Debug for VideoAttachmentData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoAttachmentData")
            .field("id", &self.id)
            .field("path", &self.path)
            .field("filename", &self.filename)
            .field("size", &self.size)
            .field("thumbnail", &self.thumbnail.as_ref().map(|_| "<texture>"))
            .field("status", &self.status)
            .field("duration_ms", &self.duration_ms)
            .field("dimensions", &self.dimensions)
            .finish()
    }
}

impl VideoAttachmentData {
    /// Create a new video attachment from a file path.
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.mp4".to_string());

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            filename,
            size,
            thumbnail: None,
            status: AttachmentStatus::Pending,
            duration_ms: None,
            dimensions: None,
        }
    }

    /// Check if this is a video file based on extension.
    pub fn is_video(&self) -> bool {
        let ext = self
            .path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        matches!(ext.as_str(), "mp4" | "webm" | "mov" | "avi" | "mkv" | "m4v")
    }
}

/// Frame analysis result from the VLM.
#[derive(Debug, Clone)]
pub struct FrameResult {
    /// Frame number
    pub frame_number: u64,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Analysis text
    pub text: String,
}

/// Progress information for an inference job.
#[derive(Debug, Clone)]
pub struct InferenceProgress {
    /// Current frame being processed
    pub current_frame: u64,
    /// Total frames to process
    pub total_frames: u64,
    /// Progress percentage (0-100)
    pub percent: f32,
}

impl InferenceProgress {
    /// Create a new progress indicator.
    pub fn new(current: u64, total: u64) -> Self {
        let percent = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        Self {
            current_frame: current,
            total_frames: total,
            percent,
        }
    }
}

/// A chat message in the conversation.
#[derive(Debug, Clone)]
pub struct ChatMessageData {
    /// Unique identifier
    pub id: String,
    /// Message role (user/assistant/system)
    pub role: MessageRole,
    /// Text content
    pub content: String,
    /// Video attachments (for user messages)
    pub attachments: Vec<VideoAttachmentData>,
    /// Frame analysis results (for assistant messages)
    pub frame_results: Vec<FrameResult>,
    /// Inference progress (for in-flight assistant messages)
    pub progress: Option<InferenceProgress>,
    /// Associated job ID (for assistant messages)
    pub job_id: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl ChatMessageData {
    /// Create a new user message.
    pub fn user(content: String, attachments: Vec<VideoAttachmentData>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content,
            attachments,
            frame_results: Vec::new(),
            progress: None,
            job_id: None,
            error: None,
        }
    }

    /// Create a new assistant message (pending inference).
    pub fn assistant_pending(job_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: String::new(),
            attachments: Vec::new(),
            frame_results: Vec::new(),
            progress: Some(InferenceProgress::new(0, 0)),
            job_id: Some(job_id),
            error: None,
        }
    }

    /// Create a new system message.
    pub fn system(content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content,
            attachments: Vec::new(),
            frame_results: Vec::new(),
            progress: None,
            job_id: None,
            error: None,
        }
    }

    /// Check if this message is currently inferring.
    pub fn is_inferring(&self) -> bool {
        self.role == MessageRole::Assistant && self.progress.is_some() && self.error.is_none()
    }

    /// Check if this message has completed.
    pub fn is_completed(&self) -> bool {
        self.role == MessageRole::Assistant && self.progress.is_none() && self.error.is_none()
    }

    /// Check if this message has failed.
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

/// The conversation state.
#[derive(Debug, Default)]
pub struct Conversation {
    /// All messages in the conversation
    pub messages: Vec<ChatMessageData>,
    /// Current draft text in the input
    pub draft_text: String,
    /// Current draft attachments
    pub draft_attachments: Vec<VideoAttachmentData>,
    /// Whether we're currently dragging files over the window
    pub drag_hover: bool,
}

impl Conversation {
    /// Create a new empty conversation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, message: ChatMessageData) {
        self.messages.push(message);
    }

    /// Get a mutable reference to a message by job ID.
    pub fn get_message_by_job_mut(&mut self, job_id: &str) -> Option<&mut ChatMessageData> {
        self.messages
            .iter_mut()
            .find(|m| m.job_id.as_deref() == Some(job_id))
    }

    /// Add an attachment to the draft.
    pub fn add_draft_attachment(&mut self, attachment: VideoAttachmentData) {
        self.draft_attachments.push(attachment);
    }

    /// Remove an attachment from the draft by ID.
    pub fn remove_draft_attachment(&mut self, id: &str) {
        self.draft_attachments.retain(|a| a.id != id);
    }

    /// Clear the draft (after sending).
    pub fn clear_draft(&mut self) {
        self.draft_text.clear();
        self.draft_attachments.clear();
    }

    /// Check if we can send the current draft.
    pub fn can_send(&self) -> bool {
        !self.draft_text.trim().is_empty() || !self.draft_attachments.is_empty()
    }

    /// Get the last assistant message that's still inferring.
    pub fn active_inference(&self) -> Option<&ChatMessageData> {
        self.messages.iter().rev().find(|m| m.is_inferring())
    }

    /// Check if any inference is currently running.
    pub fn has_active_inference(&self) -> bool {
        self.messages.iter().any(|m| m.is_inferring())
    }
}
