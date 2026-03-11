//! Text embeddings using ONNX Runtime.
//!
//! Provides text embedding generation using sentence transformers (e.g., all-MiniLM-L6-v2).
//! Uses ONNX Runtime with CoreML execution provider for acceleration.

use std::path::Path;

use anyhow::{Context, Result};
use ndarray::{Array2, ArrayViewD};
use ort::{
    ep,
    inputs,
    session::Session,
    value::Tensor,
};
use tracing::{debug, info};

use super::preprocessor::l2_normalize;

/// Text embedding dimension for all-MiniLM-L6-v2.
pub const TEXT_EMBEDDING_DIM: usize = 384;

/// Text embedder configuration.
#[derive(Debug, Clone)]
pub struct TextEmbedderConfig {
    /// Maximum sequence length.
    pub max_length: usize,
    /// Whether to use CoreML (Metal) acceleration.
    pub use_coreml: bool,
    /// Whether to normalize embeddings to unit length.
    pub normalize: bool,
    /// Padding token ID.
    pub pad_token_id: i64,
    /// CLS token ID.
    pub cls_token_id: i64,
    /// SEP token ID.
    pub sep_token_id: i64,
}

impl Default for TextEmbedderConfig {
    fn default() -> Self {
        Self {
            max_length: 128,
            use_coreml: true,
            normalize: true,
            pad_token_id: 0,
            cls_token_id: 101,
            sep_token_id: 102,
        }
    }
}

/// Simple tokenizer for text embedding models.
///
/// This is a basic word-piece-like tokenizer for demonstration.
/// For production use, consider using the `tokenizers` crate.
pub struct SimpleTokenizer {
    /// Vocabulary mapping words to token IDs.
    vocab: std::collections::HashMap<String, i64>,
    /// Unknown token ID.
    unk_token_id: i64,
    /// Maximum sequence length.
    max_length: usize,
    /// Padding token ID.
    pad_token_id: i64,
    /// CLS token ID.
    cls_token_id: i64,
    /// SEP token ID.
    sep_token_id: i64,
}

impl SimpleTokenizer {
    /// Create a new tokenizer with default vocabulary.
    ///
    /// Note: For production, load a real vocabulary file.
    pub fn new(config: &TextEmbedderConfig) -> Self {
        Self {
            vocab: std::collections::HashMap::new(),
            unk_token_id: 100,
            max_length: config.max_length,
            pad_token_id: config.pad_token_id,
            cls_token_id: config.cls_token_id,
            sep_token_id: config.sep_token_id,
        }
    }

    /// Load vocabulary from a file.
    pub fn load_vocab(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read vocabulary file")?;

        self.vocab.clear();
        for (idx, line) in content.lines().enumerate() {
            let token = line.trim();
            if !token.is_empty() {
                self.vocab.insert(token.to_string(), idx as i64);
            }
        }

        info!("Loaded {} vocabulary entries", self.vocab.len());
        Ok(())
    }

    /// Tokenize text into input IDs and attention mask.
    pub fn tokenize(&self, text: &str) -> (Vec<i64>, Vec<i64>) {
        let mut input_ids = Vec::with_capacity(self.max_length);
        let mut attention_mask = Vec::with_capacity(self.max_length);

        // Add CLS token
        input_ids.push(self.cls_token_id);
        attention_mask.push(1);

        // Tokenize words (simple whitespace + lowercase)
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if input_ids.len() >= self.max_length - 1 {
                break;
            }

            let lower = word.to_lowercase();
            let token_id = self.vocab.get(&lower).copied().unwrap_or(self.unk_token_id);
            input_ids.push(token_id);
            attention_mask.push(1);
        }

        // Add SEP token
        if input_ids.len() < self.max_length {
            input_ids.push(self.sep_token_id);
            attention_mask.push(1);
        }

        // Pad to max length
        while input_ids.len() < self.max_length {
            input_ids.push(self.pad_token_id);
            attention_mask.push(0);
        }

        (input_ids, attention_mask)
    }

    /// Batch tokenize multiple texts.
    pub fn tokenize_batch(&self, texts: &[&str]) -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
        let mut all_input_ids = Vec::with_capacity(texts.len());
        let mut all_attention_masks = Vec::with_capacity(texts.len());

        for text in texts {
            let (input_ids, attention_mask) = self.tokenize(text);
            all_input_ids.push(input_ids);
            all_attention_masks.push(attention_mask);
        }

        (all_input_ids, all_attention_masks)
    }
}

/// Text embedder using sentence transformers.
pub struct TextEmbedder {
    config: TextEmbedderConfig,
    session: Session,
    tokenizer: SimpleTokenizer,
    embedding_dim: usize,
}

impl TextEmbedder {
    /// Create a new text embedder from a model file.
    pub fn new(model_path: impl AsRef<Path>, config: TextEmbedderConfig) -> Result<Self> {
        let model_path = model_path.as_ref();
        info!("Loading text embedding model from {:?}", model_path);

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
                    info!("Using CoreML (Metal) acceleration for text embeddings");
                    s
                }
                Err(e) => {
                    debug!("CoreML not available, falling back to CPU: {}", e);
                    let mut builder = Session::builder()
                        .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?;
                    builder.commit_from_file(model_path)
                        .map_err(|e| anyhow::anyhow!("Failed to load text embedding model: {}", e))?
                }
            }
        } else {
            let mut builder = Session::builder()
                .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?;
            builder.commit_from_file(model_path)
                .map_err(|e| anyhow::anyhow!("Failed to load text embedding model: {}", e))?
        };

        let tokenizer = SimpleTokenizer::new(&config);

        Ok(Self {
            config,
            session,
            tokenizer,
            embedding_dim: TEXT_EMBEDDING_DIM,
        })
    }

    /// Load a vocabulary file for the tokenizer.
    pub fn load_vocab(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.tokenizer.load_vocab(path)
    }

    /// Get the embedding dimension.
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    /// Encode a single text to an embedding vector.
    pub fn encode(&mut self, text: &str) -> Result<Vec<f32>> {
        let (input_ids, attention_mask) = self.tokenizer.tokenize(text);
        self.encode_tokens(&input_ids, &attention_mask)
    }

    /// Encode multiple texts to embedding vectors.
    pub fn encode_batch(&mut self, texts: &[&str]) -> Result<Array2<f32>> {
        if texts.is_empty() {
            return Ok(Array2::zeros((0, self.embedding_dim)));
        }

        let (all_input_ids, all_attention_masks) = self.tokenizer.tokenize_batch(texts);

        let batch_size = texts.len();
        let seq_len = self.config.max_length;

        // Flatten for ndarray
        let input_ids_flat: Vec<i64> = all_input_ids.into_iter().flatten().collect();
        let attention_mask_flat: Vec<i64> = all_attention_masks.into_iter().flatten().collect();

        let input_ids_array = ndarray::Array2::from_shape_vec(
            (batch_size, seq_len),
            input_ids_flat,
        )?;
        let attention_mask_array = ndarray::Array2::from_shape_vec(
            (batch_size, seq_len),
            attention_mask_flat,
        )?;

        let input_ids_tensor = Tensor::from_array(input_ids_array)
            .map_err(|e| anyhow::anyhow!("Failed to create input_ids tensor: {}", e))?;
        let attention_mask_tensor = Tensor::from_array(attention_mask_array)
            .map_err(|e| anyhow::anyhow!("Failed to create attention_mask tensor: {}", e))?;

        let outputs = self.session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
        ]).map_err(|e| anyhow::anyhow!("Inference failed: {}", e))?;

        // Get output tensor - try named outputs first, then first output
        let output = if let Some(o) = outputs.get("sentence_embedding") {
            o
        } else if let Some(o) = outputs.get("last_hidden_state") {
            o
        } else {
            &outputs[0]
        };

        let output_array: ArrayViewD<f32> = output.try_extract_array()
            .map_err(|e| anyhow::anyhow!("Failed to extract output tensor: {}", e))?;

        let shape = output_array.shape();

        // Handle different output shapes
        let mut result = if shape.len() == 2 {
            // [batch, embedding_dim] - sentence embedding output
            let mut arr = Array2::zeros((shape[0], shape[1]));
            for i in 0..shape[0] {
                for j in 0..shape[1] {
                    arr[[i, j]] = output_array[[i, j]];
                }
            }
            arr
        } else if shape.len() == 3 {
            // [batch, seq_len, hidden_dim] - need to mean pool
            let hidden_dim = shape[2];
            let mut embeddings = Array2::zeros((batch_size, hidden_dim));

            for b in 0..batch_size {
                // Mean pooling over sequence dimension
                let mut sum = vec![0.0f32; hidden_dim];
                let count = shape[1];

                for s in 0..shape[1] {
                    for d in 0..hidden_dim {
                        sum[d] += output_array[[b, s, d]];
                    }
                }

                for d in 0..hidden_dim {
                    embeddings[[b, d]] = sum[d] / count as f32;
                }
            }
            embeddings
        } else {
            anyhow::bail!("Unexpected output shape: {:?}", shape);
        };

        // Normalize if configured
        if self.config.normalize {
            for i in 0..result.nrows() {
                let mut row = result.row(i).to_vec();
                l2_normalize(&mut row);
                for (j, val) in row.into_iter().enumerate() {
                    result[[i, j]] = val;
                }
            }
        }

        Ok(result)
    }

    /// Encode pre-tokenized input.
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

        let output = if let Some(o) = outputs.get("sentence_embedding") {
            o
        } else if let Some(o) = outputs.get("last_hidden_state") {
            o
        } else {
            &outputs[0]
        };

        let output_array: ArrayViewD<f32> = output.try_extract_array()
            .map_err(|e| anyhow::anyhow!("Failed to extract output tensor: {}", e))?;

        let shape = output_array.shape();

        // Extract embedding
        let mut embedding = if shape.len() == 2 {
            // [1, embedding_dim]
            let dim = shape[1].min(self.embedding_dim);
            (0..dim).map(|j| output_array[[0, j]]).collect()
        } else if shape.len() == 3 {
            // [1, seq_len, hidden_dim] - take mean pooling
            let hidden_dim = shape[2];
            let mut sum = vec![0.0f32; hidden_dim];
            let count = shape[1];

            for s in 0..shape[1] {
                for d in 0..hidden_dim {
                    sum[d] += output_array[[0, s, d]];
                }
            }

            for val in &mut sum {
                *val /= count as f32;
            }
            sum
        } else {
            anyhow::bail!("Unexpected output shape: {:?}", shape);
        };

        // Normalize if configured
        if self.config.normalize {
            l2_normalize(&mut embedding);
        }

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_embedder_config_defaults() {
        let config = TextEmbedderConfig::default();
        assert_eq!(config.max_length, 128);
        assert!(config.use_coreml);
        assert!(config.normalize);
    }

    #[test]
    fn test_simple_tokenizer() {
        let config = TextEmbedderConfig::default();
        let tokenizer = SimpleTokenizer::new(&config);

        let (input_ids, attention_mask) = tokenizer.tokenize("hello world");

        // Should have CLS + 2 words + SEP + padding
        assert_eq!(input_ids.len(), config.max_length);
        assert_eq!(attention_mask.len(), config.max_length);

        // First token should be CLS
        assert_eq!(input_ids[0], config.cls_token_id);

        // Check attention mask (1s for real tokens, 0s for padding)
        assert_eq!(attention_mask[0], 1); // CLS
        assert_eq!(attention_mask[1], 1); // hello
        assert_eq!(attention_mask[2], 1); // world
        assert_eq!(attention_mask[3], 1); // SEP
        assert_eq!(attention_mask[4], 0); // padding
    }

    #[test]
    fn test_embedding_dim() {
        assert_eq!(TEXT_EMBEDDING_DIM, 384);
    }
}
