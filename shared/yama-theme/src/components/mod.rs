//! Reusable UI components for the Yama design system.
//!
//! These components provide consistent styling and behavior across all Yama native UIs.
//! Each component is designed to match its web counterpart in the SvelteKit frontend.

mod badge;
mod button;
mod card;
mod progress;
mod status;

pub use badge::{badge, Badge, BadgeVariant};
pub use button::{danger_button, primary_button, secondary_button};
pub use card::Card;
pub use progress::{progress_bar, ProgressBar};
pub use status::{status_indicator, Status, StatusIndicator};
