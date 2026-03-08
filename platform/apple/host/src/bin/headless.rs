//! Headless Yama host for testing and CI/CD.
//!
//! Runs VLM inference without the GUI, suitable for:
//! - Automated testing
//! - Batch processing
//! - CI/CD pipelines
//! - Benchmarking
//!
//! # Usage
//!
//! ```bash
//! # Analyze a video with mock backend (no VLM container needed)
//! yama-headless --video sample.mp4 --prompt "Describe this video" --mock
//!
//! # Synthesize results into summary + key moments
//! yama-headless --video sample.mp4 --prompt "What happens?" --synthesize --mock
//!
//! # Use real VLM container (start container first: cargo run -p yama-vlm)
//! yama-headless --video sample.mp4 --prompt "What happens?" --synthesize
//!
//! # Run a test scenario
//! yama-headless --test-scenario single --mock
//!
//! # Check VLM container health
//! yama-headless --health-check
//!
//! # Output as JSON
//! yama-headless --video sample.mp4 --prompt "Describe" --output json
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::Serialize;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// Import modules from the host crate
use yama_host_apple::event_bus::{EventBus, EventBusConfig};
use yama_host_apple::inference::{
    BackendType, InferenceChunk, InferenceJob, JobStatus, UploadInfo, VlmInferenceConfig,
    VlmInferenceService,
};

/// Headless Yama host for VLM inference testing.
#[derive(Parser, Debug)]
#[command(name = "yama-headless")]
#[command(about = "Run Yama VLM inference without GUI")]
#[command(version)]
struct Args {
    /// Path to video file to analyze.
    #[arg(long)]
    video: Option<PathBuf>,

    /// Analysis prompt.
    #[arg(long, default_value = "Describe what you see in this video")]
    prompt: String,

    /// Use mock backend (no VLM container needed).
    #[arg(long)]
    mock: bool,

    /// Synthesize frame results into coherent summary + key moments.
    #[arg(long)]
    synthesize: bool,

    /// Run a predefined test scenario.
    #[arg(long, value_enum)]
    test_scenario: Option<TestScenario>,

    /// Perform a health check on the VLM container.
    #[arg(long)]
    health_check: bool,

    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    output: OutputFormat,

    /// Log level.
    #[arg(long, value_enum, default_value = "info")]
    log_level: LogLevel,

    /// Model to use.
    #[arg(long, default_value = "vlm-default")]
    model: String,
}

/// Test scenarios.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum TestScenario {
    /// Single video analysis.
    Single,
    /// Batch processing of multiple synthetic videos.
    Batch,
    /// Simulated live stream frames.
    Live,
    /// Health check and warmup.
    Health,
}

/// Output format.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum OutputFormat {
    /// Human-readable text.
    #[default]
    Text,
    /// JSON output.
    Json,
}

/// Log level.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum LogLevel {
    /// Error messages only.
    Error,
    /// Warnings and errors.
    Warn,
    /// Informational messages.
    #[default]
    Info,
    /// Debug messages.
    Debug,
    /// Trace messages.
    Trace,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

/// JSON output for results.
#[derive(Debug, Serialize)]
struct JsonOutput {
    success: bool,
    job_id: Option<String>,
    status: Option<String>,
    frames_processed: Option<u64>,
    results: Option<Vec<FrameResult>>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct FrameResult {
    frame_number: u64,
    timestamp_ms: u64,
    text: String,
}

/// Synthesized output combining frame results into a coherent answer.
#[derive(Debug, Serialize)]
struct SynthesizedOutput {
    summary: String,
    key_moments: Vec<KeyMoment>,
    frame_count: usize,
    duration_ms: u64,
}

/// A key moment extracted from the video analysis.
#[derive(Debug, Serialize)]
struct KeyMoment {
    timestamp_ms: u64,
    timestamp_formatted: String,
    description: String,
}

impl From<&InferenceJob> for JsonOutput {
    fn from(job: &InferenceJob) -> Self {
        Self {
            success: job.status == JobStatus::Completed,
            job_id: Some(job.job_id.clone()),
            status: Some(job.status.to_string()),
            frames_processed: Some(job.current_frame),
            results: Some(
                job.results
                    .iter()
                    .map(|c| FrameResult {
                        frame_number: c.frame_number,
                        timestamp_ms: c.timestamp_ms,
                        text: c.text.clone(),
                    })
                    .collect(),
            ),
            error: job.error.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::from(args.log_level))
        .with_target(false)
        .init();

    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<()> {
    info!("Yama Headless VLM Inference");

    // Start event bus server if not using mock mode
    let _event_bus = if !args.mock {
        info!("Starting event bus server for VLM container communication...");
        let event_bus = Arc::new(
            EventBus::new(EventBusConfig::default())
                .await
                .context("Failed to start event bus server")?,
        );

        // Spawn event bus server in background
        let event_bus_handle = event_bus.clone();
        tokio::spawn(async move {
            if let Err(e) = event_bus_handle.run().await {
                error!("Event bus error: {}", e);
            }
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        info!("Event bus server started (ws://localhost:8765, /tmp/yama-event.sock)");
        warn!("Make sure the VLM container is running: cargo run -p yama-vlm");

        Some(event_bus)
    } else {
        None
    };

    // Handle health check
    if args.health_check {
        return run_health_check(&args).await;
    }

    // Handle test scenarios
    if let Some(scenario) = args.test_scenario {
        return run_test_scenario(scenario, &args).await;
    }

    // Handle video analysis
    if let Some(video_path) = &args.video {
        return run_video_analysis(video_path, &args).await;
    }

    // No action specified
    println!("No action specified. Use --help for usage information.");
    println!("\nExamples:");
    println!("  yama-headless --video sample.mp4 --prompt \"Describe this\" --mock");
    println!("  yama-headless --test-scenario single --mock");
    println!("  yama-headless --health-check");

    Ok(())
}

/// Run health check on VLM backend.
async fn run_health_check(args: &Args) -> Result<()> {
    info!("Running health check...");

    let config = VlmInferenceConfig {
        backend: if args.mock {
            BackendType::Mock
        } else {
            BackendType::EventBus
        },
        mock_enabled: args.mock,
        ..Default::default()
    };

    let service = VlmInferenceService::new(config)
        .await
        .context("Failed to create inference service")?;

    let ready = service.is_ready().await;

    match args.output {
        OutputFormat::Text => {
            if ready {
                println!("✓ VLM backend ({}) is ready", service.backend_name());
            } else {
                println!("✗ VLM backend ({}) is not ready", service.backend_name());
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "success": ready,
                "backend": service.backend_name(),
                "ready": ready
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    if ready {
        Ok(())
    } else {
        anyhow::bail!("Health check failed")
    }
}

/// Run a predefined test scenario.
async fn run_test_scenario(scenario: TestScenario, args: &Args) -> Result<()> {
    info!("Running test scenario: {:?}", scenario);

    match scenario {
        TestScenario::Single => run_single_test(args).await,
        TestScenario::Batch => run_batch_test(args).await,
        TestScenario::Live => run_live_test(args).await,
        TestScenario::Health => run_health_check(args).await,
    }
}

/// Run single video analysis test with synthetic data.
async fn run_single_test(args: &Args) -> Result<()> {
    info!("Running single video test...");

    // Create a temporary test file
    let temp_dir = tempfile::tempdir()?;
    let test_video = temp_dir.path().join("test_video.mp4");

    // Create an empty file as placeholder
    tokio::fs::write(&test_video, b"mock video content").await?;

    run_video_analysis(&test_video, args).await
}

/// Run batch processing test.
async fn run_batch_test(args: &Args) -> Result<()> {
    info!("Running batch test...");

    let config = VlmInferenceConfig {
        backend: if args.mock {
            BackendType::Mock
        } else {
            BackendType::EventBus
        },
        mock_enabled: args.mock,
        ..Default::default()
    };

    let service = VlmInferenceService::new(config)
        .await
        .context("Failed to create inference service")?;

    let temp_dir = tempfile::tempdir()?;
    let mut jobs = Vec::new();

    // Create and process 3 test videos
    for i in 0..3 {
        let test_video = temp_dir.path().join(format!("test_video_{}.mp4", i));
        tokio::fs::write(&test_video, format!("mock video content {}", i)).await?;

        // Register upload
        let upload_id = format!("upload-{}", i);
        service
            .register_upload(UploadInfo {
                upload_id: upload_id.clone(),
                filename: format!("test_video_{}.mp4", i),
                size: 100,
                path: test_video,
                content_type: "video/mp4".to_string(),
            })
            .await;

        // Create job
        let job_id = service
            .create_job(args.model.clone(), format!("Batch test {}: {}", i, args.prompt))
            .await;

        // Run inference
        let job = service.run_inference_sync(&job_id, &upload_id).await?;
        jobs.push(job);
    }

    // Output results
    match args.output {
        OutputFormat::Text => {
            println!("\nBatch Test Results:");
            println!("==================");
            for (i, job) in jobs.iter().enumerate() {
                println!(
                    "\nVideo {}: {} ({} frames)",
                    i + 1,
                    job.status,
                    job.results.len()
                );
                if let Some(first) = job.results.first() {
                    println!("  First result: {}", truncate(&first.text, 60));
                }
            }
        }
        OutputFormat::Json => {
            let output: Vec<JsonOutput> = jobs.iter().map(JsonOutput::from).collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

/// Run simulated live stream test.
async fn run_live_test(args: &Args) -> Result<()> {
    info!("Running live stream simulation...");

    let config = VlmInferenceConfig {
        backend: if args.mock {
            BackendType::Mock
        } else {
            BackendType::EventBus
        },
        mock_enabled: args.mock,
        ..Default::default()
    };

    let service = VlmInferenceService::new(config)
        .await
        .context("Failed to create inference service")?;

    println!("Simulating live stream analysis...");
    println!("(Processing 5 simulated frames)");

    // Simulate 5 frames
    for i in 0..5 {
        let temp_dir = tempfile::tempdir()?;
        let frame_path = temp_dir.path().join(format!("frame_{}.jpg", i));
        tokio::fs::write(&frame_path, format!("mock frame {}", i)).await?;

        let upload_id = format!("frame-{}", i);
        service
            .register_upload(UploadInfo {
                upload_id: upload_id.clone(),
                filename: format!("frame_{}.jpg", i),
                size: 50,
                path: frame_path,
                content_type: "image/jpeg".to_string(),
            })
            .await;

        let job_id = service
            .create_job(args.model.clone(), format!("Live frame {}", i))
            .await;

        let job = service.run_inference_sync(&job_id, &upload_id).await?;

        match args.output {
            OutputFormat::Text => {
                println!(
                    "Frame {}: {} results",
                    i + 1,
                    job.results.len()
                );
            }
            OutputFormat::Json => {
                // Don't output intermediate frames in JSON mode
            }
        }

        // Small delay between frames
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    match args.output {
        OutputFormat::Text => {
            println!("\nLive stream simulation complete.");
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "frames_processed": 5,
                    "mode": "live"
                }))?
            );
        }
    }

    Ok(())
}

/// Run video analysis on a specific file.
async fn run_video_analysis(video_path: &PathBuf, args: &Args) -> Result<()> {
    info!("Analyzing video: {:?}", video_path);

    // Verify file exists
    if !video_path.exists() {
        anyhow::bail!("Video file not found: {:?}", video_path);
    }

    let config = VlmInferenceConfig {
        backend: if args.mock {
            BackendType::Mock
        } else {
            BackendType::EventBus
        },
        mock_enabled: args.mock,
        ..Default::default()
    };

    let service = VlmInferenceService::new(config)
        .await
        .context("Failed to create inference service")?;

    info!("Using backend: {}", service.backend_name());

    // Register the video as an upload
    let upload_id = uuid::Uuid::new_v4().to_string();
    let file_size = tokio::fs::metadata(video_path).await?.len();

    service
        .register_upload(UploadInfo {
            upload_id: upload_id.clone(),
            filename: video_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "video.mp4".to_string()),
            size: file_size,
            path: video_path.clone(),
            content_type: "video/mp4".to_string(),
        })
        .await;

    // Create and run job
    let job_id = service
        .create_job(args.model.clone(), args.prompt.clone())
        .await;

    info!("Created job: {}", job_id);

    let job = service
        .run_inference_sync(&job_id, &upload_id)
        .await
        .context("Inference failed")?;

    // Output results
    if args.synthesize {
        // Synthesize frame results into coherent summary + key moments
        let synthesis = synthesize_results(&job.results, &args.prompt);

        match args.output {
            OutputFormat::Text => {
                print_synthesis_text(&synthesis);
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&synthesis)?);
            }
        }
    } else {
        // Frame-by-frame output (existing behavior)
        match args.output {
            OutputFormat::Text => {
                println!("\nAnalysis Results");
                println!("================");
                println!("Job ID: {}", job.job_id);
                println!("Status: {}", job.status);
                println!("Model: {}", job.model);
                println!("Frames processed: {}", job.results.len());
                println!();

                if job.results.is_empty() {
                    println!("No results generated.");
                } else {
                    println!("Frame-by-frame analysis:");
                    println!("-----------------------");
                    for chunk in &job.results {
                        println!(
                            "\n[Frame {} @ {}ms]",
                            chunk.frame_number, chunk.timestamp_ms
                        );
                        println!("{}", chunk.text);
                    }
                }

                if let Some(error) = &job.error {
                    println!("\nError: {}", error);
                }
            }
            OutputFormat::Json => {
                let output = JsonOutput::from(&job);
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
        }
    }

    if job.status == JobStatus::Completed {
        Ok(())
    } else {
        anyhow::bail!("Job did not complete successfully: {}", job.status)
    }
}

/// Synthesize frame-by-frame results into a coherent summary with key moments.
fn synthesize_results(chunks: &[InferenceChunk], _prompt: &str) -> SynthesizedOutput {
    let frame_count = chunks.len();
    let duration_ms = chunks.last().map(|c| c.timestamp_ms).unwrap_or(0);

    // Identify key moments using heuristics
    let key_moments: Vec<KeyMoment> = chunks
        .iter()
        .filter(|chunk| is_key_moment(&chunk.text))
        .map(|chunk| KeyMoment {
            timestamp_ms: chunk.timestamp_ms,
            timestamp_formatted: format_timestamp(chunk.timestamp_ms),
            description: extract_key_description(&chunk.text),
        })
        .collect();

    // Generate summary from key observations
    let summary = generate_summary(chunks, &key_moments, duration_ms);

    SynthesizedOutput {
        summary,
        key_moments,
        frame_count,
        duration_ms,
    }
}

/// Check if a frame description contains a key moment.
fn is_key_moment(text: &str) -> bool {
    let keywords = [
        "enters",
        "exits",
        "appears",
        "disappears",
        "picks up",
        "puts down",
        "opens",
        "closes",
        "starts",
        "stops",
        "begins",
        "ends",
        "motion detected",
        "movement",
        "activity",
        "walks",
        "runs",
        "stands",
        "sits",
        "leaves",
        "arrives",
        "grabs",
        "reaches",
        "turns",
    ];
    let text_lower = text.to_lowercase();
    keywords.iter().any(|k| text_lower.contains(k))
}

/// Extract a concise description from a frame's text for the key moment.
fn extract_key_description(text: &str) -> String {
    // Find the most relevant sentence containing action keywords
    let action_keywords = [
        "enters", "exits", "picks up", "puts down", "opens", "closes",
        "walks", "runs", "stands", "sits", "leaves", "arrives", "grabs",
        "reaches", "turns", "appears", "disappears", "movement", "activity",
    ];

    // Split into sentences and find the best one
    let sentences: Vec<&str> = text
        .split(|c| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Find sentence with action keyword
    for sentence in &sentences {
        let lower = sentence.to_lowercase();
        if action_keywords.iter().any(|k| lower.contains(k)) {
            // Clean up and return
            let cleaned = sentence
                .trim_start_matches(|c: char| c.is_whitespace() || c == '-')
                .to_string();
            return truncate(&cleaned, 80);
        }
    }

    // Fallback to first sentence if no action found
    sentences
        .first()
        .map(|s| truncate(s, 80))
        .unwrap_or_else(|| truncate(text, 80))
}

/// Format milliseconds as "M:SS" timestamp.
fn format_timestamp(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}

/// Generate a coherent summary from the frame observations.
fn generate_summary(
    chunks: &[InferenceChunk],
    key_moments: &[KeyMoment],
    duration_ms: u64,
) -> String {
    if chunks.is_empty() {
        return "No video content analyzed.".to_string();
    }

    // Calculate duration string
    let duration_secs = duration_ms / 1000;
    let duration_str = if duration_secs >= 60 {
        format!("{}-minute", duration_secs / 60)
    } else {
        format!("{}-second", duration_secs)
    };

    // Extract scene context from first frame
    let scene_context = chunks
        .first()
        .map(|c| extract_scene_context(&c.text))
        .unwrap_or_else(|| "indoor".to_string());

    // Build event list from key moments
    let events: Vec<&str> = key_moments
        .iter()
        .take(3) // Limit to 3 main events
        .map(|km| km.description.as_str())
        .collect();

    // Construct summary
    if events.is_empty() {
        format!(
            "A {} {} video with {} frames analyzed. The scene shows {}.",
            duration_str,
            scene_context,
            chunks.len(),
            extract_general_activity(chunks)
        )
    } else {
        let events_str = if events.len() == 1 {
            events[0].to_string()
        } else if events.len() == 2 {
            format!("{} and {}", events[0].to_lowercase(), events[1].to_lowercase())
        } else {
            let last = events.last().unwrap();
            let rest: Vec<_> = events[..events.len() - 1]
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
            format!("{}, and {}", rest.join(", "), last.to_lowercase())
        };

        format!(
            "A {} {} video showing {}.",
            duration_str, scene_context, events_str
        )
    }
}

/// Extract scene context (indoor/outdoor, setting type) from frame text.
fn extract_scene_context(text: &str) -> String {
    let text_lower = text.to_lowercase();

    if text_lower.contains("outdoor") || text_lower.contains("outside") {
        "outdoor"
    } else if text_lower.contains("indoor") || text_lower.contains("inside") {
        "indoor"
    } else if text_lower.contains("office") {
        "office"
    } else if text_lower.contains("parking") {
        "parking lot"
    } else if text_lower.contains("room") {
        "room"
    } else {
        "indoor" // Default assumption
    }
    .to_string()
}

/// Extract general activity description when no key moments are found.
fn extract_general_activity(chunks: &[InferenceChunk]) -> &str {
    // Look for common activity indicators
    for chunk in chunks {
        let lower = chunk.text.to_lowercase();
        if lower.contains("person") || lower.contains("people") {
            return "people in the scene";
        }
        if lower.contains("vehicle") || lower.contains("car") {
            return "vehicle activity";
        }
        if lower.contains("animal") || lower.contains("pet") {
            return "animal activity";
        }
    }
    "general scene activity"
}

/// Print synthesized output in text format.
fn print_synthesis_text(synthesis: &SynthesizedOutput) {
    println!("\nSummary: {}", synthesis.summary);
    println!();

    if synthesis.key_moments.is_empty() {
        println!("Key moments: None detected");
    } else {
        println!("Key moments:");
        for moment in &synthesis.key_moments {
            println!("- {} - {}", moment.timestamp_formatted, moment.description);
        }
    }
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
