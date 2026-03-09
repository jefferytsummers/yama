//! Project - a collection of videos with conversation history.

use std::path::PathBuf;

use super::{LibraryVideo, QueryExchange};

/// A project containing videos and conversation history.
pub struct Project {
    /// Unique identifier
    pub id: String,
    /// Project name
    pub name: String,
    /// Description (optional)
    pub description: String,
    /// Videos in this project
    pub videos: Vec<LibraryVideo>,
    /// Conversation history (queries and results)
    pub exchanges: Vec<QueryExchange>,
    /// Created timestamp
    pub created_at: std::time::SystemTime,
    /// Last modified timestamp
    pub updated_at: std::time::SystemTime,
    /// Project thumbnail (first video's thumbnail or placeholder)
    pub thumbnail: Option<eframe::egui::TextureHandle>,
}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Project")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("video_count", &self.videos.len())
            .finish()
    }
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            videos: Vec::new(),
            exchanges: Vec::new(),
            created_at: now,
            updated_at: now,
            thumbnail: None,
        }
    }

    /// Create a mock project for prototyping.
    pub fn mock(name: &str, video_count: usize, days_ago: u64) -> Self {
        let now = std::time::SystemTime::now();
        let updated = now - std::time::Duration::from_secs(days_ago * 24 * 60 * 60);

        let mut videos = Vec::new();
        for i in 0..video_count {
            videos.push(LibraryVideo::mock(
                &format!("v{}", i),
                &format!("video_{}.mp4", i),
                (15 + i * 5) as u64 * 60 * 1000,
                i % 3 != 0, // Some indexed, some not
            ));
        }

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            videos,
            exchanges: Vec::new(),
            created_at: updated - std::time::Duration::from_secs(7 * 24 * 60 * 60),
            updated_at: updated,
            thumbnail: None,
        }
    }

    /// Get video count.
    pub fn video_count(&self) -> usize {
        self.videos.len()
    }

    /// Get indexed video count.
    pub fn indexed_count(&self) -> usize {
        self.videos.iter().filter(|v| v.indexed).count()
    }

    /// Get total duration in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.videos.iter().map(|v| v.duration_ms).sum()
    }

    /// Format total duration as human-readable string.
    pub fn duration_string(&self) -> String {
        let total_ms = self.total_duration_ms();
        let total_minutes = total_ms / 60000;
        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    /// Format "updated ago" string.
    pub fn updated_ago_string(&self) -> String {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(self.updated_at).unwrap_or_default();
        let secs = duration.as_secs();

        if secs < 60 {
            "Just now".to_string()
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }

    /// Touch the updated timestamp.
    pub fn touch(&mut self) {
        self.updated_at = std::time::SystemTime::now();
    }

    /// Add a video to the project.
    pub fn add_video(&mut self, video: LibraryVideo) {
        self.videos.push(video);
        self.touch();
    }

    /// Get a video by ID.
    pub fn get_video(&self, id: &str) -> Option<&LibraryVideo> {
        self.videos.iter().find(|v| v.id == id)
    }

    /// Import videos from paths.
    pub fn import_videos(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown.mp4".to_string());

            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            let video = LibraryVideo {
                id: uuid::Uuid::new_v4().to_string(),
                path: path.clone(),
                filename,
                duration_ms: 0,
                dimensions: (1920, 1080),
                size_bytes: size,
                thumbnail: None,
                indexed: false,
                has_transcript: false,
                keyframe_count: 0,
            };

            self.videos.push(video);
        }
        self.touch();
    }
}

/// State for managing all projects.
#[derive(Debug, Default)]
pub struct ProjectManager {
    /// All projects
    pub projects: Vec<Project>,
    /// Currently open project ID (None = dashboard view)
    pub current_project_id: Option<String>,
}

impl ProjectManager {
    /// Create a new project manager with mock data.
    pub fn new_with_mocks() -> Self {
        let projects = vec![
            Project::mock("Q4 Planning Review", 12, 0),
            Project::mock("Training Videos", 8, 1),
            Project::mock("Customer Feedback Analysis", 24, 3),
            Project::mock("Product Demo Collection", 6, 7),
        ];

        Self {
            projects,
            current_project_id: None,
        }
    }

    /// Create a new empty project and return its ID.
    pub fn create_project(&mut self, name: impl Into<String>) -> String {
        let project = Project::new(name);
        let id = project.id.clone();
        self.projects.insert(0, project); // Add to front
        id
    }

    /// Get a project by ID.
    pub fn get_project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// Get a mutable project by ID.
    pub fn get_project_mut(&mut self, id: &str) -> Option<&mut Project> {
        self.projects.iter_mut().find(|p| p.id == id)
    }

    /// Get the current project (if one is open).
    pub fn current_project(&self) -> Option<&Project> {
        self.current_project_id.as_ref().and_then(|id| self.get_project(id))
    }

    /// Get the current project mutably.
    pub fn current_project_mut(&mut self) -> Option<&mut Project> {
        if let Some(id) = self.current_project_id.clone() {
            self.get_project_mut(&id)
        } else {
            None
        }
    }

    /// Open a project by ID.
    pub fn open_project(&mut self, id: &str) {
        if self.projects.iter().any(|p| p.id == id) {
            self.current_project_id = Some(id.to_string());
        }
    }

    /// Close the current project (return to dashboard).
    pub fn close_project(&mut self) {
        self.current_project_id = None;
    }

    /// Delete a project by ID.
    pub fn delete_project(&mut self, id: &str) {
        self.projects.retain(|p| p.id != id);
        if self.current_project_id.as_deref() == Some(id) {
            self.current_project_id = None;
        }
    }

    /// Check if we're on the dashboard (no project open).
    pub fn is_dashboard(&self) -> bool {
        self.current_project_id.is_none()
    }
}
