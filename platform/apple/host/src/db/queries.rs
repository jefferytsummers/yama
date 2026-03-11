//! Database queries for Yama.
//!
//! CRUD operations for all database tables.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::models::*;

// ============================================================================
// Project Queries
// ============================================================================

/// Create a new project.
pub async fn create_project(pool: &SqlitePool, input: NewProject) -> Result<Project> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = input.config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());

    let row = sqlx::query_as::<_, ProjectRow>(
        r#"
        INSERT INTO projects (id, name, description, config, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&config_json)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create project")?;

    Ok(row.into_project())
}

/// Get a project by ID.
pub async fn get_project(pool: &SqlitePool, id: &str) -> Result<Option<Project>> {
    let row = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get project")?;

    Ok(row.map(|r| r.into_project()))
}

/// List all projects.
pub async fn list_projects(pool: &SqlitePool) -> Result<Vec<Project>> {
    let rows = sqlx::query_as::<_, ProjectRow>("SELECT * FROM projects ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list projects")?;

    Ok(rows.into_iter().map(|r| r.into_project()).collect())
}

/// Update a project.
pub async fn update_project(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<Option<&str>>,
    config: Option<Option<&ProjectConfig>>,
) -> Result<Option<Project>> {
    let now = chrono::Utc::now().to_rfc3339();

    // Build update query dynamically
    let mut updates = vec!["updated_at = ?"];
    let mut binds: Vec<Option<String>> = vec![Some(now.clone())];

    if let Some(n) = name {
        updates.push("name = ?");
        binds.push(Some(n.to_string()));
    }
    if let Some(d) = description {
        updates.push("description = ?");
        binds.push(d.map(|s| s.to_string()));
    }
    if let Some(c) = config {
        updates.push("config = ?");
        binds.push(c.map(|cfg| serde_json::to_string(cfg).unwrap_or_default()));
    }

    let query = format!(
        "UPDATE projects SET {} WHERE id = ? RETURNING *",
        updates.join(", ")
    );

    let mut q = sqlx::query_as::<_, ProjectRow>(&query);
    for b in &binds {
        q = q.bind(b);
    }
    q = q.bind(id);

    let row = q.fetch_optional(pool)
        .await
        .context("Failed to update project")?;

    Ok(row.map(|r| r.into_project()))
}

/// Delete a project (cascades to libraries, videos, etc.).
pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete project")?;

    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Library Queries
// ============================================================================

/// Create a new library.
pub async fn create_library(pool: &SqlitePool, input: NewLibrary) -> Result<Library> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query_as::<_, Library>(
        r#"
        INSERT INTO libraries (id, project_id, name, root_path, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.project_id)
    .bind(&input.name)
    .bind(&input.root_path)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create library")
}

/// Get a library by ID.
pub async fn get_library(pool: &SqlitePool, id: &str) -> Result<Option<Library>> {
    sqlx::query_as::<_, Library>("SELECT * FROM libraries WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get library")
}

/// List libraries for a project.
pub async fn list_libraries(pool: &SqlitePool, project_id: &str) -> Result<Vec<Library>> {
    sqlx::query_as::<_, Library>(
        "SELECT * FROM libraries WHERE project_id = ? ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .context("Failed to list libraries")
}

/// Update library's last indexed timestamp.
pub async fn update_library_indexed(pool: &SqlitePool, id: &str) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE libraries SET last_indexed_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update library indexed time")?;

    Ok(result.rows_affected() > 0)
}

/// Delete a library.
pub async fn delete_library(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM libraries WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete library")?;

    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Video Queries
// ============================================================================

/// Create a new video.
pub async fn create_video(pool: &SqlitePool, input: NewVideo) -> Result<Video> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query_as::<_, Video>(
        r#"
        INSERT INTO videos (id, library_id, path, filename, duration_ms, width, height, fps, codec, container, file_size_bytes, file_hash, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.library_id)
    .bind(&input.path)
    .bind(&input.filename)
    .bind(input.duration_ms)
    .bind(input.width)
    .bind(input.height)
    .bind(input.fps)
    .bind(&input.codec)
    .bind(&input.container)
    .bind(input.file_size_bytes)
    .bind(&input.file_hash)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create video")
}

/// Get a video by ID.
pub async fn get_video(pool: &SqlitePool, id: &str) -> Result<Option<Video>> {
    sqlx::query_as::<_, Video>("SELECT * FROM videos WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get video")
}

/// Get a video by file hash.
pub async fn get_video_by_hash(pool: &SqlitePool, hash: &str) -> Result<Option<Video>> {
    sqlx::query_as::<_, Video>("SELECT * FROM videos WHERE file_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
        .context("Failed to get video by hash")
}

/// Get a video by path within a library.
pub async fn get_video_by_path(
    pool: &SqlitePool,
    library_id: &str,
    path: &str,
) -> Result<Option<Video>> {
    sqlx::query_as::<_, Video>("SELECT * FROM videos WHERE library_id = ? AND path = ?")
        .bind(library_id)
        .bind(path)
        .fetch_optional(pool)
        .await
        .context("Failed to get video by path")
}

/// List videos in a library.
pub async fn list_videos(pool: &SqlitePool, library_id: &str) -> Result<Vec<Video>> {
    sqlx::query_as::<_, Video>(
        "SELECT * FROM videos WHERE library_id = ? ORDER BY filename ASC",
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
    .context("Failed to list videos")
}

/// Count videos in a library.
pub async fn count_videos(pool: &SqlitePool, library_id: &str) -> Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM videos WHERE library_id = ?")
        .bind(library_id)
        .fetch_one(pool)
        .await
        .context("Failed to count videos")?;

    Ok(row.0)
}

/// Update video indexed timestamp.
pub async fn update_video_indexed(pool: &SqlitePool, id: &str) -> Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE videos SET indexed_at = ?, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to update video indexed time")?;

    Ok(result.rows_affected() > 0)
}

/// Delete a video.
pub async fn delete_video(pool: &SqlitePool, id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM videos WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete video")?;

    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Segment Queries
// ============================================================================

/// Create a new segment.
pub async fn create_segment(pool: &SqlitePool, input: NewSegment) -> Result<Segment> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query_as::<_, Segment>(
        r#"
        INSERT INTO segments (id, video_id, start_ms, end_ms, segment_type, title, description, thumbnail_path, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.video_id)
    .bind(input.start_ms)
    .bind(input.end_ms)
    .bind(input.segment_type.as_str())
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.thumbnail_path)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create segment")
}

/// Get a segment by ID.
pub async fn get_segment(pool: &SqlitePool, id: &str) -> Result<Option<Segment>> {
    sqlx::query_as::<_, Segment>("SELECT * FROM segments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get segment")
}

/// List segments for a video.
pub async fn list_segments(pool: &SqlitePool, video_id: &str) -> Result<Vec<Segment>> {
    sqlx::query_as::<_, Segment>(
        "SELECT * FROM segments WHERE video_id = ? ORDER BY start_ms ASC",
    )
    .bind(video_id)
    .fetch_all(pool)
    .await
    .context("Failed to list segments")
}

/// List segments by type for a video.
pub async fn list_segments_by_type(
    pool: &SqlitePool,
    video_id: &str,
    segment_type: SegmentType,
) -> Result<Vec<Segment>> {
    sqlx::query_as::<_, Segment>(
        "SELECT * FROM segments WHERE video_id = ? AND segment_type = ? ORDER BY start_ms ASC",
    )
    .bind(video_id)
    .bind(segment_type.as_str())
    .fetch_all(pool)
    .await
    .context("Failed to list segments by type")
}

/// Find segment at a given timestamp.
pub async fn find_segment_at_time(
    pool: &SqlitePool,
    video_id: &str,
    timestamp_ms: i64,
) -> Result<Option<Segment>> {
    sqlx::query_as::<_, Segment>(
        "SELECT * FROM segments WHERE video_id = ? AND start_ms <= ? AND end_ms > ? LIMIT 1",
    )
    .bind(video_id)
    .bind(timestamp_ms)
    .bind(timestamp_ms)
    .fetch_optional(pool)
    .await
    .context("Failed to find segment at time")
}

/// Delete segments for a video.
pub async fn delete_segments_for_video(pool: &SqlitePool, video_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM segments WHERE video_id = ?")
        .bind(video_id)
        .execute(pool)
        .await
        .context("Failed to delete segments")?;

    Ok(result.rows_affected())
}

// ============================================================================
// Keyframe Queries
// ============================================================================

/// Create a new keyframe.
pub async fn create_keyframe(pool: &SqlitePool, input: NewKeyframe) -> Result<Keyframe> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query_as::<_, Keyframe>(
        r#"
        INSERT INTO keyframes (id, video_id, timestamp_ms, frame_number, image_path, width, height, extraction_method, scene_score, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.video_id)
    .bind(input.timestamp_ms)
    .bind(input.frame_number)
    .bind(&input.image_path)
    .bind(input.width)
    .bind(input.height)
    .bind(input.extraction_method.as_str())
    .bind(input.scene_score)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create keyframe")
}

/// Create multiple keyframes in a batch.
pub async fn create_keyframes_batch(
    pool: &SqlitePool,
    inputs: &[NewKeyframe],
) -> Result<Vec<Keyframe>> {
    let mut results = Vec::with_capacity(inputs.len());
    let now = chrono::Utc::now().to_rfc3339();

    for input in inputs {
        let id = Uuid::new_v4().to_string();

        let keyframe = sqlx::query_as::<_, Keyframe>(
            r#"
            INSERT INTO keyframes (id, video_id, timestamp_ms, frame_number, image_path, width, height, extraction_method, scene_score, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&input.video_id)
        .bind(input.timestamp_ms)
        .bind(input.frame_number)
        .bind(&input.image_path)
        .bind(input.width)
        .bind(input.height)
        .bind(input.extraction_method.as_str())
        .bind(input.scene_score)
        .bind(&now)
        .fetch_one(pool)
        .await
        .context("Failed to create keyframe in batch")?;

        results.push(keyframe);
    }

    Ok(results)
}

/// Get a keyframe by ID.
pub async fn get_keyframe(pool: &SqlitePool, id: &str) -> Result<Option<Keyframe>> {
    sqlx::query_as::<_, Keyframe>("SELECT * FROM keyframes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get keyframe")
}

/// List keyframes for a video.
pub async fn list_keyframes(pool: &SqlitePool, video_id: &str) -> Result<Vec<Keyframe>> {
    sqlx::query_as::<_, Keyframe>(
        "SELECT * FROM keyframes WHERE video_id = ? ORDER BY timestamp_ms ASC",
    )
    .bind(video_id)
    .fetch_all(pool)
    .await
    .context("Failed to list keyframes")
}

/// List keyframes in a time range.
pub async fn list_keyframes_in_range(
    pool: &SqlitePool,
    video_id: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<Keyframe>> {
    sqlx::query_as::<_, Keyframe>(
        "SELECT * FROM keyframes WHERE video_id = ? AND timestamp_ms >= ? AND timestamp_ms < ? ORDER BY timestamp_ms ASC",
    )
    .bind(video_id)
    .bind(start_ms)
    .bind(end_ms)
    .fetch_all(pool)
    .await
    .context("Failed to list keyframes in range")
}

/// Count keyframes for a video.
pub async fn count_keyframes(pool: &SqlitePool, video_id: &str) -> Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM keyframes WHERE video_id = ?")
        .bind(video_id)
        .fetch_one(pool)
        .await
        .context("Failed to count keyframes")?;

    Ok(row.0)
}

/// Delete keyframes for a video.
pub async fn delete_keyframes_for_video(pool: &SqlitePool, video_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM keyframes WHERE video_id = ?")
        .bind(video_id)
        .execute(pool)
        .await
        .context("Failed to delete keyframes")?;

    Ok(result.rows_affected())
}

// ============================================================================
// Embedding Queries
// ============================================================================

/// Create a new embedding.
pub async fn create_embedding(pool: &SqlitePool, input: NewEmbedding) -> Result<Embedding> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let embedding_bytes = Embedding::from_f32_vec(&input.embedding);
    let dimensions = input.embedding.len() as i32;

    sqlx::query_as::<_, Embedding>(
        r#"
        INSERT INTO embeddings (id, source_type, source_id, model_id, embedding, dimensions, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(input.source_type.as_str())
    .bind(&input.source_id)
    .bind(&input.model_id)
    .bind(&embedding_bytes)
    .bind(dimensions)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create embedding")
}

/// Get an embedding by source.
pub async fn get_embedding_by_source(
    pool: &SqlitePool,
    source_id: &str,
    model_id: &str,
) -> Result<Option<Embedding>> {
    sqlx::query_as::<_, Embedding>(
        "SELECT * FROM embeddings WHERE source_id = ? AND model_id = ?",
    )
    .bind(source_id)
    .bind(model_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get embedding by source")
}

/// List embeddings for a model.
pub async fn list_embeddings_by_model(
    pool: &SqlitePool,
    model_id: &str,
    limit: i64,
) -> Result<Vec<Embedding>> {
    sqlx::query_as::<_, Embedding>(
        "SELECT * FROM embeddings WHERE model_id = ? ORDER BY created_at DESC LIMIT ?",
    )
    .bind(model_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to list embeddings by model")
}

/// Count embeddings by model.
pub async fn count_embeddings_by_model(pool: &SqlitePool, model_id: &str) -> Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM embeddings WHERE model_id = ?")
        .bind(model_id)
        .fetch_one(pool)
        .await
        .context("Failed to count embeddings")?;

    Ok(row.0)
}

/// Delete embeddings for a source.
pub async fn delete_embeddings_for_source(pool: &SqlitePool, source_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM embeddings WHERE source_id = ?")
        .bind(source_id)
        .execute(pool)
        .await
        .context("Failed to delete embeddings")?;

    Ok(result.rows_affected())
}

// ============================================================================
// Transcript Queries
// ============================================================================

/// Create a new transcript.
pub async fn create_transcript(pool: &SqlitePool, input: NewTranscript) -> Result<Transcript> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let word_timestamps_json = input
        .word_timestamps
        .map(|wt| serde_json::to_string(&wt).unwrap_or_default());

    sqlx::query_as::<_, Transcript>(
        r#"
        INSERT INTO transcripts (id, video_id, start_ms, end_ms, text, language, confidence, speaker_id, word_timestamps, model_id, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&id)
    .bind(&input.video_id)
    .bind(input.start_ms)
    .bind(input.end_ms)
    .bind(&input.text)
    .bind(&input.language)
    .bind(input.confidence)
    .bind(&input.speaker_id)
    .bind(&word_timestamps_json)
    .bind(&input.model_id)
    .bind(&now)
    .fetch_one(pool)
    .await
    .context("Failed to create transcript")
}

/// Get a transcript by ID.
pub async fn get_transcript(pool: &SqlitePool, id: &str) -> Result<Option<Transcript>> {
    sqlx::query_as::<_, Transcript>("SELECT * FROM transcripts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to get transcript")
}

/// List transcripts for a video.
pub async fn list_transcripts(pool: &SqlitePool, video_id: &str) -> Result<Vec<Transcript>> {
    sqlx::query_as::<_, Transcript>(
        "SELECT * FROM transcripts WHERE video_id = ? ORDER BY start_ms ASC",
    )
    .bind(video_id)
    .fetch_all(pool)
    .await
    .context("Failed to list transcripts")
}

/// List transcripts in a time range.
pub async fn list_transcripts_in_range(
    pool: &SqlitePool,
    video_id: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<Transcript>> {
    sqlx::query_as::<_, Transcript>(
        "SELECT * FROM transcripts WHERE video_id = ? AND start_ms < ? AND end_ms > ? ORDER BY start_ms ASC",
    )
    .bind(video_id)
    .bind(end_ms)
    .bind(start_ms)
    .fetch_all(pool)
    .await
    .context("Failed to list transcripts in range")
}

/// Search transcripts by text.
pub async fn search_transcripts(
    pool: &SqlitePool,
    video_id: &str,
    query: &str,
) -> Result<Vec<Transcript>> {
    let pattern = format!("%{}%", query);

    sqlx::query_as::<_, Transcript>(
        "SELECT * FROM transcripts WHERE video_id = ? AND text LIKE ? ORDER BY start_ms ASC",
    )
    .bind(video_id)
    .bind(&pattern)
    .fetch_all(pool)
    .await
    .context("Failed to search transcripts")
}

/// Delete transcripts for a video.
pub async fn delete_transcripts_for_video(pool: &SqlitePool, video_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM transcripts WHERE video_id = ?")
        .bind(video_id)
        .execute(pool)
        .await
        .context("Failed to delete transcripts")?;

    Ok(result.rows_affected())
}

// ============================================================================
// Aggregate Queries
// ============================================================================

/// Statistics for a video.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStats {
    pub keyframe_count: i64,
    pub segment_count: i64,
    pub transcript_count: i64,
    pub embedding_count: i64,
}

/// Get statistics for a video.
pub async fn get_video_stats(pool: &SqlitePool, video_id: &str) -> Result<VideoStats> {
    let keyframe_count = count_keyframes(pool, video_id).await?;

    let segment_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM segments WHERE video_id = ?")
            .bind(video_id)
            .fetch_one(pool)
            .await
            .context("Failed to count segments")?;

    let transcript_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM transcripts WHERE video_id = ?")
            .bind(video_id)
            .fetch_one(pool)
            .await
            .context("Failed to count transcripts")?;

    // Count embeddings for keyframes of this video
    let embedding_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM embeddings
        WHERE source_type = 'keyframe' AND source_id IN (
            SELECT id FROM keyframes WHERE video_id = ?
        )
        "#,
    )
    .bind(video_id)
    .fetch_one(pool)
    .await
    .context("Failed to count embeddings")?;

    Ok(VideoStats {
        keyframe_count,
        segment_count: segment_count.0,
        transcript_count: transcript_count.0,
        embedding_count: embedding_count.0,
    })
}

/// Statistics for a library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub video_count: i64,
    pub total_duration_ms: i64,
    pub total_size_bytes: i64,
    pub keyframe_count: i64,
}

/// Get statistics for a library.
pub async fn get_library_stats(pool: &SqlitePool, library_id: &str) -> Result<LibraryStats> {
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as video_count,
            COALESCE(SUM(duration_ms), 0) as total_duration_ms,
            COALESCE(SUM(file_size_bytes), 0) as total_size_bytes
        FROM videos WHERE library_id = ?
        "#,
    )
    .bind(library_id)
    .fetch_one(pool)
    .await
    .context("Failed to get library video stats")?;

    let keyframe_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM keyframes
        WHERE video_id IN (SELECT id FROM videos WHERE library_id = ?)
        "#,
    )
    .bind(library_id)
    .fetch_one(pool)
    .await
    .context("Failed to count library keyframes")?;

    Ok(LibraryStats {
        video_count: row.0,
        total_duration_ms: row.1,
        total_size_bytes: row.2,
        keyframe_count: keyframe_count.0,
    })
}
