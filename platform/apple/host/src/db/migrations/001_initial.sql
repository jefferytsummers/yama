-- Yama Database Schema v1.0
-- Initial migration for video library indexing

-- Projects organize video libraries
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Libraries are directories containing videos
CREATE TABLE IF NOT EXISTS libraries (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_indexed_at TEXT,
    UNIQUE(project_id, root_path)
);

-- Videos are individual media files
CREATE TABLE IF NOT EXISTS videos (
    id TEXT PRIMARY KEY NOT NULL,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    filename TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    fps REAL NOT NULL,
    codec TEXT NOT NULL,
    container TEXT,
    file_size_bytes INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    indexed_at TEXT,
    UNIQUE(library_id, path)
);

-- Segments are contiguous portions of video (scenes, chapters)
CREATE TABLE IF NOT EXISTS segments (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    segment_type TEXT NOT NULL CHECK(segment_type IN ('scene', 'chapter', 'manual', 'silence', 'speech')),
    title TEXT,
    description TEXT,
    thumbnail_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(end_ms > start_ms)
);

-- Keyframes are individual frames extracted for analysis
CREATE TABLE IF NOT EXISTS keyframes (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    timestamp_ms INTEGER NOT NULL,
    frame_number INTEGER NOT NULL,
    image_path TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    extraction_method TEXT NOT NULL CHECK(extraction_method IN ('interval', 'scene_change', 'iframe', 'manual')),
    scene_score REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(video_id, frame_number)
);

-- Embeddings store vector representations for similarity search
CREATE TABLE IF NOT EXISTS embeddings (
    id TEXT PRIMARY KEY NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('keyframe', 'segment', 'transcript')),
    source_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    embedding BLOB NOT NULL,
    dimensions INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_id, model_id)
);

-- Transcripts store audio transcriptions
CREATE TABLE IF NOT EXISTS transcripts (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    language TEXT,
    confidence REAL,
    speaker_id TEXT,
    word_timestamps TEXT, -- JSON array of word-level timestamps
    model_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK(end_ms >= start_ms)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_libraries_project ON libraries(project_id);
CREATE INDEX IF NOT EXISTS idx_videos_library ON videos(library_id);
CREATE INDEX IF NOT EXISTS idx_videos_hash ON videos(file_hash);
CREATE INDEX IF NOT EXISTS idx_segments_video ON segments(video_id);
CREATE INDEX IF NOT EXISTS idx_segments_time ON segments(video_id, start_ms, end_ms);
CREATE INDEX IF NOT EXISTS idx_keyframes_video ON keyframes(video_id);
CREATE INDEX IF NOT EXISTS idx_keyframes_time ON keyframes(video_id, timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model_id);
CREATE INDEX IF NOT EXISTS idx_transcripts_video ON transcripts(video_id);
CREATE INDEX IF NOT EXISTS idx_transcripts_time ON transcripts(video_id, start_ms, end_ms);
