//! VLM inference engine using mistral.rs.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use image::DynamicImage;
use mistralrs::{
    Device, IsqType, Model, TextMessageRole, VisionMessages, VisionModelBuilder,
};
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, warn};

use crate::config::ModelConfig;

/// Result of VLM inference.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Generated text.
    pub text: String,
    /// Inference time in milliseconds.
    pub inference_time_ms: f32,
    /// Tokens generated.
    pub tokens_generated: u32,
}

/// VLM inference engine.
pub struct VlmEngine {
    /// The mistral.rs model.
    model: Arc<Mutex<Model>>,
    /// Model configuration.
    config: ModelConfig,
    /// Whether the model is ready for inference.
    ready: bool,
}

impl VlmEngine {
    /// Create a new VLM engine with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if model loading fails.
    #[instrument(skip_all, fields(model_id = %config.model_id))]
    pub async fn new(config: ModelConfig) -> Result<Self> {
        info!("Initializing VLM engine");

        let model = Self::load_model(&config).await?;

        info!("VLM engine initialized");

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            config,
            ready: true,
        })
    }

    /// Load the VLM model.
    async fn load_model(config: &ModelConfig) -> Result<Model> {
        info!("Loading VLM model: {}", config.model_id);

        // Parse ISQ type
        let isq_type = match config.isq.to_uppercase().as_str() {
            "Q4K" | "Q4_K" => Some(IsqType::Q4K),
            "Q8_0" | "Q8" => Some(IsqType::Q8_0),
            "F16" | "FP16" => None, // No quantization
            "BF16" => None,
            _ => {
                warn!("Unknown ISQ type '{}', using Q4K", config.isq);
                Some(IsqType::Q4K)
            }
        };

        // Determine device
        let device = match config.device.to_lowercase().as_str() {
            "metal" | "mps" => Device::new_metal(0).context("Failed to create Metal device")?,
            "cuda" => Device::new_cuda(0).context("Failed to create CUDA device")?,
            "cpu" => Device::Cpu,
            _ => {
                warn!("Unknown device '{}', using CPU", config.device);
                Device::Cpu
            }
        };

        // Determine model source
        let model_id = if let Some(ref path) = config.model_path {
            path.to_string_lossy().to_string()
        } else {
            config.model_id.clone()
        };

        // Build the vision model
        let mut builder = VisionModelBuilder::new(&model_id)
            .with_logging()
            .with_device(device);

        // Apply ISQ if specified
        if let Some(isq) = isq_type {
            builder = builder.with_isq(isq);
        }

        // Build model asynchronously
        let model = builder.build().await.context("Failed to build VLM model")?;

        info!("Model loaded successfully");

        Ok(model)
    }

    /// Analyze an image with the given prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    #[instrument(skip(self, image), fields(prompt_len = prompt.len()))]
    pub async fn analyze(&self, image: DynamicImage, prompt: &str) -> Result<InferenceResult> {
        self.analyze_with_params(image, prompt, None, None).await
    }

    /// Analyze an image with custom parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    #[instrument(skip(self, image), fields(prompt_len = prompt.len()))]
    pub async fn analyze_with_params(
        &self,
        image: DynamicImage,
        prompt: &str,
        _temperature: Option<f32>,
        _max_tokens: Option<u32>,
    ) -> Result<InferenceResult> {
        let start = Instant::now();

        let model = self.model.lock().await;

        // Build vision messages with image as a Vec
        let images = vec![image];
        let messages = VisionMessages::new().add_image_message(
            TextMessageRole::User,
            prompt.to_string(),
            images,
            &*model,
        )?;

        // Run inference
        let response = model
            .send_chat_request(messages)
            .await
            .context("VLM inference failed")?;

        let inference_time = start.elapsed();

        // Extract response text
        let text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default()
            .unwrap_or_default();

        let tokens = response.usage.completion_tokens;

        debug!(
            "Inference complete: {} tokens in {:.2}ms",
            tokens,
            inference_time.as_secs_f32() * 1000.0
        );

        Ok(InferenceResult {
            text,
            inference_time_ms: inference_time.as_secs_f32() * 1000.0,
            tokens_generated: tokens as u32,
        })
    }

    /// Analyze multiple images with the same prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if any inference fails.
    pub async fn analyze_batch(
        &self,
        images: Vec<DynamicImage>,
        prompt: &str,
    ) -> Result<Vec<InferenceResult>> {
        let mut results = Vec::with_capacity(images.len());

        for image in images {
            let result = self.analyze(image, prompt).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Check if the engine is ready for inference.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Get the model configuration.
    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    /// Warm up the model with a test inference.
    ///
    /// # Errors
    ///
    /// Returns an error if warmup fails.
    #[instrument(skip(self))]
    pub async fn warmup(&self) -> Result<()> {
        info!("Warming up VLM engine");

        // Create a small test image
        let test_image = DynamicImage::new_rgb8(64, 64);

        // Run a minimal inference
        let _ = self
            .analyze_with_params(test_image, "What is this?", Some(0.1), Some(16))
            .await?;

        info!("VLM engine warmup complete");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ModelConfig::default();
        assert_eq!(config.model_id, "Qwen/Qwen2.5-VL-7B-Instruct");
        assert_eq!(config.isq, "Q4K");
        assert_eq!(config.device, "metal");
    }
}
