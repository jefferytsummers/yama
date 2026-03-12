//! Database module for Yama.
//!
//! SQLite-based persistence for video libraries, keyframes, embeddings, and transcripts.
//!
//! # Usage
//!
//! ```rust,ignore
//! use yama_host_apple::db::{Database, queries};
//!
//! let db = Database::open("~/.yama/yama.db").await?;
//! let project = queries::create_project(&db.pool, NewProject {
//!     name: "My Project".to_string(),
//!     description: None,
//! }).await?;
//! ```

pub mod chat_history;
pub mod models;
pub mod queries;
// TODO: Re-enable when embedding pipeline is implemented
// pub mod vector_search;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use tracing::{debug, info};

// Re-exports for convenience
pub use chat_history::{ChatHistory, ChatMessageRow, ChatSessionRow, MessageRole, StoredToolCall};
pub use models::*;
pub use queries::*;
// TODO: Re-enable when embedding pipeline is implemented
// pub use vector_search::{
//     EmbeddingSource, SearchResult, StoredEmbedding, VectorIndex, VectorIndexConfig,
// };

/// Default database path under user's home directory.
pub const DEFAULT_DB_PATH: &str = ".yama/yama.db";

/// Get the default database path.
pub fn default_db_path() -> PathBuf {
    dirs_next::home_dir()
        .map(|h| h.join(DEFAULT_DB_PATH))
        .unwrap_or_else(|| PathBuf::from("yama.db"))
}

/// Database connection wrapper.
#[derive(Debug, Clone)]
pub struct Database {
    /// SQLite connection pool.
    pub pool: SqlitePool,
    /// Path to the database file.
    pub path: PathBuf,
}

impl Database {
    /// Open or create a database at the given path.
    ///
    /// Creates the parent directory if it doesn't exist.
    /// Runs migrations automatically.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
        }

        info!("Opening database at {:?}", path);

        // Configure SQLite connection options
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open database: {:?}", path))?;

        let db = Self { pool, path };

        // Run migrations
        db.migrate().await?;

        info!("Database ready");
        Ok(db)
    }

    /// Open the database at the default location.
    pub async fn open_default() -> Result<Self> {
        Self::open(default_db_path()).await
    }

    /// Run database migrations.
    async fn migrate(&self) -> Result<()> {
        debug!("Running database migrations");

        // Initial schema migration
        let initial_schema = include_str!("migrations/001_initial.sql");
        sqlx::raw_sql(initial_schema)
            .execute(&self.pool)
            .await
            .context("Failed to run initial migration")?;

        // Vector search migration
        let vector_schema = include_str!("migrations/002_vector_search.sql");
        sqlx::raw_sql(vector_schema)
            .execute(&self.pool)
            .await
            .context("Failed to run vector search migration")?;

        // Chat history migration
        let chat_schema = include_str!("migrations/003_chat_history.sql");
        sqlx::raw_sql(chat_schema)
            .execute(&self.pool)
            .await
            .context("Failed to run chat history migration")?;

        // Artifacts migration
        let artifacts_schema = include_str!("migrations/004_artifacts.sql");
        sqlx::raw_sql(artifacts_schema)
            .execute(&self.pool)
            .await
            .context("Failed to run artifacts migration")?;

        // Project config migration - add config column if it doesn't exist
        // Check if column exists first
        let has_config: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('projects') WHERE name = 'config'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if !has_config {
            sqlx::query("ALTER TABLE projects ADD COLUMN config TEXT")
                .execute(&self.pool)
                .await
                .context("Failed to add config column to projects")?;
            debug!("Added config column to projects table");
        }

        debug!("Migrations complete");
        Ok(())
    }

    /// Check if the database is accessible.
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Get database statistics.
    pub async fn stats(&self) -> Result<DatabaseStats> {
        let project_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await?;

        let library_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM libraries")
            .fetch_one(&self.pool)
            .await?;

        let video_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM videos")
            .fetch_one(&self.pool)
            .await?;

        let keyframe_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM keyframes")
            .fetch_one(&self.pool)
            .await?;

        let embedding_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM embeddings")
            .fetch_one(&self.pool)
            .await?;

        let transcript_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM transcripts")
            .fetch_one(&self.pool)
            .await?;

        // Get database file size
        let file_size = std::fs::metadata(&self.path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        Ok(DatabaseStats {
            project_count: project_count.0,
            library_count: library_count.0,
            video_count: video_count.0,
            keyframe_count: keyframe_count.0,
            embedding_count: embedding_count.0,
            transcript_count: transcript_count.0,
            file_size_bytes: file_size,
        })
    }

    /// Close the database connection pool.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Database statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    pub project_count: i64,
    pub library_count: i64,
    pub video_count: i64,
    pub keyframe_count: i64,
    pub embedding_count: i64,
    pub transcript_count: i64,
    pub file_size_bytes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_database_open() {
        let (db, _temp) = setup_test_db().await;
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_database_stats() {
        let (db, _temp) = setup_test_db().await;
        let stats = db.stats().await.unwrap();
        assert_eq!(stats.project_count, 0);
        assert_eq!(stats.video_count, 0);
    }

    #[tokio::test]
    async fn test_create_project() {
        let (db, _temp) = setup_test_db().await;

        let project = create_project(
            &db.pool,
            NewProject {
                name: "Test Project".to_string(),
                description: Some("A test project".to_string()),
                config: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));

        // Verify it's in the database
        let fetched = get_project(&db.pool, &project.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Test Project");
    }

    #[tokio::test]
    async fn test_create_library() {
        let (db, _temp) = setup_test_db().await;

        let project = create_project(
            &db.pool,
            NewProject {
                name: "Test Project".to_string(),
                description: None,
                config: None,
            },
        )
        .await
        .unwrap();

        let library = create_library(
            &db.pool,
            NewLibrary {
                project_id: project.id.clone(),
                name: "Test Library".to_string(),
                root_path: "/videos".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(library.name, "Test Library");
        assert_eq!(library.root_path, "/videos");
        assert_eq!(library.project_id, project.id);

        // List libraries
        let libraries = list_libraries(&db.pool, &project.id).await.unwrap();
        assert_eq!(libraries.len(), 1);
    }

    #[tokio::test]
    async fn test_create_video() {
        let (db, _temp) = setup_test_db().await;

        let project = create_project(
            &db.pool,
            NewProject {
                name: "Test Project".to_string(),
                description: None,
                config: None,
            },
        )
        .await
        .unwrap();

        let library = create_library(
            &db.pool,
            NewLibrary {
                project_id: project.id,
                name: "Test Library".to_string(),
                root_path: "/videos".to_string(),
            },
        )
        .await
        .unwrap();

        let video = create_video(
            &db.pool,
            NewVideo {
                library_id: library.id.clone(),
                path: "/videos/test.mp4".to_string(),
                filename: "test.mp4".to_string(),
                duration_ms: 60000,
                width: 1920,
                height: 1080,
                fps: 30.0,
                codec: "h264".to_string(),
                container: Some("mp4".to_string()),
                file_size_bytes: 10_000_000,
                file_hash: "abc123".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(video.filename, "test.mp4");
        assert_eq!(video.duration_ms, 60000);
        assert_eq!(video.width, 1920);
        assert_eq!(video.fps, 30.0);

        // Count videos
        let count = count_videos(&db.pool, &library.id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_create_keyframe() {
        let (db, _temp) = setup_test_db().await;

        let project = create_project(
            &db.pool,
            NewProject {
                name: "Test".to_string(),
                description: None,
                config: None,
            },
        )
        .await
        .unwrap();

        let library = create_library(
            &db.pool,
            NewLibrary {
                project_id: project.id,
                name: "Lib".to_string(),
                root_path: "/".to_string(),
            },
        )
        .await
        .unwrap();

        let video = create_video(
            &db.pool,
            NewVideo {
                library_id: library.id,
                path: "/test.mp4".to_string(),
                filename: "test.mp4".to_string(),
                duration_ms: 60000,
                width: 1920,
                height: 1080,
                fps: 30.0,
                codec: "h264".to_string(),
                container: None,
                file_size_bytes: 1000,
                file_hash: "hash".to_string(),
            },
        )
        .await
        .unwrap();

        let keyframe = create_keyframe(
            &db.pool,
            NewKeyframe {
                video_id: video.id.clone(),
                timestamp_ms: 5000,
                frame_number: 150,
                image_path: "/frames/0150.jpg".to_string(),
                width: 1920,
                height: 1080,
                extraction_method: ExtractionMethod::Interval,
                scene_score: Some(0.85),
            },
        )
        .await
        .unwrap();

        assert_eq!(keyframe.timestamp_ms, 5000);
        assert_eq!(keyframe.frame_number, 150);
        assert_eq!(keyframe.scene_score, Some(0.85));

        // List keyframes
        let keyframes = list_keyframes(&db.pool, &video.id).await.unwrap();
        assert_eq!(keyframes.len(), 1);
    }

    #[tokio::test]
    async fn test_embedding_encoding() {
        let original = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
        let bytes = Embedding::from_f32_vec(&original);
        assert_eq!(bytes.len(), 20); // 5 floats * 4 bytes

        let embedding = Embedding {
            id: "test".to_string(),
            source_type: "keyframe".to_string(),
            source_id: "kf1".to_string(),
            model_id: "clip".to_string(),
            embedding: bytes,
            dimensions: 5,
            created_at: "".to_string(),
        };

        let decoded = embedding.to_f32_vec();
        assert_eq!(decoded.len(), 5);

        for (a, b) in original.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let (db, _temp) = setup_test_db().await;

        let project = create_project(
            &db.pool,
            NewProject {
                name: "Test".to_string(),
                description: None,
                config: None,
            },
        )
        .await
        .unwrap();

        let library = create_library(
            &db.pool,
            NewLibrary {
                project_id: project.id.clone(),
                name: "Lib".to_string(),
                root_path: "/".to_string(),
            },
        )
        .await
        .unwrap();

        let video = create_video(
            &db.pool,
            NewVideo {
                library_id: library.id,
                path: "/test.mp4".to_string(),
                filename: "test.mp4".to_string(),
                duration_ms: 60000,
                width: 1920,
                height: 1080,
                fps: 30.0,
                codec: "h264".to_string(),
                container: None,
                file_size_bytes: 1000,
                file_hash: "hash".to_string(),
            },
        )
        .await
        .unwrap();

        // Delete project should cascade
        delete_project(&db.pool, &project.id).await.unwrap();

        // Video should be gone
        let v = get_video(&db.pool, &video.id).await.unwrap();
        assert!(v.is_none());
    }
}
