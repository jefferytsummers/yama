//! Yama Platform Traits
//!
//! This crate defines the core abstractions that platform-specific implementations
//! must provide. By coding against these traits, the rest of the system remains
//! platform-agnostic while allowing optimized implementations for each target.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Application Layer                        │
//! │         (Compositor, Agent Runtime, Services)               │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Platform Traits                           │
//! │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
//! │  │ Renderer │ │ Decoder  │ │Allocator │ │ Display      │   │
//! │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!              ┌───────────────┼───────────────┐
//!              ▼               ▼               ▼
//! ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
//! │  Apple Silicon  │ │  NVIDIA Jetson  │ │   Generic/SW    │
//! │  (Metal, VT)    │ │  (CUDA, NVDEC)  │ │   (Fallback)    │
//! └─────────────────┘ └─────────────────┘ └─────────────────┘
//! ```
//!
//! # Traits
//!
//! - [`Renderer`] - GPU rendering abstraction (Metal, Vulkan, EGL)
//! - [`VideoDecoder`] - Hardware video decoding (VideoToolbox, NVDEC)
//! - [`GpuAllocator`] - GPU memory management and DMA-BUF sharing
//! - [`DisplayBackend`] - Display/output management (DRM/KMS)
//! - [`InferenceEngine`] - ML inference abstraction (Metal, CUDA, CPU)
//!
//! # Platform Detection
//!
//! Use [`PlatformInfo`] and [`detect_platform`] for runtime platform detection
//! and capability querying.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod allocator;
pub mod decoder;
pub mod display;
pub mod error;
pub mod factory;
pub mod inference;
pub mod platform;
pub mod renderer;
pub mod types;

// Re-export main traits at crate root
pub use allocator::{GpuAllocator, GpuBuffer, BufferUsage};
pub use decoder::{VideoDecoder, DecodedFrame, DecoderCapabilities, CodecType};
pub use display::{DisplayBackend, OutputInfo, OutputId, DisplayMode};
pub use error::{PlatformError, PlatformResult};
pub use inference::{InferenceEngine, InferenceRequest, InferenceResponse, ModelInfo};
pub use platform::{Platform, PlatformInfo, PlatformCapabilities, detect_platform};
pub use renderer::{Renderer, RenderSurface, TextureHandle, TextureFormat};
pub use factory::{PlatformFactory, FactoryConfig, detect_and_log_platform};
pub use types::*;
