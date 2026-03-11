//! Compare frames tool.

use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ParameterDef, Tool, ToolContext, ToolDefinition, ToolResult};

/// Frame comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameComparison {
    /// First frame ID or timestamp.
    pub frame_a: String,
    /// Second frame ID or timestamp.
    pub frame_b: String,
    /// Overall similarity score (0.0 - 1.0).
    pub similarity: f32,
    /// Detected differences.
    pub differences: Vec<Difference>,
    /// Comparison method used.
    pub method: String,
}

/// A detected difference between frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Type of difference (e.g., "object_added", "object_removed", "motion", "lighting").
    pub diff_type: String,
    /// Description of the difference.
    pub description: String,
    /// Bounding box [x, y, width, height] if applicable.
    pub bounding_box: Option<[f32; 4]>,
    /// Confidence score.
    pub confidence: f32,
}

/// Compare frames tool.
pub struct CompareFramesTool;

impl CompareFramesTool {
    /// Create a new compare frames tool.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CompareFramesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for CompareFramesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "compare_frames",
            "Compare two video frames to identify differences. Useful for detecting changes, motion, or anomalies.",
        )
        .with_param(ParameterDef::required_string(
            "video_id",
            "ID of the video (both frames must be from the same video)",
        ))
        .with_param(ParameterDef::required_number(
            "timestamp_a_ms",
            "Timestamp of first frame in milliseconds",
        ))
        .with_param(ParameterDef::required_number(
            "timestamp_b_ms",
            "Timestamp of second frame in milliseconds",
        ))
        .with_param(ParameterDef {
            name: "method".to_string(),
            param_type: "string".to_string(),
            description: "Comparison method to use".to_string(),
            required: false,
            default: Some(Value::String("embedding".to_string())),
            enum_values: Some(vec![
                Value::String("embedding".to_string()),
                Value::String("pixel".to_string()),
                Value::String("structural".to_string()),
            ]),
        })
        .with_param(ParameterDef::optional_bool(
            "detect_objects",
            "Whether to detect and compare objects",
            true,
        ))
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

            let timestamp_a = input
                .get("timestamp_a_ms")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: timestamp_a_ms"))?;

            let timestamp_b = input
                .get("timestamp_b_ms")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: timestamp_b_ms"))?;

            let method = input
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("embedding");

            let detect_objects = input
                .get("detect_objects")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            // TODO: Implement actual frame comparison
            // For now, return a placeholder result
            let comparison = FrameComparison {
                frame_a: format!("{}:{}", video_id, timestamp_a),
                frame_b: format!("{}:{}", video_id, timestamp_b),
                similarity: 0.85,
                differences: vec![],
                method: method.to_string(),
            };

            let elapsed = start.elapsed().as_millis() as u64;

            Ok(ToolResult::ok(comparison).with_time(elapsed))
        })
    }
}
