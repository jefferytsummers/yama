//! Extract clip tool.

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ParameterDef, Tool, ToolContext, ToolDefinition, ToolResult};

/// Extracted clip information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedClip {
    /// Clip ID.
    pub clip_id: String,
    /// Source video ID.
    pub video_id: String,
    /// Start time in milliseconds.
    pub start_ms: i64,
    /// End time in milliseconds.
    pub end_ms: i64,
    /// Duration in milliseconds.
    pub duration_ms: i64,
    /// Output file path.
    pub output_path: String,
    /// File size in bytes.
    pub file_size_bytes: i64,
}

/// Extract clip tool.
pub struct ExtractClipTool;

impl ExtractClipTool {
    /// Create a new extract clip tool.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExtractClipTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ExtractClipTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "extract_clip",
            "Extract a video clip from a source video. Creates a new video file with the specified time range.",
        )
        .with_param(ParameterDef::required_string(
            "video_id",
            "ID of the source video",
        ))
        .with_param(ParameterDef::required_number(
            "start_ms",
            "Start time in milliseconds",
        ))
        .with_param(ParameterDef::required_number(
            "end_ms",
            "End time in milliseconds",
        ))
        .with_param(ParameterDef::optional_string(
            "output_name",
            "Optional name for the output clip file",
        ))
        .with_param(ParameterDef {
            name: "format".to_string(),
            param_type: "string".to_string(),
            description: "Output format".to_string(),
            required: false,
            default: Some(Value::String("mp4".to_string())),
            enum_values: Some(vec![
                Value::String("mp4".to_string()),
                Value::String("webm".to_string()),
                Value::String("gif".to_string()),
            ]),
        })
    }

    fn execute(
        &self,
        input: Value,
        ctx: ToolContext,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult>> + Send + '_>> {
        Box::pin(async move {
            let start = std::time::Instant::now();

            // Parse input
            let video_id = input
                .get("video_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: video_id"))?;

            let start_ms = input
                .get("start_ms")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: start_ms"))?;

            let end_ms = input
                .get("end_ms")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: end_ms"))?;

            let format = input
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("mp4");

            // Validate time range
            if start_ms >= end_ms {
                return Ok(ToolResult::err("start_ms must be less than end_ms"));
            }

            if start_ms < 0 {
                return Ok(ToolResult::err("start_ms cannot be negative"));
            }

            // TODO: Implement actual clip extraction using GStreamer
            // For now, return a placeholder result
            let clip_id = uuid::Uuid::new_v4().to_string();

            let clip = ExtractedClip {
                clip_id: clip_id.clone(),
                video_id: video_id.to_string(),
                start_ms,
                end_ms,
                duration_ms: end_ms - start_ms,
                output_path: format!("~/.yama/artifacts/{}.{}", clip_id, format),
                file_size_bytes: 0, // Would be calculated after extraction
            };

            let elapsed = start.elapsed().as_millis() as u64;

            Ok(ToolResult::ok(clip).with_time(elapsed))
        })
    }
}
