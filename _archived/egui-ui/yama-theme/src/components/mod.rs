//! Reusable UI components for the Yama design system.
//!
//! These components provide consistent styling and behavior across all Yama native UIs.

mod alert_timeline;
mod attachment_chip;
mod avatar;
mod badge;
mod button;
mod card;
mod chat_bubble;
mod config_card;
mod detection_overlay;
mod progress;
mod status;
mod toggle_switch;
mod video_grid;

pub use alert_timeline::{alert_timeline, AlertTimeline, EventSeverity, TimelineAction, TimelineEvent};
pub use attachment_chip::{attachment_chip, AttachmentChip, AttachmentStatus, FileType};
pub use avatar::{avatar, avatar_pulsing, Avatar, AvatarType};
pub use badge::{badge, Badge, BadgeVariant};
pub use button::{danger_button, primary_button, secondary_button};
pub use card::Card;
pub use chat_bubble::{chat_bubble, ChatBubble, ChatBubbleStyle, MessageRole};
pub use detection_overlay::{
    detection_overlay, Detection, DetectionClass, DetectionOverlay, DetectionOverlayAction, TrailPoint,
};
pub use progress::{progress_bar, ProgressBar};
pub use status::{status_indicator, Status, StatusIndicator};
pub use config_card::{config_card, ConfigCard, ConfigCardResponse};
pub use toggle_switch::{toggle_switch, toggle_switch_with_label, ToggleSwitch, ToggleSwitchSize};
pub use video_grid::{
    video_grid, GridLayout, StreamStatus, VideoGrid, VideoGridAction, VideoSourceInfo,
};
