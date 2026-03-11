//! Artifact store implementation.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Type of artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Extracted video clip.
    VideoClip,
    /// Screenshot or frame capture.
    Screenshot,
    /// Analysis report (PDF, HTML, Markdown).
    Report,
    /// Text summary.
    Summary,
    /// Frame comparison visualization.
    Comparison,
    /// Exported transcript.
    TranscriptExport,
    /// Set of keyframes.
    KeyframeSet,
    /// Other artifact type.
    Other,
}

impl ArtifactType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::VideoClip => "video_clip",
            Self::Screenshot => "screenshot",
            Self::Report => "report",
            Self::Summary => "summary",
            Self::Comparison => "comparison",
            Self::TranscriptExport => "transcript_export",
            Self::KeyframeSet => "keyframe_set",
            Self::Other => "other",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "video_clip" => Some(Self::VideoClip),
            "screenshot" => Some(Self::Screenshot),
            "report" => Some(Self::Report),
            "summary" => Some(Self::Summary),
            "comparison" => Some(Self::Comparison),
            "transcript_export" => Some(Self::TranscriptExport),
            "keyframe_set" => Some(Self::KeyframeSet),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    /// Get default file extension for this type.
    pub fn default_extension(&self) -> &'static str {
        match self {
            Self::VideoClip => "mp4",
            Self::Screenshot => "png",
            Self::Report => "md",
            Self::Summary => "txt",
            Self::Comparison => "png",
            Self::TranscriptExport => "srt",
            Self::KeyframeSet => "zip",
            Self::Other => "bin",
        }
    }

    /// Get MIME type for this artifact type.
    pub fn default_mime_type(&self) -> &'static str {
        match self {
            Self::VideoClip => "video/mp4",
            Self::Screenshot | Self::Comparison => "image/png",
            Self::Report => "text/markdown",
            Self::Summary => "text/plain",
            Self::TranscriptExport => "text/plain",
            Self::KeyframeSet => "application/zip",
            Self::Other => "application/octet-stream",
        }
    }
}

/// Artifact metadata for additional context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Source video ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_video_id: Option<String>,
    /// Time range in source video (start_ms, end_ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<(i64, i64)>,
    /// Resolution for video/image artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<(u32, u32)>,
    /// Tool that generated this artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    /// Related artifact IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_artifacts: Vec<String>,
    /// Additional custom fields.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Stored artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact ID.
    pub id: String,
    /// Associated session ID.
    pub session_id: Option<String>,
    /// Artifact type.
    pub artifact_type: ArtifactType,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// File path.
    pub path: String,
    /// File size in bytes.
    pub size_bytes: i64,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Metadata.
    pub metadata: Option<ArtifactMetadata>,
    /// Source video ID.
    pub source_video_id: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
    /// Tags.
    pub tags: Vec<String>,
}

impl Artifact {
    /// Check if the artifact has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| e < Utc::now()).unwrap_or(false)
    }

    /// Get the full file path.
    pub fn file_path(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }
}

/// Parameters for creating a new artifact.
#[derive(Debug, Clone)]
pub struct NewArtifact {
    /// Associated session ID.
    pub session_id: Option<String>,
    /// Artifact type.
    pub artifact_type: ArtifactType,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// File data.
    pub data: Vec<u8>,
    /// MIME type override.
    pub mime_type: Option<String>,
    /// Metadata.
    pub metadata: Option<ArtifactMetadata>,
    /// Source video ID.
    pub source_video_id: Option<String>,
    /// Time-to-live (artifact expires after this duration).
    pub ttl: Option<Duration>,
    /// Tags.
    pub tags: Vec<String>,
}

impl Default for NewArtifact {
    fn default() -> Self {
        Self {
            session_id: None,
            artifact_type: ArtifactType::Other,
            name: String::new(),
            description: None,
            data: Vec::new(),
            mime_type: None,
            metadata: None,
            source_video_id: None,
            ttl: None,
            tags: Vec::new(),
        }
    }
}

/// Query parameters for listing artifacts.
#[derive(Debug, Clone, Default)]
pub struct ArtifactQuery {
    /// Filter by session ID.
    pub session_id: Option<String>,
    /// Filter by artifact type.
    pub artifact_type: Option<ArtifactType>,
    /// Filter by source video ID.
    pub source_video_id: Option<String>,
    /// Filter by tag.
    pub tag: Option<String>,
    /// Include expired artifacts.
    pub include_expired: bool,
    /// Maximum results.
    pub limit: i64,
    /// Offset for pagination.
    pub offset: i64,
}

impl ArtifactQuery {
    /// Create a new query with default limits.
    pub fn new() -> Self {
        Self {
            limit: 50,
            ..Default::default()
        }
    }

    /// Filter by session.
    pub fn session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Filter by type.
    pub fn of_type(mut self, artifact_type: ArtifactType) -> Self {
        self.artifact_type = Some(artifact_type);
        self
    }

    /// Filter by tag.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    /// Set limit.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }
}

/// Configuration for artifact store.
#[derive(Debug, Clone)]
pub struct ArtifactStoreConfig {
    /// Base directory for artifact storage.
    pub storage_dir: PathBuf,
    /// Maximum artifact size in bytes.
    pub max_artifact_size: u64,
    /// Auto-cleanup expired artifacts.
    pub auto_cleanup: bool,
}

impl Default for ArtifactStoreConfig {
    fn default() -> Self {
        let storage_dir = dirs_next::home_dir()
            .map(|h| h.join(".yama").join("artifacts"))
            .unwrap_or_else(|| PathBuf::from("artifacts"));

        Self {
            storage_dir,
            max_artifact_size: 500 * 1024 * 1024, // 500 MB
            auto_cleanup: true,
        }
    }
}

/// Artifact storage service.
pub struct ArtifactStore {
    pool: SqlitePool,
    config: ArtifactStoreConfig,
}

impl ArtifactStore {
    /// Create a new artifact store.
    pub async fn new(pool: SqlitePool, config: ArtifactStoreConfig) -> Result<Self> {
        // Ensure storage directory exists
        fs::create_dir_all(&config.storage_dir)
            .await
            .with_context(|| format!("Failed to create artifact directory: {:?}", config.storage_dir))?;

        let store = Self { pool, config };

        // Run migration
        store.migrate().await?;

        Ok(store)
    }

    /// Create with default config.
    pub async fn new_default(pool: SqlitePool) -> Result<Self> {
        Self::new(pool, ArtifactStoreConfig::default()).await
    }

    /// Run the artifacts migration.
    async fn migrate(&self) -> Result<()> {
        let migration = include_str!("../db/migrations/004_artifacts.sql");
        sqlx::query(migration)
            .execute(&self.pool)
            .await
            .context("Failed to run artifacts migration")?;
        Ok(())
    }

    /// Create a new artifact.
    pub async fn create(&self, artifact: NewArtifact) -> Result<Artifact> {
        // Validate size
        if artifact.data.len() as u64 > self.config.max_artifact_size {
            return Err(anyhow::anyhow!(
                "Artifact exceeds maximum size ({} bytes > {} bytes)",
                artifact.data.len(),
                self.config.max_artifact_size
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = artifact.ttl.map(|ttl| now + ttl);

        // Determine file extension
        let extension = Path::new(&artifact.name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_else(|| artifact.artifact_type.default_extension());

        // Create file path: storage_dir/type/YYYY-MM/id.ext
        let type_dir = artifact.artifact_type.as_str();
        let date_dir = now.format("%Y-%m").to_string();
        let filename = format!("{}.{}", id, extension);
        let relative_path = PathBuf::from(type_dir).join(&date_dir).join(&filename);
        let full_path = self.config.storage_dir.join(&relative_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write file
        let mut file = fs::File::create(&full_path).await?;
        file.write_all(&artifact.data).await?;
        file.flush().await?;

        let size_bytes = artifact.data.len() as i64;
        let mime_type = artifact
            .mime_type
            .unwrap_or_else(|| artifact.artifact_type.default_mime_type().to_string());
        let metadata_json = artifact
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO artifacts (id, session_id, artifact_type, name, description, path, size_bytes, mime_type, metadata, source_video_id, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&artifact.session_id)
        .bind(artifact.artifact_type.as_str())
        .bind(&artifact.name)
        .bind(&artifact.description)
        .bind(full_path.to_string_lossy().to_string())
        .bind(size_bytes)
        .bind(&mime_type)
        .bind(&metadata_json)
        .bind(&artifact.source_video_id)
        .bind(now.to_rfc3339())
        .bind(expires_at.map(|e| e.to_rfc3339()))
        .execute(&self.pool)
        .await
        .context("Failed to insert artifact")?;

        // Insert tags
        for tag in &artifact.tags {
            sqlx::query("INSERT INTO artifact_tags (artifact_id, tag) VALUES (?, ?)")
                .bind(&id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }

        info!(id = %id, name = %artifact.name, "Created artifact");

        Ok(Artifact {
            id,
            session_id: artifact.session_id,
            artifact_type: artifact.artifact_type,
            name: artifact.name,
            description: artifact.description,
            path: full_path.to_string_lossy().to_string(),
            size_bytes,
            mime_type: Some(mime_type),
            metadata: artifact.metadata,
            source_video_id: artifact.source_video_id,
            created_at: now,
            expires_at,
            tags: artifact.tags,
        })
    }

    /// Get an artifact by ID.
    pub async fn get(&self, id: &str) -> Result<Option<Artifact>> {
        let row = sqlx::query_as::<_, (String, Option<String>, String, String, Option<String>, String, i64, Option<String>, Option<String>, Option<String>, String, Option<String>, i64)>(
            r#"
            SELECT a.id, a.session_id, a.artifact_type, a.name, a.description, a.path, a.size_bytes,
                   a.mime_type, a.metadata, a.source_video_id, a.created_at, a.expires_at, a.is_deleted
            FROM artifacts a
            WHERE a.id = ? AND a.is_deleted = 0
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            let tags = self.get_tags(id).await?;
            Ok(Some(self.row_to_artifact(r, tags)))
        } else {
            Ok(None)
        }
    }

    /// List artifacts matching a query.
    pub async fn list(&self, query: &ArtifactQuery) -> Result<Vec<Artifact>> {
        let mut sql = String::from(
            r#"
            SELECT DISTINCT a.id, a.session_id, a.artifact_type, a.name, a.description, a.path, a.size_bytes,
                   a.mime_type, a.metadata, a.source_video_id, a.created_at, a.expires_at, a.is_deleted
            FROM artifacts a
            LEFT JOIN artifact_tags t ON a.id = t.artifact_id
            WHERE a.is_deleted = 0
            "#,
        );

        let mut conditions = Vec::new();

        if query.session_id.is_some() {
            conditions.push("a.session_id = ?");
        }
        if query.artifact_type.is_some() {
            conditions.push("a.artifact_type = ?");
        }
        if query.source_video_id.is_some() {
            conditions.push("a.source_video_id = ?");
        }
        if query.tag.is_some() {
            conditions.push("t.tag = ?");
        }
        if !query.include_expired {
            conditions.push("(a.expires_at IS NULL OR a.expires_at > datetime('now'))");
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY a.created_at DESC LIMIT ? OFFSET ?");

        let mut query_builder = sqlx::query_as::<_, (String, Option<String>, String, String, Option<String>, String, i64, Option<String>, Option<String>, Option<String>, String, Option<String>, i64)>(&sql);

        if let Some(ref session_id) = query.session_id {
            query_builder = query_builder.bind(session_id);
        }
        if let Some(ref artifact_type) = query.artifact_type {
            query_builder = query_builder.bind(artifact_type.as_str());
        }
        if let Some(ref source_video_id) = query.source_video_id {
            query_builder = query_builder.bind(source_video_id);
        }
        if let Some(ref tag) = query.tag {
            query_builder = query_builder.bind(tag);
        }

        query_builder = query_builder.bind(query.limit).bind(query.offset);

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut artifacts = Vec::new();
        for row in rows {
            let id = row.0.clone();
            let tags = self.get_tags(&id).await?;
            artifacts.push(self.row_to_artifact(row, tags));
        }

        Ok(artifacts)
    }

    /// Get artifacts for a session.
    pub async fn list_by_session(&self, session_id: &str) -> Result<Vec<Artifact>> {
        self.list(&ArtifactQuery::new().session(session_id)).await
    }

    /// Delete an artifact (soft delete).
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE artifacts SET is_deleted = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        debug!(id = %id, "Deleted artifact");
        Ok(())
    }

    /// Permanently delete an artifact and its file.
    pub async fn purge(&self, id: &str) -> Result<()> {
        // Get the file path first
        let artifact = self.get(id).await?;

        if let Some(artifact) = artifact {
            // Delete file
            let path = artifact.file_path();
            if path.exists() {
                fs::remove_file(&path).await.ok();
            }

            // Delete tags
            sqlx::query("DELETE FROM artifact_tags WHERE artifact_id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;

            // Delete record
            sqlx::query("DELETE FROM artifacts WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;

            info!(id = %id, "Purged artifact");
        }

        Ok(())
    }

    /// Add a tag to an artifact.
    pub async fn add_tag(&self, id: &str, tag: &str) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO artifact_tags (artifact_id, tag) VALUES (?, ?)")
            .bind(id)
            .bind(tag)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove a tag from an artifact.
    pub async fn remove_tag(&self, id: &str, tag: &str) -> Result<()> {
        sqlx::query("DELETE FROM artifact_tags WHERE artifact_id = ? AND tag = ?")
            .bind(id)
            .bind(tag)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Cleanup expired artifacts.
    pub async fn cleanup_expired(&self) -> Result<u64> {
        // Get expired artifact IDs
        let expired: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM artifacts WHERE expires_at IS NOT NULL AND expires_at < datetime('now') AND is_deleted = 0",
        )
        .fetch_all(&self.pool)
        .await?;

        let count = expired.len() as u64;

        for (id,) in expired {
            self.purge(&id).await.ok();
        }

        if count > 0 {
            info!(count = count, "Cleaned up expired artifacts");
        }

        Ok(count)
    }

    /// Get tags for an artifact.
    async fn get_tags(&self, artifact_id: &str) -> Result<Vec<String>> {
        let tags: Vec<(String,)> = sqlx::query_as(
            "SELECT tag FROM artifact_tags WHERE artifact_id = ? ORDER BY tag",
        )
        .bind(artifact_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(|(t,)| t).collect())
    }

    /// Convert a database row to an Artifact.
    fn row_to_artifact(
        &self,
        row: (String, Option<String>, String, String, Option<String>, String, i64, Option<String>, Option<String>, Option<String>, String, Option<String>, i64),
        tags: Vec<String>,
    ) -> Artifact {
        Artifact {
            id: row.0,
            session_id: row.1,
            artifact_type: ArtifactType::from_str(&row.2).unwrap_or(ArtifactType::Other),
            name: row.3,
            description: row.4,
            path: row.5,
            size_bytes: row.6,
            mime_type: row.7,
            metadata: row.8.and_then(|s| serde_json::from_str(&s).ok()),
            source_video_id: row.9,
            created_at: DateTime::parse_from_rfc3339(&row.10)
                .unwrap_or_default()
                .with_timezone(&Utc),
            expires_at: row.11.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            tags,
        }
    }

    /// Get total storage used by artifacts.
    pub async fn storage_used(&self) -> Result<i64> {
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM artifacts WHERE is_deleted = 0",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(total)
    }

    /// Count artifacts by type.
    pub async fn count_by_type(&self) -> Result<std::collections::HashMap<String, i64>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT artifact_type, COUNT(*) FROM artifacts WHERE is_deleted = 0 GROUP BY artifact_type",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_store() -> (ArtifactStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage_dir = temp_dir.path().join("artifacts");

        let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();

        // Run base migrations
        sqlx::query(include_str!("../db/migrations/001_initial.sql"))
            .execute(&pool)
            .await
            .ok();
        sqlx::query(include_str!("../db/migrations/003_chat_history.sql"))
            .execute(&pool)
            .await
            .ok();

        let config = ArtifactStoreConfig {
            storage_dir,
            max_artifact_size: 1024 * 1024,
            auto_cleanup: false,
        };

        let store = ArtifactStore::new(pool, config).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_create_artifact() {
        let (store, _temp) = setup_test_store().await;

        let artifact = store
            .create(NewArtifact {
                artifact_type: ArtifactType::Screenshot,
                name: "test_screenshot.png".to_string(),
                data: vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!artifact.id.is_empty());
        assert_eq!(artifact.name, "test_screenshot.png");
        assert_eq!(artifact.artifact_type, ArtifactType::Screenshot);
        assert_eq!(artifact.size_bytes, 4);

        // File should exist
        assert!(PathBuf::from(&artifact.path).exists());
    }

    #[tokio::test]
    async fn test_get_artifact() {
        let (store, _temp) = setup_test_store().await;

        let created = store
            .create(NewArtifact {
                artifact_type: ArtifactType::VideoClip,
                name: "clip.mp4".to_string(),
                data: vec![0; 100],
                description: Some("Test clip".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        let retrieved = store.get(&created.id).await.unwrap();
        assert!(retrieved.is_some());

        let artifact = retrieved.unwrap();
        assert_eq!(artifact.id, created.id);
        assert_eq!(artifact.description, Some("Test clip".to_string()));
    }

    #[tokio::test]
    async fn test_list_artifacts() {
        let (store, _temp) = setup_test_store().await;

        // Create multiple artifacts
        for i in 0..5 {
            store
                .create(NewArtifact {
                    artifact_type: if i % 2 == 0 {
                        ArtifactType::Screenshot
                    } else {
                        ArtifactType::VideoClip
                    },
                    name: format!("artifact_{}.bin", i),
                    data: vec![i as u8; 10],
                    ..Default::default()
                })
                .await
                .unwrap();
        }

        // List all
        let all = store.list(&ArtifactQuery::new()).await.unwrap();
        assert_eq!(all.len(), 5);

        // List by type
        let screenshots = store
            .list(&ArtifactQuery::new().of_type(ArtifactType::Screenshot))
            .await
            .unwrap();
        assert_eq!(screenshots.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_artifact() {
        let (store, _temp) = setup_test_store().await;

        let artifact = store
            .create(NewArtifact {
                artifact_type: ArtifactType::Summary,
                name: "summary.txt".to_string(),
                data: b"Test summary content".to_vec(),
                ..Default::default()
            })
            .await
            .unwrap();

        // Soft delete
        store.delete(&artifact.id).await.unwrap();

        // Should not be found
        let retrieved = store.get(&artifact.id).await.unwrap();
        assert!(retrieved.is_none());

        // File should still exist (soft delete)
        assert!(PathBuf::from(&artifact.path).exists());
    }

    #[tokio::test]
    async fn test_purge_artifact() {
        let (store, _temp) = setup_test_store().await;

        let artifact = store
            .create(NewArtifact {
                artifact_type: ArtifactType::Report,
                name: "report.md".to_string(),
                data: b"# Report".to_vec(),
                ..Default::default()
            })
            .await
            .unwrap();

        let path = artifact.file_path();

        // Purge
        store.purge(&artifact.id).await.unwrap();

        // File should be gone
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_artifact_tags() {
        let (store, _temp) = setup_test_store().await;

        let artifact = store
            .create(NewArtifact {
                artifact_type: ArtifactType::Screenshot,
                name: "tagged.png".to_string(),
                data: vec![0; 10],
                tags: vec!["important".to_string(), "review".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(artifact.tags.len(), 2);

        // Add another tag
        store.add_tag(&artifact.id, "final").await.unwrap();

        let retrieved = store.get(&artifact.id).await.unwrap().unwrap();
        assert_eq!(retrieved.tags.len(), 3);

        // List by tag
        let tagged = store
            .list(&ArtifactQuery::new().with_tag("important"))
            .await
            .unwrap();
        assert_eq!(tagged.len(), 1);
    }

    #[tokio::test]
    async fn test_artifact_with_metadata() {
        let (store, _temp) = setup_test_store().await;

        let metadata = ArtifactMetadata {
            source_video_id: Some("video-123".to_string()),
            time_range: Some((1000, 5000)),
            resolution: Some((1920, 1080)),
            generated_by: Some("extract_clip".to_string()),
            ..Default::default()
        };

        let artifact = store
            .create(NewArtifact {
                artifact_type: ArtifactType::VideoClip,
                name: "clip_with_metadata.mp4".to_string(),
                data: vec![0; 50],
                metadata: Some(metadata),
                ..Default::default()
            })
            .await
            .unwrap();

        let retrieved = store.get(&artifact.id).await.unwrap().unwrap();
        let meta = retrieved.metadata.unwrap();

        assert_eq!(meta.source_video_id, Some("video-123".to_string()));
        assert_eq!(meta.time_range, Some((1000, 5000)));
        assert_eq!(meta.resolution, Some((1920, 1080)));
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let (store, _temp) = setup_test_store().await;

        // Create artifacts of different types
        store
            .create(NewArtifact {
                artifact_type: ArtifactType::Screenshot,
                name: "s1.png".to_string(),
                data: vec![0; 100],
                ..Default::default()
            })
            .await
            .unwrap();

        store
            .create(NewArtifact {
                artifact_type: ArtifactType::Screenshot,
                name: "s2.png".to_string(),
                data: vec![0; 200],
                ..Default::default()
            })
            .await
            .unwrap();

        store
            .create(NewArtifact {
                artifact_type: ArtifactType::VideoClip,
                name: "c1.mp4".to_string(),
                data: vec![0; 300],
                ..Default::default()
            })
            .await
            .unwrap();

        // Check total storage
        let total = store.storage_used().await.unwrap();
        assert_eq!(total, 600);

        // Check counts by type
        let counts = store.count_by_type().await.unwrap();
        assert_eq!(counts.get("screenshot"), Some(&2));
        assert_eq!(counts.get("video_clip"), Some(&1));
    }

    #[tokio::test]
    async fn test_max_size_enforcement() {
        let (store, _temp) = setup_test_store().await;

        // Try to create artifact larger than max
        let result = store
            .create(NewArtifact {
                artifact_type: ArtifactType::VideoClip,
                name: "huge.mp4".to_string(),
                data: vec![0; 2 * 1024 * 1024], // 2MB > 1MB limit
                ..Default::default()
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum size"));
    }
}
