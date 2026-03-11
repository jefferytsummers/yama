//! Tool card - individual tool configuration card.
//!
//! Displays a tool with its icon, name, description, and enable/disable toggle.

use eframe::egui::{self, Ui};
use yama_theme::{colors, font_size, radius, spacing, ConfigCard};

/// A tool configuration entry.
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// Icon (emoji or text)
    pub icon: String,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Category for grouping
    pub category: ToolCategory,
}

/// Categories for organizing tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// Video analysis tools
    Analysis,
    /// Search and retrieval tools
    Search,
    /// Content extraction tools
    Extraction,
    /// Utility tools
    Utility,
}

impl ToolCategory {
    /// Get the display label for this category.
    pub fn label(self) -> &'static str {
        match self {
            ToolCategory::Analysis => "Analysis",
            ToolCategory::Search => "Search",
            ToolCategory::Extraction => "Extraction",
            ToolCategory::Utility => "Utility",
        }
    }
}

impl ToolConfig {
    /// Create a new tool configuration.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        icon: impl Into<String>,
        category: ToolCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            icon: icon.into(),
            enabled: true,
            category,
        }
    }

    /// Create default tools for a new project.
    pub fn default_tools() -> Vec<Self> {
        vec![
            Self::new(
                "search_videos",
                "Search Videos",
                "Search through indexed video transcripts and visual content",
                "🔍",
                ToolCategory::Search,
            ),
            Self::new(
                "analyze_frame",
                "Analyze Frame",
                "Run VLM analysis on a specific video frame",
                "🎯",
                ToolCategory::Analysis,
            ),
            Self::new(
                "extract_clip",
                "Extract Clip",
                "Extract a video clip from start to end timestamp",
                "✂️",
                ToolCategory::Extraction,
            ),
            Self::new(
                "generate_summary",
                "Generate Summary",
                "Create a summary from search results or video content",
                "📝",
                ToolCategory::Analysis,
            ),
            Self::new(
                "compare_scenes",
                "Compare Scenes",
                "Compare two or more scenes across videos",
                "🔄",
                ToolCategory::Analysis,
            ),
            Self::new(
                "find_similar",
                "Find Similar",
                "Find visually or semantically similar moments",
                "🔗",
                ToolCategory::Search,
            ),
            Self::new(
                "export_transcript",
                "Export Transcript",
                "Export video transcript with timestamps",
                "📄",
                ToolCategory::Extraction,
            ),
            Self::new(
                "timeline_marker",
                "Timeline Marker",
                "Add markers to the video timeline",
                "📍",
                ToolCategory::Utility,
            ),
        ]
    }
}

/// Tool card component for displaying a single tool.
pub struct ToolCard<'a> {
    tool: &'a mut ToolConfig,
    compact: bool,
}

impl<'a> ToolCard<'a> {
    /// Create a new tool card.
    pub fn new(tool: &'a mut ToolConfig) -> Self {
        Self { tool, compact: false }
    }

    /// Use compact display mode.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Display the tool card.
    pub fn show(self, ui: &mut Ui) -> ToolCardResponse {
        let response = ConfigCard::new(&self.tool.name, &mut self.tool.enabled)
            .icon(&self.tool.icon)
            .description(if self.compact { "" } else { &self.tool.description })
            .show(ui);

        ToolCardResponse {
            changed: response.changed,
            enabled: response.enabled,
        }
    }
}

/// Response from displaying a tool card.
pub struct ToolCardResponse {
    /// Whether the enabled state changed.
    pub changed: bool,
    /// Current enabled state.
    pub enabled: bool,
}

/// Display a list of tool cards with category headers.
pub fn tool_list(ui: &mut Ui, tools: &mut [ToolConfig], compact: bool) -> bool {
    let mut any_changed = false;

    // Group by category
    let mut analysis = Vec::new();
    let mut search = Vec::new();
    let mut extraction = Vec::new();
    let mut utility = Vec::new();

    for (idx, tool) in tools.iter().enumerate() {
        match tool.category {
            ToolCategory::Analysis => analysis.push(idx),
            ToolCategory::Search => search.push(idx),
            ToolCategory::Extraction => extraction.push(idx),
            ToolCategory::Utility => utility.push(idx),
        }
    }

    let categories = [
        (ToolCategory::Analysis, analysis),
        (ToolCategory::Search, search),
        (ToolCategory::Extraction, extraction),
        (ToolCategory::Utility, utility),
    ];

    for (category, indices) in categories {
        if indices.is_empty() {
            continue;
        }

        // Category header
        ui.add_space(spacing::S2);
        ui.label(
            egui::RichText::new(category.label())
                .color(colors::ASH)
                .size(font_size::TINY)
                .strong(),
        );
        ui.add_space(spacing::S1);

        // Tools in this category
        for idx in indices {
            let tool = &mut tools[idx];
            let mut card = ToolCard::new(tool);
            if compact {
                card = card.compact();
            }
            let response = card.show(ui);
            if response.changed {
                any_changed = true;
            }
            ui.add_space(spacing::S1);
        }
    }

    any_changed
}
