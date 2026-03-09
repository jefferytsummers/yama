# Phase 8: Export & Reporting

**Duration:** 5 days
**Goal:** Generate structured reports and export analysis results in multiple formats.

---

## Overview

This phase implements export capabilities for the Content Analyst workflow. Users can generate markdown reports with embedded timestamps, export structured data (CSV/JSON), and use templates for consistent output formatting.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ANALYSIS RESULTS                                                       │
│  • Search results with timestamps                                       │
│  • Detection counts and summaries                                       │
│  • VLM-generated descriptions                                           │
│  • Extracted clips metadata                                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  EXPORT ENGINE                                                          │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ Markdown         │  │ JSON             │  │ CSV              │     │
│  │ Generator        │  │ Exporter         │  │ Exporter         │     │
│  │                  │  │                  │  │                  │     │
│  │ • Templates      │  │ • Full data      │  │ • Flat tables    │     │
│  │ • Timestamps     │  │ • Schema         │  │ • Headers        │     │
│  │ • Embedded links │  │ • Pretty/compact │  │ • Quoted fields  │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  OUTPUT FILES                                                           │
│                                                                         │
│  ~/exports/                                                             │
│  ├── report_2024-01-15.md      # Human-readable report                 │
│  ├── detections_2024-01-15.json # Full structured data                 │
│  └── timeline_2024-01-15.csv   # Spreadsheet-compatible                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Export Formats

| Format | Use Case | Features |
|--------|----------|----------|
| Markdown | Reports, documentation | Embedded timestamps, tables, links |
| JSON | Integration, archival | Full fidelity, nested structures |
| CSV | Spreadsheets, analysis | Flat tables, Excel-compatible |

---

## Milestone 8.1: Export Traits

**Duration:** 1 day

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.1.1 | ExportFormat enum | Define supported formats | All formats covered |
| 8.1.2 | Exporter trait | Common export interface | Trait compiles |
| 8.1.3 | ExportConfig | Output path, options | Config validates |

### Code: Export Types

```rust
// platform/apple/host/src/export/types.rs

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
    Json,
    JsonPretty,
    Csv,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Json | ExportFormat::JsonPretty => "json",
            ExportFormat::Csv => "csv",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Markdown => "text/markdown",
            ExportFormat::Json | ExportFormat::JsonPretty => "application/json",
            ExportFormat::Csv => "text/csv",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Output directory
    pub output_dir: PathBuf,

    /// Filename prefix (date will be appended)
    pub filename_prefix: String,

    /// Export format
    pub format: ExportFormat,

    /// Include timestamps in filenames
    pub timestamp_filenames: bool,

    /// For CSV: include header row
    pub csv_headers: bool,

    /// For JSON: indent with spaces
    pub json_indent: Option<usize>,

    /// For Markdown: template name
    pub template: Option<String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./exports"),
            filename_prefix: "export".to_string(),
            format: ExportFormat::Markdown,
            timestamp_filenames: true,
            csv_headers: true,
            json_indent: Some(2),
            template: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    /// Path to exported file
    pub path: PathBuf,

    /// File size in bytes
    pub size_bytes: u64,

    /// Number of records exported
    pub record_count: usize,

    /// Export duration
    pub duration_ms: u64,
}

/// Common trait for exportable data
pub trait Exportable: Send + Sync {
    /// Export to the specified format
    fn export(&self, config: &ExportConfig) -> Result<ExportResult>;

    /// Supported formats for this data type
    fn supported_formats(&self) -> Vec<ExportFormat>;
}
```

---

## Milestone 8.2: Markdown Report Generator

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.2.1 | Template system | Load and render templates | Templates render |
| 8.2.2 | Timestamp formatting | HH:MM:SS.ms format | Clickable timestamps |
| 8.2.3 | Table generation | Detection/search tables | Aligned columns |
| 8.2.4 | Video links | Deep links to timestamps | Links work |

### Code: Markdown Generator

```rust
// platform/apple/host/src/export/markdown.rs

use std::fmt::Write;
use chrono::{DateTime, Utc};

pub struct MarkdownGenerator {
    templates: HashMap<String, String>,
}

impl MarkdownGenerator {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Default report template
        templates.insert("report".to_string(), include_str!("templates/report.md").to_string());
        templates.insert("timeline".to_string(), include_str!("templates/timeline.md").to_string());
        templates.insert("summary".to_string(), include_str!("templates/summary.md").to_string());

        Self { templates }
    }

    pub fn generate_report(&self, data: &ReportData) -> Result<String> {
        let mut md = String::new();

        // Header
        writeln!(md, "# {}", data.title)?;
        writeln!(md)?;
        writeln!(md, "**Generated:** {}", data.generated_at.format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(md, "**Query:** {}", data.query)?;
        writeln!(md)?;

        // Summary statistics
        writeln!(md, "## Summary")?;
        writeln!(md)?;
        writeln!(md, "| Metric | Value |")?;
        writeln!(md, "|--------|-------|")?;
        writeln!(md, "| Videos searched | {} |", data.videos_searched)?;
        writeln!(md, "| Total matches | {} |", data.total_matches)?;
        writeln!(md, "| Time range | {} - {} |",
            Self::format_duration(data.time_range.0),
            Self::format_duration(data.time_range.1))?;
        writeln!(md)?;

        // Results table
        writeln!(md, "## Results")?;
        writeln!(md)?;
        writeln!(md, "| Video | Timestamp | Match | Confidence |")?;
        writeln!(md, "|-------|-----------|-------|------------|")?;

        for result in &data.results {
            writeln!(md, "| {} | {} | {} | {:.1}% |",
                result.video_name,
                Self::format_timestamp_link(&result.video_path, result.timestamp_ms),
                Self::truncate(&result.description, 50),
                result.confidence * 100.0)?;
        }
        writeln!(md)?;

        // Detailed findings
        if !data.results.is_empty() {
            writeln!(md, "## Detailed Findings")?;
            writeln!(md)?;

            for (i, result) in data.results.iter().enumerate() {
                writeln!(md, "### {}. {} at {}",
                    i + 1,
                    result.video_name,
                    Self::format_timestamp(result.timestamp_ms))?;
                writeln!(md)?;

                if let Some(ref frame_path) = result.frame_path {
                    writeln!(md, "![Frame]({})", frame_path.display())?;
                    writeln!(md)?;
                }

                writeln!(md, "{}", result.description)?;
                writeln!(md)?;

                if !result.detections.is_empty() {
                    writeln!(md, "**Detections:**")?;
                    for det in &result.detections {
                        writeln!(md, "- {} ({:.1}%)", det.label, det.confidence * 100.0)?;
                    }
                    writeln!(md)?;
                }
            }
        }

        // Export metadata
        writeln!(md, "---")?;
        writeln!(md)?;
        writeln!(md, "*Report generated by Yama Content Analyst*")?;

        Ok(md)
    }

    pub fn format_timestamp(ms: u64) -> String {
        let hours = ms / 3_600_000;
        let minutes = (ms % 3_600_000) / 60_000;
        let seconds = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
        } else {
            format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
        }
    }

    pub fn format_timestamp_link(video_path: &Path, ms: u64) -> String {
        let ts = Self::format_timestamp(ms);
        // Format: [HH:MM:SS.mmm](file:///path/to/video.mp4#t=seconds)
        let seconds = ms as f64 / 1000.0;
        format!("[{}](file://{}#t={:.3})", ts, video_path.display(), seconds)
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    fn format_duration(ms: u64) -> String {
        Self::format_timestamp(ms)
    }
}

#[derive(Debug, Clone)]
pub struct ReportData {
    pub title: String,
    pub query: String,
    pub generated_at: DateTime<Utc>,
    pub videos_searched: usize,
    pub total_matches: usize,
    pub time_range: (u64, u64),
    pub results: Vec<ReportResult>,
}

#[derive(Debug, Clone)]
pub struct ReportResult {
    pub video_name: String,
    pub video_path: PathBuf,
    pub timestamp_ms: u64,
    pub description: String,
    pub confidence: f32,
    pub frame_path: Option<PathBuf>,
    pub detections: Vec<Detection>,
}
```

### Template: report.md

```markdown
# {{ title }}

**Generated:** {{ generated_at }}
**Query:** {{ query }}

## Summary

| Metric | Value |
|--------|-------|
| Videos searched | {{ videos_searched }} |
| Total matches | {{ total_matches }} |
| Time range | {{ time_range_start }} - {{ time_range_end }} |

## Results

| Video | Timestamp | Match | Confidence |
|-------|-----------|-------|------------|
{% for result in results %}
| {{ result.video_name }} | {{ result.timestamp_link }} | {{ result.description | truncate(50) }} | {{ result.confidence }}% |
{% endfor %}

{% if results %}
## Detailed Findings

{% for result in results %}
### {{ loop.index }}. {{ result.video_name }} at {{ result.timestamp }}

{% if result.frame_path %}
![Frame]({{ result.frame_path }})

{% endif %}
{{ result.description }}

{% if result.detections %}
**Detections:**
{% for det in result.detections %}
- {{ det.label }} ({{ det.confidence }}%)
{% endfor %}
{% endif %}

{% endfor %}
{% endif %}

---

*Report generated by Yama Content Analyst*
```

---

## Milestone 8.3: JSON/CSV Exporters

**Duration:** 1 day

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.3.1 | JSON serialization | Serde-based export | Valid JSON output |
| 8.3.2 | CSV writer | Header + rows | Excel-compatible |
| 8.3.3 | Field mapping | Convert types to strings | All fields handled |

### Code: JSON Exporter

```rust
// platform/apple/host/src/export/json.rs

use serde::Serialize;
use std::io::Write;

pub struct JsonExporter;

impl JsonExporter {
    pub fn export<T: Serialize>(
        data: &T,
        config: &ExportConfig,
    ) -> Result<ExportResult> {
        let start = Instant::now();

        // Generate filename
        let filename = Self::generate_filename(config);
        let path = config.output_dir.join(&filename);

        // Ensure directory exists
        std::fs::create_dir_all(&config.output_dir)?;

        // Serialize
        let json = match config.json_indent {
            Some(indent) => {
                let formatter = serde_json::ser::PrettyFormatter::with_indent(
                    &" ".repeat(indent).as_bytes()
                );
                let mut buf = Vec::new();
                let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                data.serialize(&mut ser)?;
                String::from_utf8(buf)?
            }
            None => serde_json::to_string(data)?,
        };

        // Write file
        let mut file = std::fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;

        let size_bytes = json.len() as u64;

        Ok(ExportResult {
            path,
            size_bytes,
            record_count: 1, // JSON exports single root object
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn generate_filename(config: &ExportConfig) -> String {
        if config.timestamp_filenames {
            let now = chrono::Utc::now();
            format!(
                "{}_{}.{}",
                config.filename_prefix,
                now.format("%Y-%m-%d_%H%M%S"),
                config.format.extension()
            )
        } else {
            format!("{}.{}", config.filename_prefix, config.format.extension())
        }
    }
}

/// Structured export schema for analysis results
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisExport {
    /// Export metadata
    pub metadata: ExportMetadata,

    /// Search/analysis parameters
    pub query: QueryParams,

    /// Results
    pub results: Vec<ResultRecord>,

    /// Statistics
    pub statistics: Statistics,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportMetadata {
    pub version: String,
    pub generated_at: String,
    pub generator: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryParams {
    pub text: String,
    pub video_filter: Option<String>,
    pub time_range: Option<TimeRange>,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultRecord {
    pub video_id: i64,
    pub video_path: String,
    pub timestamp_ms: u64,
    pub timestamp_formatted: String,
    pub match_type: String,
    pub description: String,
    pub confidence: f32,
    pub detections: Vec<DetectionRecord>,
    pub transcript_snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectionRecord {
    pub label: String,
    pub confidence: f32,
    pub bbox: Option<BoundingBox>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Statistics {
    pub total_videos: usize,
    pub total_matches: usize,
    pub unique_labels: Vec<String>,
    pub confidence_distribution: ConfidenceDistribution,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidenceDistribution {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub median: f32,
}
```

### Code: CSV Exporter

```rust
// platform/apple/host/src/export/csv.rs

use csv::Writer;
use std::io::Write;

pub struct CsvExporter;

impl CsvExporter {
    pub fn export_results(
        results: &[ResultRecord],
        config: &ExportConfig,
    ) -> Result<ExportResult> {
        let start = Instant::now();

        let filename = Self::generate_filename(config);
        let path = config.output_dir.join(&filename);

        std::fs::create_dir_all(&config.output_dir)?;

        let file = std::fs::File::create(&path)?;
        let mut wtr = Writer::from_writer(file);

        // Write header if enabled
        if config.csv_headers {
            wtr.write_record(&[
                "video_id",
                "video_path",
                "timestamp_ms",
                "timestamp",
                "match_type",
                "description",
                "confidence",
                "detections",
                "transcript",
            ])?;
        }

        // Write rows
        for result in results {
            let detections_str = result.detections
                .iter()
                .map(|d| format!("{}:{:.2}", d.label, d.confidence))
                .collect::<Vec<_>>()
                .join("; ");

            wtr.write_record(&[
                result.video_id.to_string(),
                result.video_path.clone(),
                result.timestamp_ms.to_string(),
                result.timestamp_formatted.clone(),
                result.match_type.clone(),
                Self::escape_csv(&result.description),
                format!("{:.3}", result.confidence),
                detections_str,
                result.transcript_snippet.clone().unwrap_or_default(),
            ])?;
        }

        wtr.flush()?;

        let metadata = std::fs::metadata(&path)?;

        Ok(ExportResult {
            path,
            size_bytes: metadata.len(),
            record_count: results.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Export detection counts to CSV
    pub fn export_counts(
        counts: &[(String, usize)],
        config: &ExportConfig,
    ) -> Result<ExportResult> {
        let start = Instant::now();

        let filename = Self::generate_filename(config);
        let path = config.output_dir.join(&filename);

        std::fs::create_dir_all(&config.output_dir)?;

        let file = std::fs::File::create(&path)?;
        let mut wtr = Writer::from_writer(file);

        if config.csv_headers {
            wtr.write_record(&["label", "count"])?;
        }

        for (label, count) in counts {
            wtr.write_record(&[label.as_str(), &count.to_string()])?;
        }

        wtr.flush()?;

        let metadata = std::fs::metadata(&path)?;

        Ok(ExportResult {
            path,
            size_bytes: metadata.len(),
            record_count: counts.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn escape_csv(s: &str) -> String {
        // CSV handles quoting automatically, but we remove newlines
        s.replace('\n', " ").replace('\r', "")
    }

    fn generate_filename(config: &ExportConfig) -> String {
        if config.timestamp_filenames {
            let now = chrono::Utc::now();
            format!(
                "{}_{}.csv",
                config.filename_prefix,
                now.format("%Y-%m-%d_%H%M%S")
            )
        } else {
            format!("{}.csv", config.filename_prefix)
        }
    }
}
```

---

## Milestone 8.4: Export Pipeline

**Duration:** 1 day

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 8.4.1 | Pipeline orchestration | Coordinate export steps | All formats work |
| 8.4.2 | Progress reporting | Export progress callback | UI updates |
| 8.4.3 | Batch export | Export multiple formats | All files created |
| 8.4.4 | Error recovery | Handle partial failures | Partial output saved |

### Code: Export Pipeline

```rust
// platform/apple/host/src/export/pipeline.rs

pub struct ExportPipeline {
    markdown_gen: MarkdownGenerator,
    output_dir: PathBuf,
}

impl ExportPipeline {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            markdown_gen: MarkdownGenerator::new(),
            output_dir,
        }
    }

    /// Export analysis results to multiple formats
    pub async fn export_analysis(
        &self,
        results: &AnalysisResults,
        formats: &[ExportFormat],
        on_progress: impl Fn(ExportProgress),
    ) -> Result<Vec<ExportResult>> {
        let mut exports = Vec::new();
        let total = formats.len();

        for (i, format) in formats.iter().enumerate() {
            on_progress(ExportProgress {
                format: *format,
                current: i + 1,
                total,
                status: ExportStatus::InProgress,
            });

            let config = ExportConfig {
                output_dir: self.output_dir.clone(),
                filename_prefix: "analysis".to_string(),
                format: *format,
                ..Default::default()
            };

            let result = match format {
                ExportFormat::Markdown => {
                    let report_data = self.build_report_data(results);
                    let md = self.markdown_gen.generate_report(&report_data)?;
                    self.write_file(&config, md.as_bytes())?
                }
                ExportFormat::Json | ExportFormat::JsonPretty => {
                    let export_data = self.build_export_data(results);
                    JsonExporter::export(&export_data, &config)?
                }
                ExportFormat::Csv => {
                    let records = self.build_result_records(results);
                    CsvExporter::export_results(&records, &config)?
                }
            };

            exports.push(result);

            on_progress(ExportProgress {
                format: *format,
                current: i + 1,
                total,
                status: ExportStatus::Complete,
            });
        }

        Ok(exports)
    }

    /// Export clip extraction manifest
    pub async fn export_clip_manifest(
        &self,
        clips: &[ClipInfo],
        format: ExportFormat,
    ) -> Result<ExportResult> {
        let config = ExportConfig {
            output_dir: self.output_dir.clone(),
            filename_prefix: "clips_manifest".to_string(),
            format,
            ..Default::default()
        };

        match format {
            ExportFormat::Json | ExportFormat::JsonPretty => {
                JsonExporter::export(clips, &config)
            }
            ExportFormat::Csv => {
                let records: Vec<ClipRecord> = clips.iter().map(|c| ClipRecord {
                    source_video: c.source_path.display().to_string(),
                    output_file: c.output_path.display().to_string(),
                    start_ms: c.start_ms,
                    end_ms: c.end_ms,
                    duration_ms: c.end_ms - c.start_ms,
                    reason: c.reason.clone(),
                }).collect();

                CsvExporter::export_clip_records(&records, &config)
            }
            ExportFormat::Markdown => {
                let md = self.generate_clip_manifest_md(clips);
                self.write_file(&config, md.as_bytes())
            }
        }
    }

    fn write_file(&self, config: &ExportConfig, data: &[u8]) -> Result<ExportResult> {
        let start = Instant::now();

        let filename = format!(
            "{}_{}.{}",
            config.filename_prefix,
            chrono::Utc::now().format("%Y-%m-%d_%H%M%S"),
            config.format.extension()
        );
        let path = config.output_dir.join(&filename);

        std::fs::create_dir_all(&config.output_dir)?;
        std::fs::write(&path, data)?;

        Ok(ExportResult {
            path,
            size_bytes: data.len() as u64,
            record_count: 1,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn build_report_data(&self, results: &AnalysisResults) -> ReportData {
        ReportData {
            title: format!("Analysis Report: {}", results.query),
            query: results.query.clone(),
            generated_at: chrono::Utc::now(),
            videos_searched: results.videos_searched,
            total_matches: results.matches.len(),
            time_range: results.time_range,
            results: results.matches.iter().map(|m| ReportResult {
                video_name: m.video_name.clone(),
                video_path: m.video_path.clone(),
                timestamp_ms: m.timestamp_ms,
                description: m.description.clone(),
                confidence: m.confidence,
                frame_path: m.frame_path.clone(),
                detections: m.detections.clone(),
            }).collect(),
        }
    }

    fn build_export_data(&self, results: &AnalysisResults) -> AnalysisExport {
        AnalysisExport {
            metadata: ExportMetadata {
                version: "1.0".to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                generator: "Yama Content Analyst".to_string(),
            },
            query: QueryParams {
                text: results.query.clone(),
                video_filter: results.video_filter.clone(),
                time_range: results.time_range_filter.map(|(s, e)| TimeRange {
                    start_ms: s,
                    end_ms: e,
                }),
                confidence_threshold: results.confidence_threshold,
            },
            results: self.build_result_records(results),
            statistics: self.calculate_statistics(results),
        }
    }

    fn build_result_records(&self, results: &AnalysisResults) -> Vec<ResultRecord> {
        results.matches.iter().map(|m| ResultRecord {
            video_id: m.video_id,
            video_path: m.video_path.display().to_string(),
            timestamp_ms: m.timestamp_ms,
            timestamp_formatted: MarkdownGenerator::format_timestamp(m.timestamp_ms),
            match_type: m.match_type.clone(),
            description: m.description.clone(),
            confidence: m.confidence,
            detections: m.detections.iter().map(|d| DetectionRecord {
                label: d.label.clone(),
                confidence: d.confidence,
                bbox: d.bbox.map(|b| BoundingBox {
                    x: b.x,
                    y: b.y,
                    width: b.width,
                    height: b.height,
                }),
            }).collect(),
            transcript_snippet: m.transcript_snippet.clone(),
        }).collect()
    }

    fn calculate_statistics(&self, results: &AnalysisResults) -> Statistics {
        let confidences: Vec<f32> = results.matches.iter()
            .map(|m| m.confidence)
            .collect();

        let mut labels: HashSet<String> = HashSet::new();
        for m in &results.matches {
            for d in &m.detections {
                labels.insert(d.label.clone());
            }
        }

        let (min, max, mean, median) = if confidences.is_empty() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let mut sorted = confidences.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let min = *sorted.first().unwrap();
            let max = *sorted.last().unwrap();
            let mean = sorted.iter().sum::<f32>() / sorted.len() as f32;
            let median = sorted[sorted.len() / 2];

            (min, max, mean, median)
        };

        Statistics {
            total_videos: results.videos_searched,
            total_matches: results.matches.len(),
            unique_labels: labels.into_iter().collect(),
            confidence_distribution: ConfidenceDistribution { min, max, mean, median },
        }
    }

    fn generate_clip_manifest_md(&self, clips: &[ClipInfo]) -> String {
        let mut md = String::new();

        writeln!(md, "# Clip Extraction Manifest").unwrap();
        writeln!(md).unwrap();
        writeln!(md, "**Generated:** {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")).unwrap();
        writeln!(md, "**Total clips:** {}", clips.len()).unwrap();
        writeln!(md).unwrap();

        writeln!(md, "## Clips").unwrap();
        writeln!(md).unwrap();
        writeln!(md, "| # | Source | Start | End | Duration | Output |").unwrap();
        writeln!(md, "|---|--------|-------|-----|----------|--------|").unwrap();

        for (i, clip) in clips.iter().enumerate() {
            writeln!(md, "| {} | {} | {} | {} | {} | {} |",
                i + 1,
                clip.source_path.file_name().unwrap_or_default().to_string_lossy(),
                MarkdownGenerator::format_timestamp(clip.start_ms),
                MarkdownGenerator::format_timestamp(clip.end_ms),
                MarkdownGenerator::format_timestamp(clip.end_ms - clip.start_ms),
                clip.output_path.file_name().unwrap_or_default().to_string_lossy(),
            ).unwrap();
        }

        md
    }
}

#[derive(Debug, Clone)]
pub struct ExportProgress {
    pub format: ExportFormat,
    pub current: usize,
    pub total: usize,
    pub status: ExportStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportStatus {
    InProgress,
    Complete,
    Failed,
}
```

---

## Dependencies

- Phase 5 (Search) - provides search results
- Phase 6 (Clip Extraction) - provides clip manifests
- Phase 7 (Tools) - `generate_summary` tool uses export

## Blocks

- None (final phase)

---

## Checklist

### Milestone 8.1: Export Traits
- [ ] 8.1.1 ExportFormat enum
- [ ] 8.1.2 Exporter trait
- [ ] 8.1.3 ExportConfig

### Milestone 8.2: Markdown Report Generator
- [ ] 8.2.1 Template system
- [ ] 8.2.2 Timestamp formatting
- [ ] 8.2.3 Table generation
- [ ] 8.2.4 Video links

### Milestone 8.3: JSON/CSV Exporters
- [ ] 8.3.1 JSON serialization
- [ ] 8.3.2 CSV writer
- [ ] 8.3.3 Field mapping

### Milestone 8.4: Export Pipeline
- [ ] 8.4.1 Pipeline orchestration
- [ ] 8.4.2 Progress reporting
- [ ] 8.4.3 Batch export
- [ ] 8.4.4 Error recovery
