//! Test VLM backend for mocking inference responses.
//!
//! Provides configurable mock responses for testing VLM integration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;

/// Test VLM backend with configurable responses.
#[derive(Clone)]
pub struct TestVlmBackend {
    inner: Arc<TestVlmBackendInner>,
}

struct TestVlmBackendInner {
    /// Canned responses for specific prompts.
    responses: RwLock<HashMap<String, Vec<String>>>,
    /// Default response when no match found.
    default_response: RwLock<String>,
    /// Simulated latency per inference.
    latency: RwLock<Duration>,
    /// Whether to simulate failures.
    fail_mode: RwLock<FailMode>,
    /// Call history for verification.
    call_history: RwLock<Vec<InferenceCall>>,
}

/// Failure simulation mode.
#[derive(Debug, Clone, Copy, Default)]
pub enum FailMode {
    /// Normal operation.
    #[default]
    None,
    /// Fail all requests.
    All,
    /// Fail every Nth request.
    EveryN(usize),
    /// Fail with timeout.
    Timeout,
}

/// Record of an inference call.
#[derive(Debug, Clone)]
pub struct InferenceCall {
    /// The prompt that was sent.
    pub prompt: String,
    /// Image dimensions if provided.
    pub image_size: Option<(u32, u32)>,
    /// Request ID.
    pub request_id: String,
}

impl TestVlmBackend {
    /// Create a new test VLM backend.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TestVlmBackendInner {
                responses: RwLock::new(HashMap::new()),
                default_response: RwLock::new(
                    "Test VLM response: Analysis of visual content.".to_string(),
                ),
                latency: RwLock::new(Duration::from_millis(10)),
                fail_mode: RwLock::new(FailMode::None),
                call_history: RwLock::new(Vec::new()),
            }),
        }
    }

    /// Add a canned response for a prompt pattern.
    ///
    /// The pattern is matched case-insensitively against the prompt.
    pub fn add_response(&self, pattern: impl Into<String>, responses: Vec<String>) {
        let mut map = self.inner.responses.write();
        map.insert(pattern.into().to_lowercase(), responses);
    }

    /// Set the default response when no pattern matches.
    pub fn set_default_response(&self, response: impl Into<String>) {
        *self.inner.default_response.write() = response.into();
    }

    /// Set the simulated latency.
    pub fn set_latency(&self, latency: Duration) {
        *self.inner.latency.write() = latency;
    }

    /// Set the failure mode.
    pub fn set_fail_mode(&self, mode: FailMode) {
        *self.inner.fail_mode.write() = mode;
    }

    /// Get inference for a prompt.
    ///
    /// Returns the response text or an error if in fail mode.
    pub async fn infer(
        &self,
        request_id: &str,
        prompt: &str,
        image_size: Option<(u32, u32)>,
    ) -> Result<String, String> {
        // Record the call
        self.inner.call_history.write().push(InferenceCall {
            prompt: prompt.to_string(),
            image_size,
            request_id: request_id.to_string(),
        });

        // Check fail mode
        let call_count = self.inner.call_history.read().len();
        match *self.inner.fail_mode.read() {
            FailMode::All => return Err("Simulated failure".to_string()),
            FailMode::EveryN(n) if call_count % n == 0 => {
                return Err(format!("Simulated failure on call {}", call_count));
            }
            FailMode::Timeout => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                return Err("Timeout".to_string());
            }
            _ => {}
        }

        // Simulate latency
        let latency = *self.inner.latency.read();
        if latency > Duration::ZERO {
            tokio::time::sleep(latency).await;
        }

        // Find matching response
        let prompt_lower = prompt.to_lowercase();
        let responses = self.inner.responses.read();

        for (pattern, response_list) in responses.iter() {
            if prompt_lower.contains(pattern) {
                // Return a response based on call count
                let idx = (call_count - 1) % response_list.len();
                return Ok(response_list[idx].clone());
            }
        }

        // Return default response
        Ok(self.inner.default_response.read().clone())
    }

    /// Get the call history.
    pub fn call_history(&self) -> Vec<InferenceCall> {
        self.inner.call_history.read().clone()
    }

    /// Clear the call history.
    pub fn clear_history(&self) {
        self.inner.call_history.write().clear();
    }

    /// Get the number of calls made.
    pub fn call_count(&self) -> usize {
        self.inner.call_history.read().len()
    }

    /// Check if a prompt was called.
    pub fn was_called_with(&self, prompt_substring: &str) -> bool {
        self.inner
            .call_history
            .read()
            .iter()
            .any(|c| c.prompt.contains(prompt_substring))
    }
}

impl Default for TestVlmBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_inference() {
        let backend = TestVlmBackend::new();
        let result = backend.infer("req-1", "Describe this image", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_canned_response() {
        let backend = TestVlmBackend::new();
        backend.add_response("person", vec!["I see a person".to_string()]);

        let result = backend.infer("req-1", "Find person in frame", None).await;
        assert_eq!(result.unwrap(), "I see a person");
    }

    #[tokio::test]
    async fn test_fail_mode() {
        let backend = TestVlmBackend::new();
        backend.set_fail_mode(FailMode::All);

        let result = backend.infer("req-1", "Test", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_history() {
        let backend = TestVlmBackend::new();
        backend.infer("req-1", "First prompt", None).await.ok();
        backend
            .infer("req-2", "Second prompt", Some((640, 480)))
            .await
            .ok();

        assert_eq!(backend.call_count(), 2);
        assert!(backend.was_called_with("First"));
        assert!(backend.was_called_with("Second"));

        let history = backend.call_history();
        assert_eq!(history[1].image_size, Some((640, 480)));
    }
}
