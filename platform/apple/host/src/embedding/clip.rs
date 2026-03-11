//! CLIP visual embeddings using ONNX Runtime.
//!
//! Provides visual embedding generation using OpenAI's CLIP model.
//! Uses ONNX Runtime with CoreML execution provider for Metal acceleration.

use std::path::Path;

use anyhow::{Context, Result};
use image::DynamicImage;
use ndarray::{Array2, ArrayViewD};
use ort::{
    ep,
    inputs,
    session::Session,
    value::Tensor,
};
use tracing::{debug, info};

use super::preprocessor::{l2_normalize, ImagePreprocessor};

/// CLIP embedding dimension for ViT-B/32.
pub const CLIP_EMBEDDING_DIM: usize = 512;

/// CLIP embedding dimension for ViT-L/14.
pub const CLIP_LARGE_EMBEDDING_DIM: usize = 768;

/// CLIP model variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipModel {
    /// ViT-B/32 (512-dim embeddings, faster).
    VitB32,
    /// ViT-L/14 (768-dim embeddings, more accurate).
    VitL14,
}

impl ClipModel {
    /// Get the embedding dimension for this model.
    pub fn embedding_dim(&self) -> usize {
        match self {
            Self::VitB32 => CLIP_EMBEDDING_DIM,
            Self::VitL14 => CLIP_LARGE_EMBEDDING_DIM,
        }
    }

    /// Get the model name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::VitB32 => "clip-vit-base-patch32",
            Self::VitL14 => "clip-vit-large-patch14",
        }
    }
}

/// CLIP embedder configuration.
#[derive(Debug, Clone)]
pub struct ClipConfig {
    /// Model variant to use.
    pub model: ClipModel,
    /// Batch size for encoding.
    pub batch_size: usize,
    /// Whether to use CoreML (Metal) acceleration.
    pub use_coreml: bool,
    /// Whether to normalize embeddings to unit length.
    pub normalize: bool,
}

impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            model: ClipModel::VitB32,
            batch_size: 32,
            use_coreml: true,
            normalize: true,
        }
    }
}

/// CLIP visual embedder.
///
/// Generates visual embeddings from images using CLIP.
pub struct ClipEmbedder {
    config: ClipConfig,
    session: Session,
    preprocessor: ImagePreprocessor,
}

impl ClipEmbedder {
    /// Create a new CLIP embedder from a model file.
    pub fn new(model_path: impl AsRef<Path>, config: ClipConfig) -> Result<Self> {
        let model_path = model_path.as_ref();
        info!("Loading CLIP model from {:?}", model_path);

        // Build session with optional CoreML
        let session = if config.use_coreml {
            let build_result: std::result::Result<Session, ort::Error> = (|| {
                let mut builder = Session::builder()?;
                let mut builder = builder.with_execution_providers([
                    ep::CoreML::default()
                        .with_subgraphs(true)
                        .build()
                ])?;
                builder.commit_from_file(model_path)
            })();

            match build_result {
                Ok(s) => {
                    info!("Using CoreML (Metal) acceleration");
                    s
                }
                Err(e) => {
                    debug!("CoreML not available, falling back to CPU: {}", e);
                    let mut builder = Session::builder()
                        .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?;
                    builder.commit_from_file(model_path)
                        .map_err(|e| anyhow::anyhow!("Failed to load CLIP model: {}", e))?
                }
            }
        } else {
            let mut builder = Session::builder()
                .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?;
            builder.commit_from_file(model_path)
                .map_err(|e| anyhow::anyhow!("Failed to load CLIP model: {}", e))?
        };

        let preprocessor = ImagePreprocessor::clip();

        Ok(Self {
            config,
            session,
            preprocessor,
        })
    }

    /// Get the embedding dimension.
    pub fn embedding_dim(&self) -> usize {
        self.config.model.embedding_dim()
    }

    /// Encode a single image to an embedding vector.
    pub fn encode_image(&mut self, image: &DynamicImage) -> Result<Vec<f32>> {
        let tensor = self.preprocessor.preprocess(image)?;
        let embeddings = self.run_inference(tensor)?;

        let mut embedding = embeddings.row(0).to_vec();
        if self.config.normalize {
            l2_normalize(&mut embedding);
        }

        Ok(embedding)
    }

    /// Encode image bytes (JPEG, PNG, etc.) to an embedding vector.
    pub fn encode_bytes(&mut self, bytes: &[u8]) -> Result<Vec<f32>> {
        let image = image::load_from_memory(bytes)
            .context("Failed to decode image")?;
        self.encode_image(&image)
    }

    /// Encode multiple images to embedding vectors.
    ///
    /// Returns a 2D array of shape [N, embedding_dim].
    pub fn encode_images(&mut self, images: &[DynamicImage]) -> Result<Array2<f32>> {
        if images.is_empty() {
            return Ok(Array2::zeros((0, self.embedding_dim())));
        }

        // Process in batches
        let mut all_embeddings = Vec::new();

        for batch_start in (0..images.len()).step_by(self.config.batch_size) {
            let batch_end = (batch_start + self.config.batch_size).min(images.len());
            let batch = &images[batch_start..batch_end];

            let tensor = self.preprocessor.preprocess_batch(batch)?;
            let embeddings = self.run_inference(tensor)?;

            for i in 0..embeddings.nrows() {
                let mut embedding = embeddings.row(i).to_vec();
                if self.config.normalize {
                    l2_normalize(&mut embedding);
                }
                all_embeddings.push(embedding);
            }
        }

        // Convert to 2D array
        let n = all_embeddings.len();
        let dim = self.embedding_dim();
        let mut result = Array2::zeros((n, dim));

        for (i, emb) in all_embeddings.into_iter().enumerate() {
            for (j, val) in emb.into_iter().enumerate() {
                result[[i, j]] = val;
            }
        }

        Ok(result)
    }

    /// Encode multiple image byte arrays.
    pub fn encode_bytes_batch(&mut self, images: &[&[u8]]) -> Result<Array2<f32>> {
        let decoded: Result<Vec<DynamicImage>> = images
            .iter()
            .map(|bytes| {
                image::load_from_memory(bytes).context("Failed to decode image")
            })
            .collect();

        self.encode_images(&decoded?)
    }

    /// Run inference on preprocessed image tensor.
    fn run_inference(&mut self, tensor: ndarray::Array4<f32>) -> Result<Array2<f32>> {
        // Create owned tensor for ONNX input
        let input = Tensor::from_array(tensor)
            .map_err(|e| anyhow::anyhow!("Failed to create input tensor: {}", e))?;

        // Run inference
        let outputs = self.session.run(inputs![input])
            .map_err(|e| anyhow::anyhow!("Inference failed: {}", e))?;

        // Extract embeddings from output - try named outputs first, then first output
        let output = if let Some(o) = outputs.get("image_embeds") {
            o
        } else if let Some(o) = outputs.get("last_hidden_state") {
            o
        } else if let Some(o) = outputs.get("output") {
            o
        } else {
            &outputs[0]
        };

        // Try to extract as ndarray
        let output_array: ArrayViewD<f32> = output.try_extract_array()
            .map_err(|e| anyhow::anyhow!("Failed to extract output as array: {}", e))?;

        let shape = output_array.shape();

        // Handle different output shapes
        let result = if shape.len() == 2 {
            // [batch, embedding_dim]
            let batch_size = shape[0];
            let embedding_dim = shape[1];
            let mut result = Array2::zeros((batch_size, embedding_dim));
            for i in 0..batch_size {
                for j in 0..embedding_dim {
                    result[[i, j]] = output_array[[i, j]];
                }
            }
            result
        } else if shape.len() == 3 {
            // [batch, seq_len, embedding_dim] - take CLS token (first position)
            let batch_size = shape[0];
            let embedding_dim = shape[2];
            let mut embeddings = Array2::zeros((batch_size, embedding_dim));

            for b in 0..batch_size {
                for d in 0..embedding_dim {
                    embeddings[[b, d]] = output_array[[b, 0, d]];
                }
            }
            embeddings
        } else {
            anyhow::bail!("Unexpected output shape: {:?}", shape);
        };

        Ok(result)
    }
}

/// Text embedder for CLIP (optional, for similarity search).
pub struct ClipTextEmbedder {
    session: Session,
    embedding_dim: usize,
    normalize: bool,
}

impl ClipTextEmbedder {
    /// Create a new text embedder from a model file.
    pub fn new(model_path: impl AsRef<Path>, embedding_dim: usize) -> Result<Self> {
        let model_path = model_path.as_ref();
        info!("Loading CLIP text encoder from {:?}", model_path);

        let mut builder = Session::builder()
            .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?;
        let session = builder.commit_from_file(model_path)
            .map_err(|e| anyhow::anyhow!("Failed to load CLIP text model: {}", e))?;

        Ok(Self {
            session,
            embedding_dim,
            normalize: true,
        })
    }

    /// Encode a text query to an embedding vector.
    ///
    /// Note: This requires a tokenizer, which is not included here.
    /// For full text encoding, use a tokenizer like `tokenizers` crate.
    pub fn encode_tokens(&mut self, input_ids: &[i64], attention_mask: &[i64]) -> Result<Vec<f32>> {
        let seq_len = input_ids.len();

        let input_ids_array = ndarray::Array2::from_shape_vec(
            (1, seq_len),
            input_ids.to_vec(),
        )?;
        let attention_mask_array = ndarray::Array2::from_shape_vec(
            (1, seq_len),
            attention_mask.to_vec(),
        )?;

        let input_ids_tensor = Tensor::from_array(input_ids_array)
            .map_err(|e| anyhow::anyhow!("Failed to create input_ids tensor: {}", e))?;
        let attention_mask_tensor = Tensor::from_array(attention_mask_array)
            .map_err(|e| anyhow::anyhow!("Failed to create attention_mask tensor: {}", e))?;

        let outputs = self.session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
        ]).map_err(|e| anyhow::anyhow!("Inference failed: {}", e))?;

        let output = &outputs[0];

        let output_array: ArrayViewD<f32> = output.try_extract_array()
            .map_err(|e| anyhow::anyhow!("Failed to extract output tensor: {}", e))?;

        let mut embedding: Vec<f32> = output_array.iter().copied().take(self.embedding_dim).collect();

        if self.normalize {
            l2_normalize(&mut embedding);
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_model_dim() {
        assert_eq!(ClipModel::VitB32.embedding_dim(), 512);
        assert_eq!(ClipModel::VitL14.embedding_dim(), 768);
    }

    #[test]
    fn test_clip_config_defaults() {
        let config = ClipConfig::default();
        assert_eq!(config.model, ClipModel::VitB32);
        assert_eq!(config.batch_size, 32);
        assert!(config.use_coreml);
        assert!(config.normalize);
    }

    // Integration tests require actual model files
    // They would be run with: cargo test --features integration-tests
}
