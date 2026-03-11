//! Error types for platform operations.

use thiserror::Error;

/// Result type for platform operations.
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Errors that can occur in platform operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// Platform not supported.
    #[error("Platform not supported: {0}")]
    Unsupported(String),

    /// Feature not available on this platform.
    #[error("Feature not available: {0}")]
    FeatureNotAvailable(String),

    /// Initialization failed.
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Resource allocation failed.
    #[error("Allocation failed: {0}")]
    AllocationFailed(String),

    /// Buffer operation failed.
    #[error("Buffer error: {0}")]
    BufferError(String),

    /// Texture operation failed.
    #[error("Texture error: {0}")]
    TextureError(String),

    /// Rendering failed.
    #[error("Render error: {0}")]
    RenderError(String),

    /// Video decoding failed.
    #[error("Decode error: {0}")]
    DecodeError(String),

    /// Codec not supported.
    #[error("Codec not supported: {0}")]
    CodecNotSupported(String),

    /// Display/output error.
    #[error("Display error: {0}")]
    DisplayError(String),

    /// Mode setting failed.
    #[error("Mode setting failed: {0}")]
    ModeSetFailed(String),

    /// Inference error.
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Model loading failed.
    #[error("Model loading failed: {0}")]
    ModelLoadFailed(String),

    /// DMA-BUF operation failed.
    #[error("DMA-BUF error: {0}")]
    DmaBufError(String),

    /// Import/export failed.
    #[error("Import/export failed: {0}")]
    ImportExportFailed(String),

    /// Invalid parameter.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Resource not found.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Operation timed out.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Operation was cancelled.
    #[error("Operation cancelled")]
    Cancelled,

    /// Device lost or disconnected.
    #[error("Device lost: {0}")]
    DeviceLost(String),

    /// Out of memory.
    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl PlatformError {
    /// Create an unsupported error.
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    /// Create a feature not available error.
    pub fn feature_not_available(msg: impl Into<String>) -> Self {
        Self::FeatureNotAvailable(msg.into())
    }

    /// Create an initialization failed error.
    pub fn init_failed(msg: impl Into<String>) -> Self {
        Self::InitializationFailed(msg.into())
    }

    /// Create an allocation failed error.
    pub fn allocation_failed(msg: impl Into<String>) -> Self {
        Self::AllocationFailed(msg.into())
    }

    /// Create a decode error.
    pub fn decode_error(msg: impl Into<String>) -> Self {
        Self::DecodeError(msg.into())
    }

    /// Create an inference error.
    pub fn inference_error(msg: impl Into<String>) -> Self {
        Self::InferenceError(msg.into())
    }

    /// Check if this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_) | Self::DecodeError(_) | Self::BufferError(_) | Self::Cancelled
        )
    }

    /// Check if this error is a cancellation.
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Check if this error indicates the device is lost.
    #[must_use]
    pub const fn is_device_lost(&self) -> bool {
        matches!(self, Self::DeviceLost(_))
    }
}

impl From<anyhow::Error> for PlatformError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
