//! Video frame handling for zero-copy transfer.
//!
//! This module provides utilities for sending and receiving video frames
//! between containers using DMA-BUF (where available) or shared memory.

use std::os::unix::io::RawFd;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, error, instrument, warn};

use yama_protocol::common::{PixelFormat, VideoFrameMeta};

use crate::client::EventBusClient;

/// A video frame with metadata and buffer reference.
#[derive(Debug)]
pub struct VideoFrame {
    /// Frame metadata.
    pub meta: VideoFrameMeta,
    /// Frame buffer (if using shared memory).
    pub buffer: Option<Arc<[u8]>>,
    /// DMA-BUF file descriptor (if using DMA-BUF).
    pub dma_buf_fd: Option<RawFd>,
}

impl VideoFrame {
    /// Create a new frame from shared memory.
    pub fn from_buffer(meta: VideoFrameMeta, buffer: Vec<u8>) -> Self {
        Self {
            meta,
            buffer: Some(Arc::from(buffer.into_boxed_slice())),
            dma_buf_fd: None,
        }
    }

    /// Create a new frame from a DMA-BUF file descriptor.
    pub fn from_dma_buf(meta: VideoFrameMeta, fd: RawFd) -> Self {
        Self {
            meta,
            buffer: None,
            dma_buf_fd: Some(fd),
        }
    }

    /// Get the frame width.
    pub fn width(&self) -> u32 {
        self.meta.width
    }

    /// Get the frame height.
    pub fn height(&self) -> u32 {
        self.meta.height
    }

    /// Get the pixel format.
    pub fn format(&self) -> PixelFormat {
        PixelFormat::try_from(self.meta.format).unwrap_or(PixelFormat::Unspecified)
    }

    /// Get the frame number.
    pub fn frame_number(&self) -> u64 {
        self.meta.frame_number
    }

    /// Get the source ID.
    pub fn source_id(&self) -> &str {
        &self.meta.source_id
    }
}

/// Sender for video frames.
pub struct VideoFrameSender {
    /// Event bus client.
    client: Arc<EventBusClient>,
    /// Source ID.
    source_id: String,
    /// Frame counter.
    frame_counter: u64,
}

impl VideoFrameSender {
    /// Create a new video frame sender.
    pub fn new(client: Arc<EventBusClient>, source_id: impl Into<String>) -> Self {
        Self {
            client,
            source_id: source_id.into(),
            frame_counter: 0,
        }
    }

    /// Send a video frame.
    #[instrument(skip(self, buffer), fields(source_id = %self.source_id, frame = %self.frame_counter))]
    pub async fn send_frame(
        &mut self,
        width: u32,
        height: u32,
        format: PixelFormat,
        buffer: &[u8],
    ) -> Result<()> {
        self.frame_counter += 1;

        // For now, we use shared memory for frame data
        // TODO: Implement DMA-BUF support for zero-copy

        let meta = VideoFrameMeta {
            source_id: self.source_id.clone(),
            frame_number: self.frame_counter,
            pts: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                nanos: 0,
            }),
            width,
            height,
            format: format.into(),
            dma_buf_fd: -1, // Not using DMA-BUF
            shm_name: String::new(),
            shm_offset: 0,
            shm_size: buffer.len() as u64,
        };

        debug!("Sending frame {} ({}x{})", self.frame_counter, width, height);

        // Send metadata via event bus
        // In a real implementation, the frame data would be sent via DMA-BUF or shared memory
        self.client
            .publish(&format!("video.frame.{}", self.source_id), meta)
            .await
            .context("Failed to publish frame metadata")?;

        Ok(())
    }

    /// Send a frame with DMA-BUF reference.
    #[instrument(skip(self), fields(source_id = %self.source_id, frame = %self.frame_counter))]
    pub async fn send_dma_buf_frame(
        &mut self,
        width: u32,
        height: u32,
        format: PixelFormat,
        fd: RawFd,
    ) -> Result<()> {
        self.frame_counter += 1;

        let meta = VideoFrameMeta {
            source_id: self.source_id.clone(),
            frame_number: self.frame_counter,
            pts: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                nanos: 0,
            }),
            width,
            height,
            format: format.into(),
            dma_buf_fd: fd,
            shm_name: String::new(),
            shm_offset: 0,
            shm_size: 0,
        };

        debug!(
            "Sending DMA-BUF frame {} ({}x{}, fd={})",
            self.frame_counter, width, height, fd
        );

        self.client
            .publish(&format!("video.frame.{}", self.source_id), meta)
            .await
            .context("Failed to publish frame metadata")?;

        Ok(())
    }

    /// Get the current frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_counter
    }
}

/// Receiver for video frames.
pub struct VideoFrameReceiver {
    /// Source ID filter (None = all sources).
    source_filter: Option<String>,
    /// Frame channel.
    rx: mpsc::Receiver<VideoFrame>,
}

impl VideoFrameReceiver {
    /// Create a new video frame receiver.
    pub async fn new(client: &mut EventBusClient, source_id: Option<&str>) -> Result<Self> {
        let (tx, rx) = mpsc::channel(10);

        // Subscribe to video frame events
        let topic = match source_id {
            Some(id) => format!("video.frame.{}", id),
            None => "video.frame.*".to_string(),
        };

        if source_id.is_some() {
            client.subscribe(&[&topic]).await?;
        } else {
            client.subscribe_patterns(&[&topic]).await?;
        }

        // Note: In a real implementation, we would spawn a task to receive
        // frames and decode them. For now, this is a placeholder.
        let _ = tx; // Silence unused warning

        Ok(Self {
            source_filter: source_id.map(String::from),
            rx,
        })
    }

    /// Receive the next frame.
    pub async fn recv(&mut self) -> Option<VideoFrame> {
        self.rx.recv().await
    }

    /// Get the source filter.
    pub fn source_filter(&self) -> Option<&str> {
        self.source_filter.as_deref()
    }
}

/// Shared memory segment for video frames.
#[cfg(target_os = "linux")]
pub mod shm {
    use std::ffi::CString;
    use std::os::unix::io::RawFd;

    use anyhow::{Context, Result};

    /// Create a shared memory segment.
    pub fn create(name: &str, size: usize) -> Result<RawFd> {
        let name = CString::new(name).context("Invalid shm name")?;

        unsafe {
            let fd = libc::shm_open(
                name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR,
                0o600,
            );
            if fd < 0 {
                return Err(std::io::Error::last_os_error()).context("shm_open failed");
            }

            if libc::ftruncate(fd, size as libc::off_t) < 0 {
                libc::close(fd);
                return Err(std::io::Error::last_os_error()).context("ftruncate failed");
            }

            Ok(fd)
        }
    }

    /// Open an existing shared memory segment.
    pub fn open(name: &str) -> Result<RawFd> {
        let name = CString::new(name).context("Invalid shm name")?;

        unsafe {
            let fd = libc::shm_open(name.as_ptr(), libc::O_RDONLY, 0);
            if fd < 0 {
                return Err(std::io::Error::last_os_error()).context("shm_open failed");
            }
            Ok(fd)
        }
    }

    /// Unlink a shared memory segment.
    pub fn unlink(name: &str) -> Result<()> {
        let name = CString::new(name).context("Invalid shm name")?;

        unsafe {
            if libc::shm_unlink(name.as_ptr()) < 0 {
                return Err(std::io::Error::last_os_error()).context("shm_unlink failed");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_frame() {
        let meta = VideoFrameMeta {
            source_id: "test".to_string(),
            frame_number: 1,
            pts: None,
            width: 1920,
            height: 1080,
            format: PixelFormat::Nv12.into(),
            dma_buf_fd: -1,
            shm_name: String::new(),
            shm_offset: 0,
            shm_size: 0,
        };

        let frame = VideoFrame::from_buffer(meta, vec![0u8; 100]);
        assert_eq!(frame.width(), 1920);
        assert_eq!(frame.height(), 1080);
        assert_eq!(frame.frame_number(), 1);
        assert_eq!(frame.source_id(), "test");
    }
}
