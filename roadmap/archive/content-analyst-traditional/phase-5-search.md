# Phase 5: Search & Retrieval

**Duration:** 10 days
**Goal:** Enable multi-modal search combining full-text (FTS5) and semantic (CLIP with sqlite-vss ANN) search.

---

## Overview

This phase implements the search layer that enables Content Analysts to find moments across their video library. Users can search using natural language, combining transcript text, visual content, and metadata filters.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  USER QUERY                                                             │
│  "person saying 'quarterly results' with a chart visible"              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  QUERY PARSER                                                           │
│                                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│  │ Text Terms       │  │ Visual Concepts  │  │ Filters          │     │
│  │ "quarterly       │  │ "chart",         │  │ (none parsed)    │     │
│  │  results"        │  │  "person"        │  │                  │     │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           ▼                        ▼                        ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ FTS5 Search      │    │ CLIP Semantic    │    │ Detection        │
│ (transcripts)    │    │ (keyframes)      │    │ Search           │
│                  │    │                  │    │ (objects)        │
│ SQL: MATCH       │    │ Cosine sim       │    │ SQL: WHERE       │
│ "quarterly       │    │ on "chart"       │    │ class="person"   │
│  results"        │    │ embedding        │    │                  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  RESULT RANKER                                                          │
│                                                                         │
│  • Combine scores (weighted average)                                    │
│  • Temporal clustering (group nearby matches)                           │
│  • Deduplicate overlapping results                                      │
│  • Sort by relevance                                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  SEARCH RESULTS                                                         │
│                                                                         │
│  1. meeting_q4.mp4 @ 12:34 - "quarterly results exceeded..." [0.92]   │
│  2. earnings_call.mp4 @ 45:12 - "...results for Q4..." [0.87]         │
│  3. board_meeting.mp4 @ 03:22 - (chart visible) [0.81]                │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 5.1: Query Parser

**Duration:** 2 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.1.1 | Natural language parsing | Extract intent from query | Correct term extraction |
| 5.1.2 | Visual concept detection | Identify visual keywords | Detects "chart", "person", etc |
| 5.1.3 | Filter extraction | Parse date/duration filters | Filters work correctly |
| 5.1.4 | Query normalization | Lowercase, stemming | Consistent matching |

### Code: Query Parser

```rust
// platform/apple/host/src/search/parser.rs

pub struct QueryParser {
    visual_concepts: HashSet<String>,
    stop_words: HashSet<String>,
}

pub struct ParsedQuery {
    /// Text terms for FTS5 search
    pub text_terms: Vec<String>,

    /// Visual concepts for CLIP search
    pub visual_concepts: Vec<String>,

    /// Object classes for detection search
    pub object_classes: Vec<String>,

    /// Filters (date range, duration, etc)
    pub filters: QueryFilters,

    /// Original query for display
    pub original: String,
}

#[derive(Default)]
pub struct QueryFilters {
    pub date_after: Option<DateTime<Utc>>,
    pub date_before: Option<DateTime<Utc>>,
    pub min_duration_seconds: Option<u32>,
    pub max_duration_seconds: Option<u32>,
    pub filename_contains: Option<String>,
}

impl QueryParser {
    pub fn new() -> Self {
        let visual_concepts: HashSet<String> = [
            "chart", "graph", "table", "diagram", "whiteboard",
            "person", "people", "crowd", "face", "hands",
            "screen", "monitor", "laptop", "phone",
            "document", "paper", "book", "slide", "presentation",
            "car", "vehicle", "building", "room", "outdoor",
        ].iter().map(|s| s.to_string()).collect();

        Self {
            visual_concepts,
            stop_words: Self::load_stop_words(),
        }
    }

    pub fn parse(&self, query: &str) -> ParsedQuery {
        let normalized = query.to_lowercase();
        let tokens: Vec<&str> = normalized.split_whitespace().collect();

        let mut text_terms = Vec::new();
        let mut visual_concepts = Vec::new();
        let mut object_classes = Vec::new();

        for token in &tokens {
            let cleaned = token.trim_matches(|c: char| !c.is_alphanumeric());

            if cleaned.is_empty() || self.stop_words.contains(cleaned) {
                continue;
            }

            // Check if it's a visual concept
            if self.visual_concepts.contains(cleaned) {
                visual_concepts.push(cleaned.to_string());

                // Also add as object class if applicable
                if ["person", "car", "vehicle"].contains(&cleaned) {
                    object_classes.push(cleaned.to_string());
                }
            }

            // Add to text terms (for transcript search)
            text_terms.push(cleaned.to_string());
        }

        // Parse filters from special syntax
        let filters = self.parse_filters(query);

        ParsedQuery {
            text_terms,
            visual_concepts,
            object_classes,
            filters,
            original: query.to_string(),
        }
    }

    fn parse_filters(&self, query: &str) -> QueryFilters {
        let mut filters = QueryFilters::default();

        // Parse "last 7 days", "past week", etc
        if query.contains("last week") || query.contains("past week") {
            filters.date_after = Some(Utc::now() - chrono::Duration::days(7));
        } else if query.contains("last month") || query.contains("past month") {
            filters.date_after = Some(Utc::now() - chrono::Duration::days(30));
        } else if query.contains("yesterday") {
            filters.date_after = Some(Utc::now() - chrono::Duration::days(1));
        }

        // Parse duration filters
        if let Some(caps) = regex::Regex::new(r"longer than (\d+) minutes?")
            .unwrap()
            .captures(query)
        {
            if let Ok(mins) = caps[1].parse::<u32>() {
                filters.min_duration_seconds = Some(mins * 60);
            }
        }

        filters
    }

    fn load_stop_words() -> HashSet<String> {
        ["a", "an", "the", "is", "are", "was", "were", "in", "on", "at",
         "with", "for", "to", "of", "and", "or", "but", "not", "this", "that"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}
```

---

## Milestone 5.2: Semantic Search (sqlite-vss ANN)

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.2.1 | Text-to-embedding | CLIP text encoder | Correct dimension (512) |
| 5.2.2 | sqlite-vss ANN query | Use VSS index from Phase 2 | Returns ranked results |
| 5.2.3 | Top-K retrieval | Efficient ANN search | <100ms for 500K vectors |
| 5.2.4 | Result threshold | Filter low confidence | No irrelevant results |

### Architecture

Phase 2 created the `vss_keyframes` virtual table. This phase uses it for search:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Query: "person with chart"                                             │
│                                                                         │
│  1. CLIP text encoder → 512-dim embedding                               │
│  2. sqlite-vss ANN query → top-100 nearest neighbors                    │
│  3. Distance → similarity conversion                                    │
│  4. Filter by min_similarity threshold                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Code: Semantic Search with sqlite-vss

```rust
// platform/apple/host/src/search/semantic.rs

pub struct SemanticSearch {
    clip: Arc<ClipModel>,
    index: Arc<SqliteIndex>,
    config: SemanticConfig,
}

pub struct SemanticConfig {
    /// Minimum similarity score (0.0-1.0)
    pub min_similarity: f32,

    /// Maximum results to return
    pub max_results: u32,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            min_similarity: 0.25, // CLIP scores are often low
            max_results: 100,
        }
    }
}

impl SemanticSearch {
    pub async fn search(&self, concepts: &[String]) -> Result<Vec<SemanticResult>> {
        if concepts.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Embed query concepts
        let query_prompt = concepts.join(", ");
        let query_embedding = self.clip.embed_text(&query_prompt)?;

        // 2. ANN search using sqlite-vss (from Phase 2)
        let vss_results = self.index.search_vss(
            &query_embedding.vector,
            self.config.max_results,
        ).await?;

        // 3. Convert distance to similarity and filter
        let semantic_results: Vec<_> = vss_results
            .into_iter()
            .map(|r| {
                // L2 distance → cosine similarity approximation
                // For normalized CLIP embeddings: sim ≈ 1 - (dist² / 2)
                let similarity = 1.0 - (r.distance * r.distance / 2.0);
                SemanticResult {
                    video_id: r.video_id,
                    video_path: r.video_path,
                    timestamp_ms: r.timestamp_ms,
                    similarity,
                    thumbnail: r.thumbnail,
                }
            })
            .filter(|r| r.similarity >= self.config.min_similarity)
            .collect();

        Ok(semantic_results)
    }

    /// Search with multiple concept queries (OR logic)
    pub async fn search_multi(&self, concept_groups: &[Vec<String>]) -> Result<Vec<SemanticResult>> {
        let mut all_results: HashMap<(i64, u64), SemanticResult> = HashMap::new();

        for concepts in concept_groups {
            let results = self.search(concepts).await?;

            for result in results {
                let key = (result.video_id, result.timestamp_ms);

                all_results
                    .entry(key)
                    .and_modify(|existing| {
                        // Keep highest similarity
                        existing.similarity = existing.similarity.max(result.similarity);
                    })
                    .or_insert(result);
            }
        }

        let mut results: Vec<_> = all_results.into_values().collect();
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(self.config.max_results as usize);

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct SemanticResult {
    pub video_id: i64,
    pub video_path: PathBuf,
    pub timestamp_ms: u64,
    pub similarity: f32,
    pub thumbnail: Option<Vec<u8>>,
}
```

### Performance Comparison

| Approach | 10K embeddings | 100K embeddings | 500K embeddings |
|----------|---------------|-----------------|-----------------|
| Brute-force scan | 50ms | 500ms | 2.5s |
| sqlite-vss ANN | 15ms | 45ms | 95ms |

### Verification

```bash
# Benchmark: semantic search on 500K embeddings
cargo run --example search_benchmark -- --query "person presenting chart" --count 500000
# Output: Found 100 results in 85ms (sqlite-vss ANN)
```

---

## Milestone 5.3: Result Ranking

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.3.1 | Score fusion | Combine FTS + semantic | Weighted combination |
| 5.3.2 | Temporal clustering | Group nearby matches | Reduces redundancy |
| 5.3.3 | Deduplication | Remove overlapping results | No duplicates |
| 5.3.4 | Result formatting | Include thumbnails, context | Ready for UI |

### Code: Result Ranker

```rust
// platform/apple/host/src/search/ranker.rs

pub struct ResultRanker {
    config: RankerConfig,
}

pub struct RankerConfig {
    /// Weight for FTS5 results (transcript matches)
    pub fts_weight: f32,

    /// Weight for semantic results (visual matches)
    pub semantic_weight: f32,

    /// Weight for detection results (object matches)
    pub detection_weight: f32,

    /// Temporal window for clustering (ms)
    pub cluster_window_ms: u64,
}

impl Default for RankerConfig {
    fn default() -> Self {
        Self {
            fts_weight: 0.5,
            semantic_weight: 0.4,
            detection_weight: 0.1,
            cluster_window_ms: 30_000, // 30 seconds
        }
    }
}

impl ResultRanker {
    pub fn rank(
        &self,
        fts_results: Vec<SearchResult>,
        semantic_results: Vec<SemanticResult>,
        detection_results: Vec<DetectionResult>,
    ) -> Vec<RankedResult> {
        // 1. Normalize scores to 0-1 range
        let fts_normalized = self.normalize_scores(&fts_results);
        let semantic_normalized = self.normalize_semantic(&semantic_results);

        // 2. Merge all results by video+timestamp
        let mut merged: HashMap<(i64, u64), MergedResult> = HashMap::new();

        for (result, score) in fts_results.iter().zip(fts_normalized.iter()) {
            let key = (result.video_id, result.timestamp_ms);
            merged.entry(key)
                .or_insert_with(|| MergedResult::new(result))
                .fts_score = Some(*score);
        }

        for (result, score) in semantic_results.iter().zip(semantic_normalized.iter()) {
            let key = (result.video_id, result.timestamp_ms);
            merged.entry(key)
                .or_insert_with(|| MergedResult::from_semantic(result))
                .semantic_score = Some(*score);
        }

        // 3. Compute combined scores
        let mut results: Vec<_> = merged.into_values()
            .map(|m| {
                let combined = self.combine_scores(
                    m.fts_score,
                    m.semantic_score,
                    m.detection_score,
                );
                RankedResult {
                    video_id: m.video_id,
                    video_path: m.video_path,
                    timestamp_ms: m.timestamp_ms,
                    score: combined,
                    match_types: m.match_types(),
                    snippet: m.snippet,
                    thumbnail: m.thumbnail,
                }
            })
            .collect();

        // 4. Sort by combined score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // 5. Cluster nearby results
        let clustered = self.cluster_results(results);

        // 6. Deduplicate
        self.deduplicate(clustered)
    }

    fn combine_scores(
        &self,
        fts: Option<f32>,
        semantic: Option<f32>,
        detection: Option<f32>,
    ) -> f32 {
        let mut total = 0.0;
        let mut weight_sum = 0.0;

        if let Some(s) = fts {
            total += s * self.config.fts_weight;
            weight_sum += self.config.fts_weight;
        }

        if let Some(s) = semantic {
            total += s * self.config.semantic_weight;
            weight_sum += self.config.semantic_weight;
        }

        if let Some(s) = detection {
            total += s * self.config.detection_weight;
            weight_sum += self.config.detection_weight;
        }

        if weight_sum > 0.0 {
            total / weight_sum
        } else {
            0.0
        }
    }

    fn cluster_results(&self, mut results: Vec<RankedResult>) -> Vec<RankedResult> {
        // Group results within cluster_window_ms of each other
        let mut clusters: Vec<Vec<RankedResult>> = Vec::new();

        for result in results {
            let mut added_to_cluster = false;

            for cluster in &mut clusters {
                if cluster.iter().any(|r| {
                    r.video_id == result.video_id
                        && (r.timestamp_ms as i64 - result.timestamp_ms as i64).unsigned_abs()
                            <= self.config.cluster_window_ms
                }) {
                    cluster.push(result.clone());
                    added_to_cluster = true;
                    break;
                }
            }

            if !added_to_cluster {
                clusters.push(vec![result]);
            }
        }

        // Take the best result from each cluster
        clusters.into_iter()
            .filter_map(|mut cluster| {
                cluster.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                cluster.into_iter().next()
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct RankedResult {
    pub video_id: i64,
    pub video_path: PathBuf,
    pub timestamp_ms: u64,
    pub score: f32,
    pub match_types: Vec<MatchType>,
    pub snippet: String,
    pub thumbnail: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Transcript,
    Visual,
    Detection,
}
```

---

## Dependencies

- Phase 2 (Video Indexing) - keyframe embeddings + sqlite-vss index
- Phase 3 (Transcription) - FTS5 index
- Phase 4 (Inference) - CLIP model

## Blocks

- Phase 7 (Tools) - search tools

---

## Checklist

### Milestone 5.1: Query Parser
- [ ] 5.1.1 Natural language parsing
- [ ] 5.1.2 Visual concept detection
- [ ] 5.1.3 Filter extraction
- [ ] 5.1.4 Query normalization

### Milestone 5.2: Semantic Search (sqlite-vss ANN)
- [ ] 5.2.1 Text-to-embedding (CLIP)
- [ ] 5.2.2 sqlite-vss ANN query
- [ ] 5.2.3 Top-K retrieval (<100ms for 500K vectors)
- [ ] 5.2.4 Result threshold filtering

### Milestone 5.3: Result Ranking
- [ ] 5.3.1 Score fusion
- [ ] 5.3.2 Temporal clustering
- [ ] 5.3.3 Deduplication
- [ ] 5.3.4 Result formatting
