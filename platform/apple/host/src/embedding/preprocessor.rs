//! Image preprocessing for embedding models.
//!
//! Handles resizing, normalization, and tensor conversion for CLIP and other models.

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use ndarray::{Array, Array4};

/// CLIP model input size (224x224 for ViT-B/32).
pub const CLIP_IMAGE_SIZE: u32 = 224;

/// ImageNet normalization mean values (RGB).
pub const IMAGENET_MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];

/// ImageNet normalization std values (RGB).
pub const IMAGENET_STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

/// Image preprocessor for embedding models.
#[derive(Debug, Clone)]
pub struct ImagePreprocessor {
    /// Target image size.
    target_size: u32,
    /// Mean values for normalization (RGB).
    mean: [f32; 3],
    /// Std values for normalization (RGB).
    std: [f32; 3],
    /// Whether to use center crop.
    center_crop: bool,
}

impl ImagePreprocessor {
    /// Create a preprocessor for CLIP models.
    pub fn clip() -> Self {
        Self {
            target_size: CLIP_IMAGE_SIZE,
            mean: IMAGENET_MEAN,
            std: IMAGENET_STD,
            center_crop: true,
        }
    }

    /// Create a custom preprocessor.
    pub fn new(target_size: u32, mean: [f32; 3], std: [f32; 3]) -> Self {
        Self {
            target_size,
            mean,
            std,
            center_crop: true,
        }
    }

    /// Set whether to use center crop.
    pub fn with_center_crop(mut self, center_crop: bool) -> Self {
        self.center_crop = center_crop;
        self
    }

    /// Preprocess an image for model input.
    ///
    /// Returns a tensor of shape [1, 3, H, W] with normalized values.
    pub fn preprocess(&self, image: &DynamicImage) -> Result<Array4<f32>> {
        // Resize and optionally center crop
        let processed = if self.center_crop {
            self.resize_and_center_crop(image)
        } else {
            self.resize_stretch(image)
        };

        // Convert to normalized tensor
        self.to_tensor(&processed)
    }

    /// Preprocess multiple images as a batch.
    ///
    /// Returns a tensor of shape [N, 3, H, W].
    pub fn preprocess_batch(&self, images: &[DynamicImage]) -> Result<Array4<f32>> {
        if images.is_empty() {
            return Ok(Array4::zeros((0, 3, self.target_size as usize, self.target_size as usize)));
        }

        let mut batch = Vec::with_capacity(images.len());
        for image in images {
            let tensor = self.preprocess(image)?;
            batch.push(tensor);
        }

        // Stack along batch dimension
        let batch_size = batch.len();
        let c = 3;
        let h = self.target_size as usize;
        let w = self.target_size as usize;

        let mut result = Array4::zeros((batch_size, c, h, w));
        for (i, tensor) in batch.into_iter().enumerate() {
            result.slice_mut(ndarray::s![i, .., .., ..]).assign(&tensor.slice(ndarray::s![0, .., .., ..]));
        }

        Ok(result)
    }

    /// Preprocess image bytes (JPEG, PNG, etc.).
    pub fn preprocess_bytes(&self, bytes: &[u8]) -> Result<Array4<f32>> {
        let image = image::load_from_memory(bytes)
            .context("Failed to decode image")?;
        self.preprocess(&image)
    }

    /// Resize and center crop to target size.
    fn resize_and_center_crop(&self, image: &DynamicImage) -> DynamicImage {
        let (width, height) = image.dimensions();
        let target = self.target_size;

        // Calculate scale to make shorter side equal to target
        let scale = if width < height {
            target as f32 / width as f32
        } else {
            target as f32 / height as f32
        };

        let new_width = (width as f32 * scale).ceil() as u32;
        let new_height = (height as f32 * scale).ceil() as u32;

        // Resize
        let resized = image.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        );

        // Center crop
        let x = (new_width - target) / 2;
        let y = (new_height - target) / 2;
        resized.crop_imm(x, y, target, target)
    }

    /// Resize by stretching to target size.
    fn resize_stretch(&self, image: &DynamicImage) -> DynamicImage {
        image.resize_exact(
            self.target_size,
            self.target_size,
            image::imageops::FilterType::Lanczos3,
        )
    }

    /// Convert image to normalized tensor.
    fn to_tensor(&self, image: &DynamicImage) -> Result<Array4<f32>> {
        let rgb = image.to_rgb8();
        let (width, height) = rgb.dimensions();

        let mut tensor = Array4::zeros((1, 3, height as usize, width as usize));

        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x, y);
                for c in 0..3 {
                    // Convert to [0, 1] range
                    let value = pixel[c] as f32 / 255.0;
                    // Normalize with mean and std
                    let normalized = (value - self.mean[c]) / self.std[c];
                    tensor[[0, c, y as usize, x as usize]] = normalized;
                }
            }
        }

        Ok(tensor)
    }
}

/// Normalize a vector to unit length (L2 normalization).
pub fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-12 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 1e-12 && norm_b > 1e-12 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_preprocessor() {
        let preprocessor = ImagePreprocessor::clip();
        assert_eq!(preprocessor.target_size, 224);
    }

    #[test]
    fn test_l2_normalize() {
        let mut vec = vec![3.0, 4.0];
        l2_normalize(&mut vec);
        assert!((vec[0] - 0.6).abs() < 1e-5);
        assert!((vec[1] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-5);
    }

    #[test]
    fn test_preprocess_synthetic_image() {
        let preprocessor = ImagePreprocessor::clip();

        // Create a simple test image
        let img = DynamicImage::ImageRgb8(ImageBuffer::from_fn(640, 480, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));

        let tensor = preprocessor.preprocess(&img).unwrap();
        assert_eq!(tensor.shape(), &[1, 3, 224, 224]);
    }
}
