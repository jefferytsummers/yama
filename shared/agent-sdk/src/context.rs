//! Context management for agent conversations.
//!
//! This module handles the sliding context window, message history,
//! and overflow strategies for long conversations.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::tools::ToolResult;

/// Context window configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextConfig {
    /// Maximum tokens in the context window.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Strategy for handling context overflow.
    #[serde(default)]
    pub overflow_strategy: OverflowStrategy,
    /// Number of tokens to keep when truncating.
    #[serde(default = "default_keep_tokens")]
    pub keep_tokens: u32,
}

fn default_max_tokens() -> u32 {
    100_000
}

fn default_keep_tokens() -> u32 {
    50_000
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            overflow_strategy: OverflowStrategy::default(),
            keep_tokens: default_keep_tokens(),
        }
    }
}

/// Strategy for handling context overflow.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowStrategy {
    /// Slide the window, removing oldest messages.
    #[default]
    SlidingWindow,
    /// Summarize older messages.
    Summarize,
    /// Truncate the oldest messages.
    TruncateOldest,
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender.
    pub role: MessageRole,
    /// Content of the message.
    pub content: MessageContent,
    /// Estimated token count.
    pub token_count: u32,
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool result.
    Tool,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Structured content (for tool use, images, etc.).
    Structured(Vec<ContentBlock>),
}

/// A block of content within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text { text: String },
    /// Tool use request.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result.
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    /// Image content.
    Image {
        source: ImageSource,
        media_type: String,
    },
}

/// Source of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image.
    Base64 { data: String },
    /// URL to an image.
    Url { url: String },
}

/// The full context for a conversation.
#[derive(Debug, Clone)]
pub struct Context {
    /// System prompt (always included).
    pub system: String,
    /// Message history.
    pub messages: Vec<Message>,
    /// Total token count.
    pub total_tokens: u32,
}

/// Manages the context window for a single connection.
pub struct ContextManager {
    /// Configuration.
    config: ContextConfig,
    /// Message history.
    messages: VecDeque<Message>,
    /// Current token count.
    token_count: u32,
}

impl ContextManager {
    /// Create a new context manager.
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            messages: VecDeque::new(),
            token_count: 0,
        }
    }

    /// Add a user message to the context.
    pub fn add_user_message(&mut self, content: &str) {
        let token_count = estimate_tokens(content);
        self.add_message(Message {
            role: MessageRole::User,
            content: MessageContent::Text(content.to_string()),
            token_count,
        });
    }

    /// Add an assistant message to the context.
    pub fn add_assistant_message(
        &mut self,
        text: &str,
        tool_uses: &[(String, String, serde_json::Value)],
    ) {
        let content = if tool_uses.is_empty() {
            MessageContent::Text(text.to_string())
        } else {
            let mut blocks = Vec::new();
            if !text.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: text.to_string(),
                });
            }
            for (id, name, input) in tool_uses {
                blocks.push(ContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
            }
            MessageContent::Structured(blocks)
        };

        let token_count = estimate_tokens(text)
            + tool_uses.iter().map(|(_, _, i)| estimate_tokens(&i.to_string())).sum::<u32>();

        self.add_message(Message {
            role: MessageRole::Assistant,
            content,
            token_count,
        });
    }

    /// Add a tool result to the context.
    pub fn add_tool_result(&mut self, tool_use_id: &str, result: &ToolResult) {
        let (content, is_error) = match result {
            ToolResult::Success(s) => (s.clone(), false),
            ToolResult::Error(e) => (e.clone(), true),
        };

        let token_count = estimate_tokens(&content);

        self.add_message(Message {
            role: MessageRole::Tool,
            content: MessageContent::Structured(vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content,
                is_error,
            }]),
            token_count,
        });
    }

    /// Add a message, applying overflow strategy if needed.
    fn add_message(&mut self, message: Message) {
        self.token_count += message.token_count;
        self.messages.push_back(message);

        // Apply overflow strategy if needed
        while self.token_count > self.config.max_tokens {
            self.apply_overflow_strategy();
        }
    }

    /// Apply the configured overflow strategy.
    fn apply_overflow_strategy(&mut self) {
        match self.config.overflow_strategy {
            OverflowStrategy::SlidingWindow | OverflowStrategy::TruncateOldest => {
                // Remove oldest messages until we're under the limit
                while self.token_count > self.config.keep_tokens {
                    if let Some(msg) = self.messages.pop_front() {
                        self.token_count = self.token_count.saturating_sub(msg.token_count);
                        debug!(
                            "Removed message with {} tokens, {} remaining",
                            msg.token_count, self.token_count
                        );
                    } else {
                        break;
                    }
                }
            }
            OverflowStrategy::Summarize => {
                // For now, fall back to truncation
                // TODO: Implement actual summarization
                warn!("Summarization not yet implemented, falling back to truncation");
                while self.token_count > self.config.keep_tokens {
                    if let Some(msg) = self.messages.pop_front() {
                        self.token_count = self.token_count.saturating_sub(msg.token_count);
                    } else {
                        break;
                    }
                }
            }
        }
    }

    /// Build the messages list for an LLM call.
    pub fn build_messages(&self, system_prompt: &str) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.messages.len() + 1);

        // System message first
        messages.push(Message {
            role: MessageRole::System,
            content: MessageContent::Text(system_prompt.to_string()),
            token_count: estimate_tokens(system_prompt),
        });

        // Then all conversation messages
        messages.extend(self.messages.iter().cloned());

        messages
    }

    /// Get the current token count.
    pub fn token_count(&self) -> u32 {
        self.token_count
    }

    /// Get the number of messages.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.token_count = 0;
    }
}

/// Estimate the number of tokens in a string.
///
/// This is a rough estimate based on character count.
/// For more accurate counts, use a proper tokenizer.
fn estimate_tokens(text: &str) -> u32 {
    // Rough estimate: ~4 characters per token
    (text.len() / 4).max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_basic() {
        let config = ContextConfig::default();
        let mut ctx = ContextManager::new(config);

        ctx.add_user_message("Hello, world!");
        assert_eq!(ctx.message_count(), 1);

        ctx.add_assistant_message("Hi there!", &[]);
        assert_eq!(ctx.message_count(), 2);
    }

    #[test]
    fn test_context_overflow() {
        let config = ContextConfig {
            max_tokens: 100,
            overflow_strategy: OverflowStrategy::SlidingWindow,
            keep_tokens: 50,
        };
        let mut ctx = ContextManager::new(config);

        // Add messages until we overflow
        for i in 0..20 {
            ctx.add_user_message(&format!("Message {i} with some content"));
        }

        // Should have removed some messages
        assert!(ctx.token_count() <= 100);
    }

    #[test]
    fn test_build_messages() {
        let config = ContextConfig::default();
        let mut ctx = ContextManager::new(config);

        ctx.add_user_message("Hello");
        ctx.add_assistant_message("Hi!", &[]);

        let messages = ctx.build_messages("You are helpful.");
        assert_eq!(messages.len(), 3); // system + user + assistant
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
        assert_eq!(messages[2].role, MessageRole::Assistant);
    }
}
