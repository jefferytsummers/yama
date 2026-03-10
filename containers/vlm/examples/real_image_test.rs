//! Real Image VLM Test - Test with actual image file
//!
//! Run with: VLM_IMAGE=/tmp/test_frame.jpg cargo run -p yama-vlm --features metal --example real_image_test
//!
//! This loads a real image and runs inference to verify the image pipeline works.

use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use image::io::Reader as ImageReader;
use mistralrs::{Device, TextMessageRole, VisionMessages, VisionModelBuilder};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    // Get image path from environment
    let image_path = std::env::var("VLM_IMAGE")
        .unwrap_or_else(|_| "/tmp/test_frame.jpg".to_string());

    let model_id = std::env::var("VLM_MODEL_ID")
        .unwrap_or_else(|_| "Qwen/Qwen2.5-VL-3B-Instruct".to_string());

    println!("\n==========================================");
    println!("  Yama VLM Real Image Test");
    println!("==========================================\n");
    println!("Image: {}", image_path);
    println!("Model: {}", model_id);
    println!("Device: Metal (GPU)\n");

    // Load the real image
    info!("Loading image from file...");
    let image = ImageReader::open(Path::new(&image_path))?
        .decode()?;

    println!("Image dimensions: {}x{}", image.width(), image.height());

    // Create Metal device
    info!("Creating Metal device...");
    let device = Device::new_metal(0)?;
    info!("Metal device ready");

    // Build the model (no ISQ for faster load)
    info!("Loading VLM model...");
    let load_start = Instant::now();

    let model = VisionModelBuilder::new(&model_id)
        .with_logging()
        .with_device(device)
        .build()
        .await?;

    let load_time = load_start.elapsed();
    info!("Model loaded in {:.2}s", load_time.as_secs_f64());

    // Run inference
    println!("\n==========================================");
    println!("  Running Inference on Real Image");
    println!("==========================================\n");

    let prompt = "Describe what you see in this image in detail.";
    println!("Prompt: {}\n", prompt);

    let infer_start = Instant::now();

    let messages = VisionMessages::new().add_image_message(
        TextMessageRole::User,
        prompt.to_string(),
        vec![image],
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
