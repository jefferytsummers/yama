//! Vector similarity search for embeddings.
//!
//! Provides similarity search over embeddings stored in SQLite.
//! Uses brute-force cosine similarity with optional sqlite-vss acceleration.

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::{FromRow, SqlitePool};
use tracing::{debug, info};

use crate::embedding::preprocessor::{cosine_similarity, l2_normalize};

/// Source type for embeddings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddingSource {
    /// Keyframe visual embedding.
    Keyframe,
    /// Transcript text embedding.
    Transcript,
    /// Document text embedding.
    Document,
}

impl EmbeddingSource {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keyframe => "keyframe",
            Self::Transcript => "transcript",
            Self::Document => "document",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "keyframe" => Some(Self::Keyframe),
            "transcript" => Some(Self::Transcript),
            "document" => Some(Self::Document),
            _ => None,
        }
    }
}

/// Stored embedding record.
#[derive(Debug, Clone)]
pub struct StoredEmbedding {
    /// Unique embedding ID.
    pub id: String,
    /// Source type.
    pub source_type: EmbeddingSource,
    /// Source record ID (keyframe_id, transcript_id, etc.).
    pub source_id: String,
    /// Model used to generate the embedding.
    pub model_id: String,
    /// The embedding vector.
    pub embedding: Vec<f32>,
}

/// Search result with similarity score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Embedding ID.
    pub id: String,
    /// Source type.
    pub source_type: EmbeddingSource,
    /// Source record ID.
    pub source_id: String,
    /// Cosine similarity score (0.0 - 1.0).
    pub similarity: f32,
}

/// Vector index configuration.
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    /// Embedding dimension.
    pub dimension: usize,
    /// Whether to use sqlite-vss if available.
    pub use_vss: bool,
    /// Default search limit.
    pub default_limit: usize,
    /// Minimum similarity threshold.
    pub min_similarity: f32,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            dimension: 512, // CLIP ViT-B/32
            use_vss: false, // Disabled by default, requires extension
            default_limit: 10,
            min_similarity: 0.0,
        }
    }
}

/// Vector index for similarity search.
pub struct VectorIndex {
    pool: SqlitePool,
    config: VectorIndexConfig,
    /// In-memory cache for fast search (optional).
    cache: Option<EmbeddingCache>,
}

/// In-memory embedding cache for fast brute-force search.
struct EmbeddingCache {
    embeddings: HashMap<String, CachedEmbedding>,
}

struct CachedEmbedding {
    source_type: EmbeddingSource,
    source_id: String,
    vector: Vec<f32>,
}

impl VectorIndex {
    /// Create a new vector index.
    pub fn new(pool: SqlitePool, config: VectorIndexConfig) -> Self {
        Self {
            pool,
            config,
            cache: None,
        }
    }

    /// Run migrations for vector search.
    pub async fn migrate(&self) -> Result<()> {
        let migration = include_str!("migrations/002_vector_search.sql");
        sqlx::raw_sql(migration)
            .execute(&self.pool)
            .await
            .context("Failed to run vector search migration")?;
        info!("Vector search migration complete");
        Ok(())
    }

    /// Insert an embedding.
    pub async fn insert(
        &mut self,
        id: &str,
        source_type: EmbeddingSource,
        source_id: &str,
        model_id: &str,
        embedding: &[f32],
    ) -> Result<()> {
        // Normalize the embedding
        let mut normalized = embedding.to_vec();
        l2_normalize(&mut normalized);

        // Convert to bytes
        let embedding_bytes = bytemuck::cast_slice::<f32, u8>(&normalized).to_vec();

        sqlx::query(
            r#"
            INSERT INTO embeddings_store (id, source_type, source_id, model_id, embedding)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                embedding = excluded.embedding,
                created_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(id)
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(model_id)
        .bind(&embedding_bytes)
        .execute(&self.pool)
        .await
        .context("Failed to insert embedding")?;

        // Update cache if enabled
        if let Some(ref mut cache) = self.cache {
            cache.embeddings.insert(
                id.to_string(),
                CachedEmbedding {
                    source_type,
                    source_id: source_id.to_string(),
                    vector: normalized,
                },
            );
        }

        Ok(())
    }

    /// Insert multiple embeddings in a batch.
    pub async fn insert_batch(&mut self, embeddings: &[StoredEmbedding]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for emb in embeddings {
            let mut normalized = emb.embedding.clone();
            l2_normalize(&mut normalized);
            let embedding_bytes = bytemuck::cast_slice::<f32, u8>(&normalized).to_vec();

            sqlx::query(
                r#"
                INSERT INTO embeddings_store (id, source_type, source_id, model_id, embedding)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    embedding = excluded.embedding,
                    created_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(&emb.id)
            .bind(emb.source_type.as_str())
            .bind(&emb.source_id)
            .bind(&emb.model_id)
            .bind(&embedding_bytes)
            .execute(&mut *tx)
            .await
            .context("Failed to insert embedding")?;
        }

        tx.commit().await?;

        // Invalidate cache
        self.cache = None;

        Ok(())
    }

    /// Delete an embedding.
    pub async fn delete(&mut self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM embeddings_store WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete embedding")?;

        if let Some(ref mut cache) = self.cache {
            cache.embeddings.remove(id);
        }

        Ok(())
    }

    /// Search for similar embeddings.
    pub async fn search(
        &mut self,
        query: &[f32],
        source_type: Option<EmbeddingSource>,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        let limit = limit.unwrap_or(self.config.default_limit);

        // Normalize query
        let mut query_normalized = query.to_vec();
        l2_normalize(&mut query_normalized);

        // Use cache if available
        if let Some(ref cache) = self.cache {
            return Ok(self.search_cached(&query_normalized, source_type, limit, cache));
        }

        // Load embeddings from database
        let embeddings = self.load_embeddings(source_type).await?;

        // Compute similarities
        let mut results: Vec<SearchResult> = embeddings
            .into_iter()
            .filter_map(|(id, emb)| {
                let similarity = cosine_similarity(&query_normalized, &emb.vector);
                if similarity >= self.config.min_similarity {
                    Some(SearchResult {
                        id,
                        source_type: emb.source_type,
                        source_id: emb.source_id,
                        similarity,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N
        results.truncate(limit);

        Ok(results)
    }

    /// Search using the in-memory cache.
    fn search_cached(
        &self,
        query: &[f32],
        source_type: Option<EmbeddingSource>,
        limit: usize,
        cache: &EmbeddingCache,
    ) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = cache
            .embeddings
            .iter()
            .filter(|(_, emb)| {
                source_type.map_or(true, |st| emb.source_type == st)
            })
            .filter_map(|(id, emb)| {
                let similarity = cosine_similarity(query, &emb.vector);
                if similarity >= self.config.min_similarity {
                    Some(SearchResult {
                        id: id.clone(),
                        source_type: emb.source_type,
                        source_id: emb.source_id.clone(),
                        similarity,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Load all embeddings from the database.
    async fn load_embeddings(
        &self,
        source_type: Option<EmbeddingSource>,
    ) -> Result<HashMap<String, CachedEmbedding>> {
        #[derive(FromRow)]
        struct EmbeddingRow {
            id: String,
            source_type: String,
            source_id: String,
            embedding: Vec<u8>,
        }

        let rows: Vec<EmbeddingRow> = if let Some(st) = source_type {
            sqlx::query_as(
                "SELECT id, source_type, source_id, embedding FROM embeddings_store WHERE source_type = ?"
            )
            .bind(st.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT id, source_type, source_id, embedding FROM embeddings_store"
            )
            .fetch_all(&self.pool)
            .await?
        };

        let mut result = HashMap::new();
        for row in rows {
            let vector: Vec<f32> = bytemuck::cast_slice(&row.embedding).to_vec();
            let source_type = EmbeddingSource::from_str(&row.source_type)
                .unwrap_or(EmbeddingSource::Document);

            result.insert(
                row.id,
                CachedEmbedding {
                    source_type,
                    source_id: row.source_id,
                    vector,
                },
            );
        }

        debug!("Loaded {} embeddings from database", result.len());
        Ok(result)
    }

    /// Build the in-memory cache for fast searching.
    pub async fn build_cache(&mut self) -> Result<()> {
        info!("Building embedding cache");
        let embeddings = self.load_embeddings(None).await?;
        info!("Cached {} embeddings", embeddings.len());
        self.cache = Some(EmbeddingCache { embeddings });
        Ok(())
    }

    /// Clear the in-memory cache.
    pub fn clear_cache(&mut self) {
        self.cache = None;
    }

    /// Get the number of stored embeddings.
    pub async fn count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM embeddings_store")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Get the number of embeddings by source type.
    pub async fn count_by_type(&self, source_type: EmbeddingSource) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM embeddings_store WHERE source_type = ?"
        )
        .bind(source_type.as_str())
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_source() {
        assert_eq!(EmbeddingSource::Keyframe.as_str(), "keyframe");
        assert_eq!(EmbeddingSource::Transcript.as_str(), "transcript");
        assert_eq!(EmbeddingSource::Document.as_str(), "document");

        assert_eq!(EmbeddingSource::from_str("keyframe"), Some(EmbeddingSource::Keyframe));
        assert_eq!(EmbeddingSource::from_str("invalid"), None);
    }

    #[test]
    fn test_config_defaults() {
        let config = VectorIndexConfig::default();
        assert_eq!(config.dimension, 512);
        assert!(!config.use_vss);
        assert_eq!(config.default_limit, 10);
    }

    #[test]
    fn test_similarity_ranking() {
        // Create a mock cache for testing
        let query = vec![1.0, 0.0, 0.0];
        let mut query_normalized = query.clone();
        l2_normalize(&mut query_normalized);

        let emb1 = vec![1.0, 0.0, 0.0]; // Identical
        let emb2 = vec![0.0, 1.0, 0.0]; // Orthogonal
        let emb3 = vec![0.7, 0.7, 0.0]; // Similar

        let sim1 = cosine_similarity(&query_normalized, &emb1);
        let sim2 = cosine_similarity(&query_normalized, &emb2);
        let sim3 = cosine_similarity(&query_normalized, &emb3);

        assert!(sim1 > sim3);
        assert!(sim3 > sim2);
        assert!((sim1 - 1.0).abs() < 0.001);
        assert!(sim2.abs() < 0.001);
    }
}
