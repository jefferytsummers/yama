-- Chat session history tables for agent conversations
-- Migration: 003_chat_history

-- Chat sessions table
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT,
    preset_id TEXT NOT NULL,
    title TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    is_archived INTEGER NOT NULL DEFAULT 0,
    metadata TEXT,  -- JSON for flexible extension

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

-- Indexes for chat_sessions
CREATE INDEX IF NOT EXISTS idx_chat_sessions_project ON chat_sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_preset ON chat_sessions(preset_id);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_created ON chat_sessions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated ON chat_sessions(updated_at DESC);

-- Chat messages table
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    tool_calls TEXT,      -- JSON array of tool calls
    tool_call_id TEXT,    -- For tool response messages
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    token_count INTEGER,  -- Optional token usage tracking
    model_id TEXT,        -- Which model generated this (for assistant messages)

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Indexes for chat_messages
CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session_created ON chat_messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_chat_messages_role ON chat_messages(role);

-- Trigger to update session updated_at when messages are added
CREATE TRIGGER IF NOT EXISTS update_session_timestamp
AFTER INSERT ON chat_messages
BEGIN
    UPDATE chat_sessions
    SET updated_at = datetime('now')
    WHERE id = NEW.session_id;
END;
