//! Get keyframes tool.

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ParameterDef, Tool, ToolContext, ToolDefinition, ToolResult};

/// Keyframe information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeInfo {
    /// Keyframe ID.
    pub id: String,
    /// Video ID.
    pub video_id: String,
    /// Timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Frame number.
    pub frame_number: i64,
    /// Path to the keyframe image.
    pub image_path: String,
    /// Image width.
    pub width: i32,
    /// Image height.
    pub height: i32,
    /// Optional embedding ID if indexed.
    pub embedding_id: Option<String>,
}

/// Get keyframes tool.
pub struct GetKeyframesTool;

impl GetKeyframesTool {
    /// Create a new get keyframes tool.
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetKeyframesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GetKeyframesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "get_keyframes",
            "Retrieve keyframes from a video. Returns frame images and metadata for visual analysis.",
        )
        .with_param(ParameterDef::required_string(
            "video_id",
            "ID of the video to get keyframes from",
        ))
        .with_param(ParameterDef::optional_number(
            "start_ms",
            "Start time in milliseconds (default: beginning)",
            0.0,
        ))
        .with_param(ParameterDef::optional_number(
            "end_ms",
            "End time in milliseconds (default: end of video)",
            -1.0,
        ))
        .with_param(ParameterDef::optional_integer(
            "limit",
            "Maximum number of keyframes to return",
            10,
        ))
        .with_param(ParameterDef {
            name: "strategy".to_string(),
            param_type: "string".to_string(),
            description: "Keyframe selection strategy".to_string(),
            required: false,
            default: Some(Value::String("interval".to_string())),
            enum_values: Some(vec![
                Value::String("interval".to_string()),
                Value::String("scene_change".to_string()),
                Value::String("uniform".to_string()),
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
                .unwrap_or(0);

            let end_ms = input.get("end_ms").and_then(|v| v.as_i64()).unwrap_or(-1);

            let limit = input
                .get("limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as usize;

            let strategy = input
                .get("strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("interval");

            // TODO: Implement actual keyframe retrieval from database
            // For now, return a placeholder result
            let keyframes: Vec<KeyframeInfo> = Vec::new();

            let elapsed = start.elapsed().as_millis() as u64;

            Ok(ToolResult::ok(serde_json::json!({
                "video_id": video_id,
                "start_ms": start_ms,
                "end_ms": end_ms,
                "strategy": strategy,
                "keyframes": keyframes,
                "count": keyframes.len(),
                "limit": limit,
            }))
            .with_time(elapsed))
        })
    }
}
