//! Video decoder trait for hardware-accelerated decoding.
//!
//! This trait abstracts over different hardware decoders:
//! - VideoToolbox (Apple)
//! - NVDEC (NVIDIA)
//! - VAAPI (Intel, AMD on Linux)
//! - Software fallback (libavcodec)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::PlatformResult;
use crate::types::{PixelFormat, Size};
use crate::allocator::GpuBuffer;

/// Video codec type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CodecType {
    /// H.264 / AVC
    H264,
    /// H.265 / HEVC
    H265,
    /// VP8
    Vp8,
    /// VP9
    Vp9,
    /// AV1
    Av1,
    /// MJPEG
    Mjpeg,
}

impl CodecType {
    /// Get human-readable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265",
            Self::Vp8 => "VP8",
            Self::Vp9 => "VP9",
            Self::Av1 => "AV1",
            Self::Mjpeg => "MJPEG",
        }
    }
}

/// Decoder capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderCapabilities {
    /// Decoder name/identifier.
    pub name: String,
    /// Whether this is a hardware decoder.
    pub is_hardware: bool,
    /// Supported codecs.
    pub supported_codecs: Vec<CodecType>,
    /// Maximum supported resolution.
    pub max_resolution: Size,
    /// Supported output formats.
    pub output_formats: Vec<PixelFormat>,
    /// Whether zero-copy output is supported.
    pub supports_zero_copy: bool,
    /// Maximum number of concurrent decode sessions.
    pub max_sessions: u32,
}

/// Codec configuration for decoder initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecConfig {
    /// Codec type.
    pub codec: CodecType,
    /// Video width.
    pub width: u32,
    /// Video height.
    pub height: u32,
    /// Desired output format (None = decoder's choice).
    pub output_format: Option<PixelFormat>,
    /// Codec-specific extra data (SPS/PPS for H.264, etc.).
    pub extra_data: Option<Vec<u8>>,
    /// Enable low-latency mode if available.
    pub low_latency: bool,
    /// Number of reference frames (0 = auto).
    pub reference_frames: u32,
}

impl CodecConfig {
    /// Create a basic H.264 configuration.
    #[must_use]
    pub fn h264(width: u32, height: u32) -> Self {
        Self {
            codec: CodecType::H264,
            width,
            height,
            output_format: Some(PixelFormat::Nv12),
            extra_data: None,
            low_latency: true,
            reference_frames: 0,
        }
    }

    /// Create a basic H.265 configuration.
    #[must_use]
    pub fn h265(width: u32, height: u32) -> Self {
        Self {
            codec: CodecType::H265,
            width,
            height,
            output_format: Some(PixelFormat::Nv12),
            extra_data: None,
            low_latency: true,
            reference_frames: 0,
        }
    }
}

/// A decoded video frame.
#[derive(Debug)]
pub struct DecodedFrame {
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Pixel format.
    pub format: PixelFormat,
    /// Presentation timestamp (microseconds).
    pub pts: i64,
    /// Decode timestamp (microseconds).
    pub dts: i64,
    /// Frame duration (microseconds).
    pub duration: i64,
    /// Whether this is a keyframe.
    pub is_keyframe: bool,
    /// Frame data (one of the following will be set).
    pub data: FrameData,
}

/// Frame data storage.
#[derive(Debug)]
pub enum FrameData {
    /// CPU memory buffer.
    CpuBuffer {
        /// Pixel data.
        data: Vec<u8>,
        /// Row stride for each plane.
        strides: Vec<usize>,
    },
    /// GPU buffer (zero-copy).
    GpuBuffer(GpuBuffer),
    /// DMA-BUF file descriptor (zero-copy, Linux).
    #[cfg(unix)]
    DmaBuf {
        /// File descriptor.
        fd: std::os::unix::io::RawFd,
        /// Buffer offset.
        offset: u64,
        /// Row stride.
        stride: u32,
        /// Modifier (DRM format modifier).
        modifier: u64,
    },
}

impl DecodedFrame {
    /// Check if frame is in GPU memory (zero-copy).
    #[must_use]
    pub const fn is_zero_copy(&self) -> bool {
        matches!(self.data, FrameData::GpuBuffer(_) | FrameData::DmaBuf { .. })
    }

    /// Get frame size.
    #[must_use]
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

/// Input packet for decoding.
#[derive(Debug, Clone)]
pub struct DecodePacket {
    /// Compressed data.
    pub data: Vec<u8>,
    /// Presentation timestamp (microseconds).
    pub pts: i64,
    /// Decode timestamp (microseconds).
    pub dts: i64,
    /// Whether this is a keyframe.
    pub is_keyframe: bool,
}

impl DecodePacket {
    /// Create a new packet.
    #[must_use]
    pub fn new(data: Vec<u8>, pts: i64) -> Self {
        Self {
            data,
            pts,
            dts: pts,
            is_keyframe: false,
        }
    }

    /// Mark as keyframe.
    #[must_use]
    pub const fn with_keyframe(mut self, is_keyframe: bool) -> Self {
        self.is_keyframe = is_keyframe;
        self
    }
}

/// Hardware video decoder abstraction.
///
/// This trait defines the interface for platform-specific video decoders.
/// Implementations handle codec initialization, packet decoding, and frame output.
#[async_trait]
pub trait VideoDecoder: Send + Sync {
    /// Get decoder capabilities.
    fn capabilities(&self) -> &DecoderCapabilities;

    /// Initialize the decoder with codec configuration.
    fn configure(&mut self, config: CodecConfig) -> PlatformResult<()>;

    /// Decode a packet.
    ///
    /// Returns decoded frames (may be empty if frames are buffered).
    /// The decoder may buffer multiple packets before outputting frames.
    async fn decode(&mut self, packet: &DecodePacket) -> PlatformResult<Vec<DecodedFrame>>;

    /// Flush the decoder and return any buffered frames.
    async fn flush(&mut self) -> PlatformResult<Vec<DecodedFrame>>;

    /// Reset the decoder state.
    ///
    /// Call this when seeking or after errors.
    fn reset(&mut self) -> PlatformResult<()>;

    /// Check if codec is supported.
    fn is_codec_supported(&self, codec: CodecType) -> bool {
        self.capabilities().supported_codecs.contains(&codec)
    }

    /// Get current codec configuration.
    fn current_config(&self) -> Option<&CodecConfig>;

    /// Get decoder statistics.
    fn statistics(&self) -> DecoderStatistics;
}

/// Decoder statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecoderStatistics {
    /// Total frames decoded.
    pub frames_decoded: u64,
    /// Frames dropped due to errors.
    pub frames_dropped: u64,
    /// Average decode time (microseconds).
    pub avg_decode_time_us: u64,
    /// Peak decode time (microseconds).
    pub peak_decode_time_us: u64,
    /// Current decode queue depth.
    pub queue_depth: u32,
}
