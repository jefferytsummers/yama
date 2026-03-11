//! Chat history persistence for agent sessions.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Chat message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message.
    User,
    /// Assistant response.
    Assistant,
    /// System prompt.
    System,
    /// Tool result.
    Tool,
}

impl MessageRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "system" => Some(Self::System),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

/// Tool call information stored in messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToolCall {
    /// Tool call ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Tool arguments as JSON.
    pub arguments: Value,
}

/// A chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionRow {
    /// Session ID.
    pub id: String,
    /// Associated project ID.
    pub project_id: Option<String>,
    /// Agent preset ID.
    pub preset_id: String,
    /// Session title.
    pub title: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Whether the session is archived.
    pub is_archived: bool,
    /// Additional metadata.
    pub metadata: Option<Value>,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRow {
    /// Message ID.
    pub id: String,
    /// Session ID.
    pub session_id: String,
    /// Message role.
    pub role: MessageRole,
    /// Message content.
    pub content: String,
    /// Tool calls (for assistant messages).
    pub tool_calls: Option<Vec<StoredToolCall>>,
    /// Tool call ID (for tool response messages).
    pub tool_call_id: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Token count (optional).
    pub token_count: Option<i64>,
    /// Model ID (for assistant messages).
    pub model_id: Option<String>,
}

/// Chat history database operations.
pub struct ChatHistory {
    pool: SqlitePool,
}

impl ChatHistory {
    /// Create a new chat history handler.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new chat session.
    pub async fn create_session(
        &self,
        preset_id: &str,
        project_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<ChatSessionRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, project_id, preset_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(preset_id)
        .bind(title)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create chat session")?;

        Ok(ChatSessionRow {
            id,
            project_id: project_id.map(|s| s.to_string()),
            preset_id: preset_id.to_string(),
            title: title.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
            is_archived: false,
            metadata: None,
        })
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Result<Option<ChatSessionRow>> {
        let row = sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, Option<String>)>(
            r#"
            SELECT id, project_id, preset_id, title, created_at, updated_at, is_archived, metadata
            FROM chat_sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get chat session")?;

        Ok(row.map(|r| ChatSessionRow {
            id: r.0,
            project_id: r.1,
            preset_id: r.2,
            title: r.3,
            created_at: DateTime::parse_from_rfc3339(&r.4)
                .unwrap_or_default()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&r.5)
                .unwrap_or_default()
                .with_timezone(&Utc),
            is_archived: r.6 != 0,
            metadata: r.7.and_then(|s| serde_json::from_str(&s).ok()),
        }))
    }

    /// List sessions for a project (or all sessions if project_id is None).
    pub async fn list_sessions(
        &self,
        project_id: Option<&str>,
        include_archived: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatSessionRow>> {
        let rows = if let Some(pid) = project_id {
            if include_archived {
                sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, Option<String>)>(
                    r#"
                    SELECT id, project_id, preset_id, title, created_at, updated_at, is_archived, metadata
                    FROM chat_sessions
                    WHERE project_id = ?
                    ORDER BY updated_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(pid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, Option<String>)>(
                    r#"
                    SELECT id, project_id, preset_id, title, created_at, updated_at, is_archived, metadata
                    FROM chat_sessions
                    WHERE project_id = ? AND is_archived = 0
                    ORDER BY updated_at DESC
                    LIMIT ? OFFSET ?
                    "#,
                )
                .bind(pid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        } else if include_archived {
            sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, Option<String>)>(
                r#"
                SELECT id, project_id, preset_id, title, created_at, updated_at, is_archived, metadata
                FROM chat_sessions
                ORDER BY updated_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, Option<String>, String, Option<String>, String, String, i64, Option<String>)>(
                r#"
                SELECT id, project_id, preset_id, title, created_at, updated_at, is_archived, metadata
                FROM chat_sessions
                WHERE is_archived = 0
                ORDER BY updated_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| ChatSessionRow {
                id: r.0,
                project_id: r.1,
                preset_id: r.2,
                title: r.3,
                created_at: DateTime::parse_from_rfc3339(&r.4)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&r.5)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                is_archived: r.6 != 0,
                metadata: r.7.and_then(|s| serde_json::from_str(&s).ok()),
            })
            .collect())
    }

    /// Update session title.
    pub async fn update_session_title(&self, session_id: &str, title: &str) -> Result<()> {
        sqlx::query("UPDATE chat_sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(&self.pool)
            .await
            .context("Failed to update session title")?;
        Ok(())
    }

    /// Archive a session.
    pub async fn archive_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("UPDATE chat_sessions SET is_archived = 1, updated_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(&self.pool)
            .await
            .context("Failed to archive session")?;
        Ok(())
    }

    /// Delete a session and all its messages.
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete session")?;
        Ok(())
    }

    /// Add a message to a session.
    pub async fn add_message(
        &self,
        session_id: &str,
        role: MessageRole,
        content: &str,
        tool_calls: Option<&[StoredToolCall]>,
        tool_call_id: Option<&str>,
        model_id: Option<&str>,
        token_count: Option<i64>,
    ) -> Result<ChatMessageRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let tool_calls_json = tool_calls.map(|tc| serde_json::to_string(tc).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO chat_messages (id, session_id, role, content, tool_calls, tool_call_id, created_at, token_count, model_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(session_id)
        .bind(role.as_str())
        .bind(content)
        .bind(&tool_calls_json)
        .bind(tool_call_id)
        .bind(now.to_rfc3339())
        .bind(token_count)
        .bind(model_id)
        .execute(&self.pool)
        .await
        .context("Failed to add chat message")?;

        Ok(ChatMessageRow {
            id,
            session_id: session_id.to_string(),
            role,
            content: content.to_string(),
            tool_calls: tool_calls.map(|tc| tc.to_vec()),
            tool_call_id: tool_call_id.map(|s| s.to_string()),
            created_at: now,
            token_count,
            model_id: model_id.map(|s| s.to_string()),
        })
    }

    /// Get all messages for a session.
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<ChatMessageRow>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, Option<i64>, Option<String>)>(
            r#"
            SELECT id, session_id, role, content, tool_calls, tool_call_id, created_at, token_count, model_id
            FROM chat_messages
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get chat messages")?;

        Ok(rows
            .into_iter()
            .map(|r| ChatMessageRow {
                id: r.0,
                session_id: r.1,
                role: MessageRole::from_str(&r.2).unwrap_or(MessageRole::User),
                content: r.3,
                tool_calls: r.4.and_then(|s| serde_json::from_str(&s).ok()),
                tool_call_id: r.5,
                created_at: DateTime::parse_from_rfc3339(&r.6)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                token_count: r.7,
                model_id: r.8,
            })
            .collect())
    }

    /// Get the last N messages for a session.
    pub async fn get_recent_messages(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<ChatMessageRow>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, String, Option<i64>, Option<String>)>(
            r#"
            SELECT id, session_id, role, content, tool_calls, tool_call_id, created_at, token_count, model_id
            FROM chat_messages
            WHERE session_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get recent chat messages")?;

        // Reverse to get chronological order
        let mut messages: Vec<_> = rows
            .into_iter()
            .map(|r| ChatMessageRow {
                id: r.0,
                session_id: r.1,
                role: MessageRole::from_str(&r.2).unwrap_or(MessageRole::User),
                content: r.3,
                tool_calls: r.4.and_then(|s| serde_json::from_str(&s).ok()),
                tool_call_id: r.5,
                created_at: DateTime::parse_from_rfc3339(&r.6)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                token_count: r.7,
                model_id: r.8,
            })
            .collect();

        messages.reverse();
        Ok(messages)
    }

    /// Count messages in a session.
    pub async fn count_messages(&self, session_id: &str) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count messages")?;

        Ok(count)
    }

    /// Run the chat history migration.
    pub async fn migrate(&self) -> Result<()> {
        let migration = include_str!("migrations/003_chat_history.sql");
        sqlx::query(migration)
            .execute(&self.pool)
            .await
            .context("Failed to run chat history migration")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Run migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let history = ChatHistory::new(pool.clone());
        history.migrate().await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_session() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, Some("Test Session"))
            .await
            .unwrap();

        assert_eq!(session.preset_id, "video-analyst");
        assert_eq!(session.title, Some("Test Session".to_string()));
        assert!(!session.is_archived);
    }

    #[tokio::test]
    async fn test_get_session() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("quick-search", None, None)
            .await
            .unwrap();

        let retrieved = history.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().preset_id, "quick-search");
    }

    #[tokio::test]
    async fn test_add_and_get_messages() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, None)
            .await
            .unwrap();

        // Add user message
        history
            .add_message(&session.id, MessageRole::User, "Hello!", None, None, None, None)
            .await
            .unwrap();

        // Add assistant message
        history
            .add_message(
                &session.id,
                MessageRole::Assistant,
                "Hi there!",
                None,
                None,
                Some("gpt-4"),
                Some(15),
            )
            .await
            .unwrap();

        let messages = history.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content, "Hello!");
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert_eq!(messages[1].model_id, Some("gpt-4".to_string()));
    }

    #[tokio::test]
    async fn test_message_with_tool_calls() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, None)
            .await
            .unwrap();

        let tool_calls = vec![StoredToolCall {
            id: "call_123".to_string(),
            name: "search_videos".to_string(),
            arguments: serde_json::json!({"query": "cat video"}),
        }];

        history
            .add_message(
                &session.id,
                MessageRole::Assistant,
                "Let me search for that.",
                Some(&tool_calls),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let messages = history.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].tool_calls.is_some());
        assert_eq!(messages[0].tool_calls.as_ref().unwrap()[0].name, "search_videos");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        history
            .create_session("video-analyst", None, Some("Session 1"))
            .await
            .unwrap();
        history
            .create_session("quick-search", None, Some("Session 2"))
            .await
            .unwrap();

        let sessions = history.list_sessions(None, false, 10, 0).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_archive_session() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, None)
            .await
            .unwrap();

        history.archive_session(&session.id).await.unwrap();

        // Should not appear in non-archived list
        let sessions = history.list_sessions(None, false, 10, 0).await.unwrap();
        assert_eq!(sessions.len(), 0);

        // Should appear with include_archived
        let sessions = history.list_sessions(None, true, 10, 0).await.unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_session_cascades() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, None)
            .await
            .unwrap();

        history
            .add_message(&session.id, MessageRole::User, "Test", None, None, None, None)
            .await
            .unwrap();

        history.delete_session(&session.id).await.unwrap();

        // Session should be gone
        let retrieved = history.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_none());

        // Messages should be gone too (cascade delete)
        let messages = history.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_recent_messages() {
        let pool = setup_test_db().await;
        let history = ChatHistory::new(pool);

        let session = history
            .create_session("video-analyst", None, None)
            .await
            .unwrap();

        for i in 0..5 {
            history
                .add_message(
                    &session.id,
                    MessageRole::User,
                    &format!("Message {}", i),
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .unwrap();
        }

        let recent = history.get_recent_messages(&session.id, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
        // Should be in chronological order (oldest first after reversal)
        assert_eq!(recent[0].content, "Message 2");
        assert_eq!(recent[2].content, "Message 4");
    }
}
