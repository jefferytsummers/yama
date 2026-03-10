//! Quick VLM Test - Standalone inference without event bus
//!
//! Run with: cargo run -p yama-vlm --features metal --example quick_test
//!
//! This loads the VLM model and runs a single inference on a test image.
//! First run will download the model (~4GB for 7B, ~2GB for 3B).
//!
//! Environment variables:
//!   VLM_MODEL_ID - HuggingFace model ID (default: Qwen/Qwen2.5-VL-7B-Instruct)
//!   VLM_ISQ - Quantization level: Q4K, Q8_0, F16 (default: Q4K)

use std::time::Instant;

use anyhow::Result;
use image::{DynamicImage, Rgb, RgbImage};
use mistralrs::{Device, IsqType, TextMessageRole, VisionMessages, VisionModelBuilder};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Get model configuration from environment
    // Note: Qwen2.5-VL has image handling issues in current mistral.rs
    // Using Qwen2-VL-2B which is documented and working
    let model_id = std::env::var("VLM_MODEL_ID")
        .unwrap_or_else(|_| "Qwen/Qwen2-VL-2B-Instruct".to_string());

    let isq = std::env::var("VLM_ISQ").unwrap_or_else(|_| "Q4K".to_string());

    println!("\n==========================================");
    println!("  Yama VLM Quick Test");
    println!("==========================================\n");
    println!("Model: {}", model_id);
    println!("Quantization: {}", isq);
    println!("Device: Metal (GPU)\n");

    // Parse ISQ type
    let isq_type = match isq.to_uppercase().as_str() {
        "Q4K" | "Q4_K" => Some(IsqType::Q4K),
        "Q8_0" | "Q8" => Some(IsqType::Q8_0),
        "F16" | "FP16" => None,
        _ => Some(IsqType::Q4K),
    };

    // Create Metal device
    info!("Creating Metal device...");
    let device = Device::new_metal(0)?;
    info!("Metal device ready");

    // Build the model
    info!("Loading VLM model (first run downloads ~4GB)...");
    let load_start = Instant::now();

    let mut builder = VisionModelBuilder::new(&model_id)
        .with_logging()
        .with_device(device);

    if let Some(isq) = isq_type {
        builder = builder.with_isq(isq);
    }

    let model = builder.build().await?;
    let load_time = load_start.elapsed();

    info!("Model loaded in {:.2}s", load_time.as_secs_f64());

    // Create a test image (gradient pattern)
    info!("Creating test image...");
    let mut img = RgbImage::new(256, 256);
    for y in 0..256 {
        for x in 0..256 {
            img.put_pixel(x, y, Rgb([x as u8, y as u8, 128]));
        }
    }
    let test_image = DynamicImage::ImageRgb8(img);

    // Run warmup inference
    info!("Running warmup inference...");
    let warmup_start = Instant::now();

    let warmup_messages = VisionMessages::new().add_image_message(
        TextMessageRole::User,
        "What colors do you see?".to_string(),
        vec![test_image.clone()],
        &model,
    )?;

    let _ = model.send_chat_request(warmup_messages).await?;
    let warmup_time = warmup_start.elapsed();
    info!("Warmup complete in {:.2}s", warmup_time.as_secs_f64());

    // Run actual test
    println!("\n==========================================");
    println!("  Running Inference Test");
    println!("==========================================\n");

    let prompt = "Describe this image in detail. What patterns, colors, and shapes do you see?";
    println!("Prompt: {}\n", prompt);

    let infer_start = Instant::now();

    let messages = VisionMessages::new().add_image_message(
        TextMessageRole::User,
        prompt.to_string(),
        vec![test_image],
        &model,
    )?;

    let response = model.send_chat_request(messages).await?;
    let infer_time = infer_start.elapsed();

    // Extract response
    let text = response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    let tokens = response.usage.completion_tokens;

    println!("Response:\n{}\n", text);
    println!("==========================================");
    println!("  Stats");
    println!("==========================================");
    println!("Tokens generated: {}", tokens);
    println!("Inference time: {:.2}s", infer_time.as_secs_f64());
    println!(
        "Tokens/sec: {:.1}",
        tokens as f64 / infer_time.as_secs_f64()
    );
    println!("Model load time: {:.2}s", load_time.as_secs_f64());
    println!();

    Ok(())
}
