//! Synthetic video source for testing.
//!
//! Generates test frames without requiring actual video files.

use std::time::Duration;

/// A synthetic test frame.
#[derive(Debug, Clone)]
pub struct TestFrame {
    /// Frame number (0-indexed).
    pub number: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Frame width.
    pub width: u32,
    /// Frame height.
    pub height: u32,
    /// Raw pixel data (RGB format).
    pub data: Vec<u8>,
}

impl TestFrame {
    /// Create a solid color frame.
    pub fn solid_color(number: u64, width: u32, height: u32, r: u8, g: u8, b: u8) -> Self {
        let pixel_count = (width * height) as usize;
        let mut data = Vec::with_capacity(pixel_count * 3);
        for _ in 0..pixel_count {
            data.push(r);
            data.push(g);
            data.push(b);
        }
        Self {
            number,
            timestamp_ms: number * 1000 / 30, // 30fps
            width,
            height,
            data,
        }
    }

    /// Create a gradient frame.
    pub fn gradient(number: u64, width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        let mut data = Vec::with_capacity(pixel_count * 3);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = (((number % 256) as f32 / 255.0) * 128.0) as u8;
                data.push(r);
                data.push(g);
                data.push(b);
            }
        }
        Self {
            number,
            timestamp_ms: number * 1000 / 30,
            width,
            height,
            data,
        }
    }

    /// Create a minimal placeholder frame (1x1 pixel).
    pub fn placeholder(number: u64) -> Self {
        Self::solid_color(number, 1, 1, 128, 128, 128)
    }
}

/// Configuration for test video generation.
#[derive(Debug, Clone)]
pub struct TestVideoConfig {
    /// Video width.
    pub width: u32,
    /// Video height.
    pub height: u32,
    /// Frame rate (fps).
    pub fps: u32,
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Frame generation pattern.
    pub pattern: FramePattern,
}

/// Frame generation pattern.
#[derive(Debug, Clone, Copy, Default)]
pub enum FramePattern {
    /// Solid black frames.
    #[default]
    Black,
    /// Solid white frames.
    White,
    /// Color gradient that changes over time.
    Gradient,
    /// Alternating black and white frames.
    Alternating,
    /// Solid color with specified RGB.
    SolidColor(u8, u8, u8),
}

impl Default for TestVideoConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            fps: 30,
            duration_secs: 10,
            pattern: FramePattern::Black,
        }
    }
}

/// Synthetic video source for testing.
pub struct TestVideoSource {
    config: TestVideoConfig,
    current_frame: u64,
    total_frames: u64,
}

impl TestVideoSource {
    /// Create a new test video source.
    pub fn new(config: TestVideoConfig) -> Self {
        let total_frames = (config.fps * config.duration_secs) as u64;
        Self {
            config,
            current_frame: 0,
            total_frames,
        }
    }

    /// Create a default test video source (10 seconds, 30fps, 640x480).
    pub fn default_video() -> Self {
        Self::new(TestVideoConfig::default())
    }

    /// Get the total number of frames.
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Get the video duration.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.config.duration_secs as u64)
    }

    /// Generate the next frame.
    pub fn next_frame(&mut self) -> Option<TestFrame> {
        if self.current_frame >= self.total_frames {
            return None;
        }

        let frame = self.generate_frame(self.current_frame);
        self.current_frame += 1;
        Some(frame)
    }

    /// Generate a specific frame.
    pub fn generate_frame(&self, number: u64) -> TestFrame {
        let (r, g, b) = match self.config.pattern {
            FramePattern::Black => (0, 0, 0),
            FramePattern::White => (255, 255, 255),
            FramePattern::Gradient => {
                // Generate gradient frame
                return TestFrame::gradient(number, self.config.width, self.config.height);
            }
            FramePattern::Alternating => {
                if number % 2 == 0 {
                    (0, 0, 0)
                } else {
                    (255, 255, 255)
                }
            }
            FramePattern::SolidColor(r, g, b) => (r, g, b),
        };

        TestFrame::solid_color(number, self.config.width, self.config.height, r, g, b)
    }

    /// Generate all frames at once.
    pub fn all_frames(&self) -> Vec<TestFrame> {
        (0..self.total_frames)
            .map(|n| self.generate_frame(n))
            .collect()
    }

    /// Generate frames with a specific interval (for sampling).
    pub fn sampled_frames(&self, interval_ms: u64) -> Vec<TestFrame> {
        let frame_duration_ms = 1000 / self.config.fps as u64;
        let frame_interval = interval_ms / frame_duration_ms;
        let frame_interval = frame_interval.max(1);

        (0..self.total_frames)
            .step_by(frame_interval as usize)
            .map(|n| self.generate_frame(n))
            .collect()
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.current_frame = 0;
    }
}

impl Iterator for TestVideoSource {
    type Item = TestFrame;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_color_frame() {
        let frame = TestFrame::solid_color(0, 2, 2, 255, 0, 0);
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 2);
        assert_eq!(frame.data.len(), 12); // 2*2*3
        assert_eq!(&frame.data[0..3], &[255, 0, 0]); // First pixel is red
    }

    #[test]
    fn test_video_source_iteration() {
        let config = TestVideoConfig {
            width: 10,
            height: 10,
            fps: 10,
            duration_secs: 1,
            pattern: FramePattern::Black,
        };
        let source = TestVideoSource::new(config);
        let frames: Vec<_> = source.collect();
        assert_eq!(frames.len(), 10);
    }

    #[test]
    fn test_sampled_frames() {
        let config = TestVideoConfig {
            fps: 30,
            duration_secs: 1,
            ..Default::default()
        };
        let source = TestVideoSource::new(config);
        let sampled = source.sampled_frames(100); // Every 100ms = ~3 frames at 30fps
        assert!(sampled.len() < 30);
        assert!(sampled.len() >= 10);
    }
}
