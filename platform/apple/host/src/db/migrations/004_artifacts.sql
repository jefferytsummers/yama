-- Artifact storage tables for agent-generated content
-- Migration: 004_artifacts

-- Artifacts table
CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN (
        'video_clip',
        'screenshot',
        'report',
        'summary',
        'comparison',
        'transcript_export',
        'keyframe_set',
        'other'
    )),
    name TEXT NOT NULL,
    description TEXT,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,
    metadata TEXT,  -- JSON for flexible extension
    source_video_id TEXT,  -- Optional reference to source video
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,  -- Optional expiration for temp artifacts
    is_deleted INTEGER NOT NULL DEFAULT 0,

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE SET NULL,
    FOREIGN KEY (source_video_id) REFERENCES videos(id) ON DELETE SET NULL
);

-- Indexes for artifacts
CREATE INDEX IF NOT EXISTS idx_artifacts_session ON artifacts(session_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_type ON artifacts(artifact_type);
CREATE INDEX IF NOT EXISTS idx_artifacts_source_video ON artifacts(source_video_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_created ON artifacts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_artifacts_expires ON artifacts(expires_at) WHERE expires_at IS NOT NULL;

-- Artifact tags for categorization
CREATE TABLE IF NOT EXISTS artifact_tags (
    artifact_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (artifact_id, tag),
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

-- Index for tag lookups
CREATE INDEX IF NOT EXISTS idx_artifact_tags_tag ON artifact_tags(tag);
