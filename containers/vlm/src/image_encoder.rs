//! Image format conversion utilities.
//!
//! Converts video frame formats (NV12, BGRA, etc.) to RGB for VLM input.

use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage, Rgba, RgbaImage};

/// Pixel format of the input image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// NV12 (YUV 4:2:0 semi-planar) - common hardware decoder output.
    Nv12,
    /// I420 (YUV 4:2:0 planar).
    I420,
    /// RGB (8-bit per channel).
    Rgb,
    /// RGBA (8-bit per channel).
    Rgba,
    /// BGRA (8-bit per channel) - common on macOS/Windows.
    Bgra,
    /// BGR (8-bit per channel).
    Bgr,
}

impl PixelFormat {
    /// Parse pixel format from protocol enum value.
    pub fn from_proto(value: i32) -> Option<Self> {
        match value {
            0 => None, // Unspecified
            1 => Some(Self::Nv12),
            2 => Some(Self::I420),
            3 => Some(Self::Rgba),
            4 => Some(Self::Bgra),
            5 => Some(Self::Rgb),
            6 => Some(Self::Bgr),
            _ => None,
        }
    }

    /// Get bytes per pixel for packed formats.
    pub fn bytes_per_pixel(&self) -> Option<usize> {
        match self {
            Self::Rgb | Self::Bgr => Some(3),
            Self::Rgba | Self::Bgra => Some(4),
            Self::Nv12 | Self::I420 => None, // Planar formats
        }
    }
}

/// Image encoder for converting video frames to RGB.
pub struct ImageEncoder;

impl ImageEncoder {
    /// Convert raw image data to a `DynamicImage`.
    ///
    /// # Errors
    ///
    /// Returns an error if the format is unsupported or data is invalid.
    pub fn decode(
        data: &[u8],
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<DynamicImage> {
        match format {
            PixelFormat::Rgb => Self::from_rgb(data, width, height),
            PixelFormat::Rgba => Self::from_rgba(data, width, height),
            PixelFormat::Bgra => Self::from_bgra(data, width, height),
            PixelFormat::Bgr => Self::from_bgr(data, width, height),
            PixelFormat::Nv12 => Self::from_nv12(data, width, height),
            PixelFormat::I420 => Self::from_i420(data, width, height),
        }
    }

    /// Convert RGB data to `DynamicImage`.
    fn from_rgb(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let expected_size = (width * height * 3) as usize;
        if data.len() < expected_size {
            bail!(
                "RGB data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        let image =
            RgbImage::from_raw(width, height, data[..expected_size].to_vec())
                .context("Failed to create RGB image")?;

        Ok(DynamicImage::ImageRgb8(image))
    }

    /// Convert RGBA data to `DynamicImage`.
    fn from_rgba(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let expected_size = (width * height * 4) as usize;
        if data.len() < expected_size {
            bail!(
                "RGBA data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        let image = RgbaImage::from_raw(width, height, data[..expected_size].to_vec())
            .context("Failed to create RGBA image")?;

        Ok(DynamicImage::ImageRgba8(image))
    }

    /// Convert BGRA data to `DynamicImage`.
    fn from_bgra(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let expected_size = (width * height * 4) as usize;
        if data.len() < expected_size {
            bail!(
                "BGRA data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        // Convert BGRA to RGBA
        let mut rgba_data = Vec::with_capacity(expected_size);
        for chunk in data[..expected_size].chunks_exact(4) {
            rgba_data.push(chunk[2]); // R <- B
            rgba_data.push(chunk[1]); // G <- G
            rgba_data.push(chunk[0]); // B <- R
            rgba_data.push(chunk[3]); // A <- A
        }

        let image = RgbaImage::from_raw(width, height, rgba_data)
            .context("Failed to create RGBA image from BGRA")?;

        Ok(DynamicImage::ImageRgba8(image))
    }

    /// Convert BGR data to `DynamicImage`.
    fn from_bgr(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let expected_size = (width * height * 3) as usize;
        if data.len() < expected_size {
            bail!(
                "BGR data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        // Convert BGR to RGB
        let mut rgb_data = Vec::with_capacity(expected_size);
        for chunk in data[..expected_size].chunks_exact(3) {
            rgb_data.push(chunk[2]); // R <- B
            rgb_data.push(chunk[1]); // G <- G
            rgb_data.push(chunk[0]); // B <- R
        }

        let image = RgbImage::from_raw(width, height, rgb_data)
            .context("Failed to create RGB image from BGR")?;

        Ok(DynamicImage::ImageRgb8(image))
    }

    /// Convert NV12 (YUV 4:2:0 semi-planar) to `DynamicImage`.
    ///
    /// NV12 layout:
    /// - Y plane: width * height bytes
    /// - UV plane: width * height / 2 bytes (interleaved U and V)
    fn from_nv12(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 2;
        let expected_size = y_size + uv_size;

        if data.len() < expected_size {
            bail!(
                "NV12 data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        let y_plane = &data[..y_size];
        let uv_plane = &data[y_size..expected_size];

        let mut rgb_data = Vec::with_capacity(y_size * 3);

        for y_pos in 0..height {
            for x_pos in 0..width {
                let y_idx = (y_pos * width + x_pos) as usize;
                let uv_idx = ((y_pos / 2) * width + (x_pos & !1)) as usize;

                let y = y_plane[y_idx] as f32;
                let u = uv_plane[uv_idx] as f32 - 128.0;
                let v = uv_plane[uv_idx + 1] as f32 - 128.0;

                // BT.601 conversion
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        let image = RgbImage::from_raw(width, height, rgb_data)
            .context("Failed to create RGB image from NV12")?;

        Ok(DynamicImage::ImageRgb8(image))
    }

    /// Convert I420 (YUV 4:2:0 planar) to `DynamicImage`.
    ///
    /// I420 layout:
    /// - Y plane: width * height bytes
    /// - U plane: width/2 * height/2 bytes
    /// - V plane: width/2 * height/2 bytes
    fn from_i420(data: &[u8], width: u32, height: u32) -> Result<DynamicImage> {
        let y_size = (width * height) as usize;
        let uv_size = y_size / 4;
        let expected_size = y_size + 2 * uv_size;

        if data.len() < expected_size {
            bail!(
                "I420 data too small: expected {}, got {}",
                expected_size,
                data.len()
            );
        }

        let y_plane = &data[..y_size];
        let u_plane = &data[y_size..y_size + uv_size];
        let v_plane = &data[y_size + uv_size..expected_size];

        let uv_width = width / 2;

        let mut rgb_data = Vec::with_capacity(y_size * 3);

        for y_pos in 0..height {
            for x_pos in 0..width {
                let y_idx = (y_pos * width + x_pos) as usize;
                let uv_idx = ((y_pos / 2) * uv_width + (x_pos / 2)) as usize;

                let y = y_plane[y_idx] as f32;
                let u = u_plane[uv_idx] as f32 - 128.0;
                let v = v_plane[uv_idx] as f32 - 128.0;

                // BT.601 conversion
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

                rgb_data.push(r);
                rgb_data.push(g);
                rgb_data.push(b);
            }
        }

        let image = RgbImage::from_raw(width, height, rgb_data)
            .context("Failed to create RGB image from I420")?;

        Ok(DynamicImage::ImageRgb8(image))
    }

    /// Resize an image to fit within the given dimensions while preserving aspect ratio.
    pub fn resize_to_fit(image: DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
        let (width, height) = (image.width(), image.height());

        if width <= max_width && height <= max_height {
            return image;
        }

        let scale = f32::min(
            max_width as f32 / width as f32,
            max_height as f32 / height as f32,
        );

        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    }

    /// Decode encoded image data (JPEG, PNG, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if the image format is unsupported or data is invalid.
    pub fn decode_encoded(data: &[u8]) -> Result<DynamicImage> {
        image::load_from_memory(data).context("Failed to decode image")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_conversion() {
        let width = 2;
        let height = 2;
        let data = vec![
            255, 0, 0, // Red
            0, 255, 0, // Green
            0, 0, 255, // Blue
            255, 255, 255, // White
        ];

        let image = ImageEncoder::from_rgb(&data, width, height).unwrap();
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
    }

    #[test]
    fn test_bgra_conversion() {
        let width = 2;
        let height = 1;
        // BGRA: Blue=255, Green=0, Red=0, Alpha=255 -> should become Red=0, Green=0, Blue=255
        let data = vec![255, 0, 0, 255, 0, 255, 0, 255];

        let image = ImageEncoder::from_bgra(&data, width, height).unwrap();
        let rgba = image.to_rgba8();

        // First pixel should be blue
        let pixel = rgba.get_pixel(0, 0);
        assert_eq!(pixel[0], 0); // R
        assert_eq!(pixel[1], 0); // G
        assert_eq!(pixel[2], 255); // B

        // Second pixel should be green
        let pixel = rgba.get_pixel(1, 0);
        assert_eq!(pixel[0], 0); // R
        assert_eq!(pixel[1], 255); // G
        assert_eq!(pixel[2], 0); // B
    }

    #[test]
    fn test_resize_to_fit() {
        let image = DynamicImage::new_rgb8(1920, 1080);
        let resized = ImageEncoder::resize_to_fit(image, 640, 480);

        assert!(resized.width() <= 640);
        assert!(resized.height() <= 480);
    }

    #[test]
    fn test_small_image_no_resize() {
        let image = DynamicImage::new_rgb8(320, 240);
        let resized = ImageEncoder::resize_to_fit(image, 640, 480);

        assert_eq!(resized.width(), 320);
        assert_eq!(resized.height(), 240);
    }
}
