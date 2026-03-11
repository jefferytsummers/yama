-- Vector search support
-- Note: sqlite-vss extension must be loaded separately

-- Create embeddings table (used without vss extension as fallback)
-- The embeddings are stored as JSON arrays or raw blobs
CREATE TABLE IF NOT EXISTS embeddings_store (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,  -- 'keyframe', 'transcript', 'document'
    source_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- Raw f32 bytes
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_embeddings_store_source
ON embeddings_store(source_type, source_id);

CREATE INDEX IF NOT EXISTS idx_embeddings_store_model
ON embeddings_store(model_id);

-- When sqlite-vss is available, create virtual tables:
-- CREATE VIRTUAL TABLE vss_embeddings USING vss0(embedding(512));
-- CREATE TABLE vss_embeddings_mapping (rowid INTEGER PRIMARY KEY, embedding_id TEXT);
