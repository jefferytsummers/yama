//! Content Analyst UI - batch video library management and search.
//!
//! This module provides the Content Analyst workflow interface:
//! - Library view with video grid
//! - Import progress overlay
//! - Search results with thumbnails
//! - Video preview panel
//! - Clip extraction dialog
//! - Project configuration panel

pub mod config_panel;
pub mod dashboard;
pub mod library;
pub mod library_modal;
pub mod project;
pub mod query_panel;
pub mod search;
pub mod tool_card;

pub use config_panel::{
    ConfigPanel, ConfigPanelAction, ConfigPanelState, ModelConfig, ModelType, ProjectConfig,
    WorkflowConfig,
};
pub use dashboard::{Dashboard, DashboardAction, DashboardState};
pub use library::{LibraryAction, LibraryView};
pub use library_modal::{LibraryModal, LibraryModalAction};
pub use project::{Project, ProjectManager};
pub use query_panel::{QueryAction, QueryExchange, QueryPanel, QueryPanelState, ToolCall, ToolResult};
pub use search::{SearchAction, SearchBar, SearchResults};
pub use tool_card::{ToolCard, ToolCardResponse, ToolCategory, ToolConfig};

use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui;

/// A video in the library.
#[derive(Clone)]
pub struct LibraryVideo {
    /// Unique identifier
    pub id: String,
    /// File path
    pub path: PathBuf,
    /// Display filename
    pub filename: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Video dimensions
    pub dimensions: (u32, u32),
    /// File size in bytes
    pub size_bytes: u64,
    /// Thumbnail texture
    pub thumbnail: Option<egui::TextureHandle>,
    /// Whether the video has been indexed
    pub indexed: bool,
    /// Whether the video has a transcript
    pub has_transcript: bool,
    /// Number of keyframes extracted
    pub keyframe_count: u32,
}

impl std::fmt::Debug for LibraryVideo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryVideo")
            .field("id", &self.id)
            .field("filename", &self.filename)
            .field("duration_ms", &self.duration_ms)
            .field("indexed", &self.indexed)
            .finish()
    }
}

impl LibraryVideo {
    /// Create a mock video for prototyping.
    pub fn mock(id: &str, filename: &str, duration_ms: u64, indexed: bool) -> Self {
        Self {
            id: id.to_string(),
            path: PathBuf::from(format!("~/Videos/{}", filename)),
            filename: filename.to_string(),
            duration_ms,
            dimensions: (1920, 1080),
            size_bytes: duration_ms * 1000, // Rough estimate
            thumbnail: None,
            indexed,
            has_transcript: indexed,
            keyframe_count: if indexed { (duration_ms / 5000) as u32 } else { 0 },
        }
    }

    /// Format duration as MM:SS or HH:MM:SS.
    pub fn duration_string(&self) -> String {
        let total_seconds = self.duration_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Format file size as human-readable string.
    pub fn size_string(&self) -> String {
        let kb = self.size_bytes as f64 / 1024.0;
        let mb = kb / 1024.0;
        let gb = mb / 1024.0;

        if gb >= 1.0 {
            format!("{:.1} GB", gb)
        } else if mb >= 1.0 {
            format!("{:.1} MB", mb)
        } else {
            format!("{:.0} KB", kb)
        }
    }
}

/// Search result from the library.
#[derive(Clone)]
pub struct SearchResult {
    /// Video ID
    pub video_id: String,
    /// Video filename
    pub video_filename: String,
    /// Video path
    pub video_path: PathBuf,
    /// Match timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Confidence score (0.0 - 1.0)
    pub score: f32,
    /// Match type
    pub match_type: MatchType,
    /// Snippet or description of the match
    pub snippet: String,
    /// Thumbnail for this specific moment
    pub thumbnail: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchResult")
            .field("video_id", &self.video_id)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("score", &self.score)
            .finish()
    }
}

impl SearchResult {
    /// Create a mock search result for prototyping.
    pub fn mock(video_id: &str, filename: &str, timestamp_ms: u64, score: f32, match_type: MatchType, snippet: &str) -> Self {
        Self {
            video_id: video_id.to_string(),
            video_filename: filename.to_string(),
            video_path: PathBuf::from(format!("~/Videos/{}", filename)),
            timestamp_ms,
            score,
            match_type,
            snippet: snippet.to_string(),
            thumbnail: None,
        }
    }

    /// Format timestamp as MM:SS.
    pub fn timestamp_string(&self) -> String {
        let total_seconds = self.timestamp_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Type of match in a search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Matched in transcript text
    Transcript,
    /// Matched visually (CLIP embedding)
    Visual,
    /// Matched both transcript and visual
    Both,
}

impl MatchType {
    /// Get display label for the match type.
    pub fn label(&self) -> &'static str {
        match self {
            MatchType::Transcript => "Transcript",
            MatchType::Visual => "Visual",
            MatchType::Both => "Both",
        }
    }
}

/// Indexing progress state.
#[derive(Debug, Clone)]
pub struct IndexingProgress {
    /// Current stage
    pub stage: IndexingStage,
    /// Total videos to process
    pub total_videos: u32,
    /// Videos processed so far
    pub processed_videos: u32,
    /// Current file being processed
    pub current_file: Option<String>,
    /// Errors encountered
    pub errors: Vec<(String, String)>,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u64>,
    /// Whether indexing is paused
    pub paused: bool,
}

impl IndexingProgress {
    /// Create a new indexing progress tracker.
    pub fn new(total_videos: u32) -> Self {
        Self {
            stage: IndexingStage::Scanning,
            total_videos,
            processed_videos: 0,
            current_file: None,
            errors: Vec::new(),
            eta_seconds: None,
            paused: false,
        }
    }

    /// Get overall progress as a percentage.
    pub fn percent(&self) -> f32 {
        if self.total_videos == 0 {
            return 0.0;
        }
        (self.processed_videos as f32 / self.total_videos as f32) * 100.0
    }

    /// Format ETA as human-readable string.
    pub fn eta_string(&self) -> String {
        match self.eta_seconds {
            Some(secs) if secs > 3600 => format!("~{:.0}h {}m", secs / 3600, (secs % 3600) / 60),
            Some(secs) if secs > 60 => format!("~{}m", secs / 60),
            Some(secs) => format!("~{}s", secs),
            None => "Calculating...".to_string(),
        }
    }
}

/// Stage of the indexing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexingStage {
    /// Scanning files
    Scanning,
    /// Extracting keyframes
    ExtractingKeyframes,
    /// Generating CLIP embeddings
    GeneratingEmbeddings,
    /// Transcribing audio
    Transcribing,
    /// Indexing complete
    Complete,
}

impl IndexingStage {
    /// Get stage index (0-4).
    pub fn index(&self) -> usize {
        match self {
            IndexingStage::Scanning => 0,
            IndexingStage::ExtractingKeyframes => 1,
            IndexingStage::GeneratingEmbeddings => 2,
            IndexingStage::Transcribing => 3,
            IndexingStage::Complete => 4,
        }
    }

    /// Get stage label.
    pub fn label(&self) -> &'static str {
        match self {
            IndexingStage::Scanning => "Scanning files",
            IndexingStage::ExtractingKeyframes => "Extracting keyframes",
            IndexingStage::GeneratingEmbeddings => "Generating embeddings",
            IndexingStage::Transcribing => "Transcribing audio",
            IndexingStage::Complete => "Complete",
        }
    }

    /// Get number of total stages (excluding Complete).
    pub fn total() -> usize {
        4
    }
}

/// State for the Content Analyst view.
#[derive(Debug)]
pub struct AnalystState {
    /// All videos in the library
    pub videos: Vec<LibraryVideo>,
    /// Currently selected video IDs
    pub selected_videos: HashSet<String>,
    /// Current search query
    pub search_query: String,
    /// Search results (None if no search, empty if no results)
    pub search_results: Option<Vec<SearchResult>>,
    /// Search mode
    pub search_mode: SearchMode,
    /// Currently previewing video (if any)
    pub preview_video_id: Option<String>,
    /// Preview timestamp (for search results)
    pub preview_timestamp_ms: Option<u64>,
    /// Indexing progress (None if not indexing)
    pub indexing_progress: Option<IndexingProgress>,
    /// Show model settings
    pub show_model_settings: bool,
}

impl Default for AnalystState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalystState {
    /// Create a new analyst state with mock data.
    pub fn new() -> Self {
        // Create mock videos for prototyping
        let videos = vec![
            LibraryVideo::mock("v1", "meeting_q4_planning.mp4", 45 * 60 * 1000, true),
            LibraryVideo::mock("v2", "earnings_call_2024.mp4", 62 * 60 * 1000, true),
            LibraryVideo::mock("v3", "product_demo_v2.mp4", 15 * 60 * 1000, true),
            LibraryVideo::mock("v4", "interview_ceo.mp4", 35 * 60 * 1000, true),
            LibraryVideo::mock("v5", "team_standup_weekly.mp4", 28 * 60 * 1000, true),
            LibraryVideo::mock("v6", "board_meeting_jan.mp4", 90 * 60 * 1000, true),
            LibraryVideo::mock("v7", "training_session.mp4", 55 * 60 * 1000, false),
            LibraryVideo::mock("v8", "conference_keynote.mp4", 48 * 60 * 1000, false),
            LibraryVideo::mock("v9", "webinar_marketing.mp4", 72 * 60 * 1000, true),
            LibraryVideo::mock("v10", "customer_feedback.mp4", 22 * 60 * 1000, true),
            LibraryVideo::mock("v11", "internal_review.mp4", 40 * 60 * 1000, true),
            LibraryVideo::mock("v12", "sales_pitch_deck.mp4", 18 * 60 * 1000, true),
        ];

        Self {
            videos,
            selected_videos: HashSet::new(),
            search_query: String::new(),
            search_results: None,
            search_mode: SearchMode::Both,
            preview_video_id: None,
            preview_timestamp_ms: None,
            indexing_progress: None,
            show_model_settings: false,
        }
    }

    /// Get a video by ID.
    pub fn get_video(&self, id: &str) -> Option<&LibraryVideo> {
        self.videos.iter().find(|v| v.id == id)
    }

    /// Execute a mock search.
    pub fn execute_search(&mut self) {
        if self.search_query.trim().is_empty() {
            self.search_results = None;
            return;
        }

        // Mock search results
        let query = self.search_query.to_lowercase();
        let mut results = Vec::new();

        if query.contains("quarterly") || query.contains("q4") || query.contains("results") {
            results.push(SearchResult::mock(
                "v1", "meeting_q4_planning.mp4",
                12 * 60 * 1000 + 34 * 1000, 0.92, MatchType::Both,
                "...quarterly results exceeded our projections by 15%..."
            ));
            results.push(SearchResult::mock(
                "v2", "earnings_call_2024.mp4",
                45 * 60 * 1000 + 12 * 1000, 0.87, MatchType::Transcript,
                "...results for Q4 show significant growth..."
            ));
            results.push(SearchResult::mock(
                "v6", "board_meeting_jan.mp4",
                3 * 60 * 1000 + 22 * 1000, 0.81, MatchType::Visual,
                "(chart visible showing quarterly metrics)"
            ));
        }

        if query.contains("chart") || query.contains("graph") {
            results.push(SearchResult::mock(
                "v3", "product_demo_v2.mp4",
                8 * 60 * 1000, 0.78, MatchType::Visual,
                "(product growth chart visible)"
            ));
        }

        if query.contains("meeting") || query.contains("team") {
            results.push(SearchResult::mock(
                "v5", "team_standup_weekly.mp4",
                5 * 60 * 1000, 0.75, MatchType::Both,
                "...let's go around the room and share updates..."
            ));
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        self.search_results = Some(results);
    }

    /// Start mock indexing.
    pub fn start_indexing(&mut self, video_count: u32) {
        self.indexing_progress = Some(IndexingProgress::new(video_count));
    }

    /// Toggle video selection.
    pub fn toggle_selection(&mut self, video_id: &str) {
        if self.selected_videos.contains(video_id) {
            self.selected_videos.remove(video_id);
        } else {
            self.selected_videos.insert(video_id.to_string());
        }
    }

    /// Select all videos.
    pub fn select_all(&mut self) {
        for video in &self.videos {
            self.selected_videos.insert(video.id.clone());
        }
    }

    /// Clear selection.
    pub fn clear_selection(&mut self) {
        self.selected_videos.clear();
    }
}

/// Search mode for filtering results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Search transcript text only
    Transcript,
    /// Search visual content only
    Visual,
    /// Search both transcript and visual
    Both,
}

impl SearchMode {
    /// Get all search modes.
    pub fn all() -> &'static [SearchMode] {
        &[SearchMode::Transcript, SearchMode::Visual, SearchMode::Both]
    }

    /// Get display label.
    pub fn label(&self) -> &'static str {
        match self {
            SearchMode::Transcript => "Transcript",
            SearchMode::Visual => "Visual",
            SearchMode::Both => "Both",
        }
    }
}
