//! Reusable UI components for the Yama design system.
//!
//! These components provide consistent styling and behavior across all Yama native UIs.
//! Each component is designed to match its web counterpart in the SvelteKit frontend.

mod attachment_chip;
mod avatar;
mod badge;
mod button;
mod card;
mod chat_bubble;
mod progress;
mod status;

pub use attachment_chip::{attachment_chip, AttachmentChip, AttachmentStatus, FileType};
pub use avatar::{avatar, avatar_pulsing, Avatar, AvatarType};
pub use badge::{badge, Badge, BadgeVariant};
pub use button::{danger_button, primary_button, secondary_button};
pub use card::Card;
pub use chat_bubble::{chat_bubble, ChatBubble, ChatBubbleStyle, MessageRole};
pub use progress::{progress_bar, ProgressBar};
pub use status::{status_indicator, Status, StatusIndicator};
