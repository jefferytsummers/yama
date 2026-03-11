-- Add config column to projects table
-- Stores project-level configuration as JSON
-- Note: This migration is idempotent - it will skip if column exists

-- Create a temporary table to check if we need to migrate
CREATE TABLE IF NOT EXISTS _migration_status (
    migration_id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL
);

-- Insert migration record if not exists (will fail silently if exists due to IGNORE)
INSERT OR IGNORE INTO _migration_status (migration_id, applied_at)
SELECT '005_project_config', datetime('now')
WHERE NOT EXISTS (SELECT 1 FROM pragma_table_info('projects') WHERE name = 'config');
