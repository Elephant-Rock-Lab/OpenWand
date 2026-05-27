//! Deterministic mock LLM client for testing.
//!
//! Only compiled with `#[cfg(feature = "testing")]`.
//! Proves the session-facing LLM stream contract without providers/network/API keys.

use async_trait::async_trait;
use futures::stream;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::client::{LlmClient, LlmStream};
use crate::error::LlmError;
use crate::request::{LlmCapabilities, LlmRequest, LlmTarget};
use crate::response::{LlmDelta, LlmResponse};

/// A scripted action the mock client will perform.
pub enum MockLlmAction {
    /// Yield a sequence of deltas (or errors) as a stream.
    Stream(Vec<Result<LlmDelta, LlmError>>),
    /// Return a complete response (or error).
    Complete(Result<LlmResponse, LlmError>),
    /// Health check result.
    Health(Result<(), LlmError>),
}

/// Deterministic mock LLM client.
///
/// Tests push scripted actions, then the client replays them in order.
/// If a script queue is empty, returns a safe error.
pub struct MockLlmClient {
    actions: Arc<Mutex<VecDeque<MockLlmAction>>>,
    capabilities: LlmCapabilities,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(Mutex::new(VecDeque::new())),
            capabilities: LlmCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: true,
                supports_vision: false,
                max_context_tokens: Some(128_000),
                supported_features: vec!["streaming".into(), "tools".into()],
            },
        }
    }

    /// Create a mock that will stream the given deltas on first `chat_stream` call.
    pub fn with_stream(deltas: Vec<LlmDelta>) -> Self {
        let client = Self::new();
        client.push_stream(deltas);
        client
    }

    /// Create a mock that will stream the given results (allows errors in stream).
    pub fn with_stream_results(results: Vec<Result<LlmDelta, LlmError>>) -> Self {
        let client = Self::new();
        client.push_stream_results(results);
        client
    }

    /// Create a mock that will return the given response on first `complete` call.
    pub fn with_response(response: LlmResponse) -> Self {
        let client = Self::new();
        client.push_response(response);
        client
    }

    /// Push a stream script (all deltas are Ok).
    pub fn push_stream(&self, deltas: Vec<LlmDelta>) {
        let results: Vec<Result<LlmDelta, LlmError>> =
            deltas.into_iter().map(Ok).collect();
        self.push_stream_results(results);
    }

    /// Push a stream script with explicit results (allows errors mid-stream).
    pub fn push_stream_results(&self, results: Vec<Result<LlmDelta, LlmError>>) {
        let mut actions = self.actions.lock().unwrap();
        actions.push_back(MockLlmAction::Stream(results));
    }

    /// Push a complete response.
    pub fn push_response(&self, response: LlmResponse) {
        let mut actions = self.actions.lock().unwrap();
        actions.push_back(MockLlmAction::Complete(Ok(response)));
    }

    fn pop_stream_script(&self) -> Option<Vec<Result<LlmDelta, LlmError>>> {
        let mut actions = self.actions.lock().unwrap();
        loop {
            let action = actions.pop_front()?;
            match action {
                MockLlmAction::Stream(items) => return Some(items),
                // Skip non-stream actions, keep looking
                _ => continue,
            }
        }
    }

    fn pop_complete_script(&self) -> Option<Result<LlmResponse, LlmError>> {
        let mut actions = self.actions.lock().unwrap();
        loop {
            let action = actions.pop_front()?;
            match action {
                MockLlmAction::Complete(result) => return Some(result),
                _ => continue,
            }
        }
    }

    fn pop_health_script(&self) -> Option<Result<(), LlmError>> {
        let mut actions = self.actions.lock().unwrap();
        loop {
            let action = actions.pop_front()?;
            match action {
                MockLlmAction::Health(result) => return Some(result),
                _ => continue,
            }
        }
    }
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn chat_stream(&self, _request: LlmRequest) -> Result<LlmStream, LlmError> {
        match self.pop_stream_script() {
            Some(items) => {
                let stream = stream::iter(items);
                Ok(Box::pin(stream))
            }
            None => Err(LlmError::RequestInvalid {
                message: "MockLlmClient: no stream script queued".into(),
            }),
        }
    }

    async fn complete(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        match self.pop_complete_script() {
            Some(result) => result,
            None => Err(LlmError::RequestInvalid {
                message: "MockLlmClient: no complete script queued".into(),
            }),
        }
    }

    async fn health_check(&self, _target: &LlmTarget) -> Result<(), LlmError> {
        match self.pop_health_script() {
            Some(result) => result,
            None => Ok(()), // Default: healthy
        }
    }

    fn capabilities(&self, _target: &LlmTarget) -> LlmCapabilities {
        self.capabilities.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{LlmContent, LlmMessage, LlmProvider, LlmTarget};
    use crate::response::LlmStopReason;
    use futures::StreamExt;
    use openwand_core::snapshots::TokenUsageSnapshot;

    fn test_request() -> LlmRequest {
        LlmRequest {
            target: LlmTarget {
                provider: LlmProvider::OpenAI,
                model: "mock".into(),
                base_url: None,
                api_key: None,
            },
            system_prompt: String::new(),
            messages: vec![LlmMessage::User {
                content: vec![LlmContent::Text("test".into())],
            }],
            tools: vec![],
            thinking_budget: None,
            max_tokens: None,
            temperature: None,
            tool_choice: None,
            provider_options: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn mock_llm_stream_text() {
        let client = MockLlmClient::with_stream(vec![
            LlmDelta::Text { delta: "Hel".into() },
            LlmDelta::Text { delta: "lo".into() },
            LlmDelta::Text { delta: "!".into() },
            LlmDelta::Done {
                stop_reason: LlmStopReason::Stop,
                usage: Some(TokenUsageSnapshot {
                    input: 1,
                    output: 3,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                }),
                provider_message_id: Some("mock_msg_1".into()),
            },
        ]);

        let stream = client.chat_stream(test_request()).await.unwrap();
        let items: Vec<_> = stream.collect().await;

        assert_eq!(4, items.len());

        // Collect text
        let mut text = String::new();
        for item in &items {
            if let Ok(LlmDelta::Text { delta }) = item {
                text.push_str(delta);
            }
        }
        assert_eq!("Hello!", text);

        // Last should be Done
        match &items[3] {
            Ok(LlmDelta::Done { stop_reason, .. }) => {
                assert!(matches!(stop_reason, LlmStopReason::Stop));
            }
            _ => panic!("expected Done delta"),
        }
    }

    #[tokio::test]
    async fn mock_llm_stream_preserves_order() {
        let client = MockLlmClient::with_stream(vec![
            LlmDelta::Text { delta: "A".into() },
            LlmDelta::Text { delta: "B".into() },
            LlmDelta::Text { delta: "C".into() },
        ]);

        let stream = client.chat_stream(test_request()).await.unwrap();
        let items: Vec<_> = stream.collect().await;

        let letters: Vec<&str> = items
            .iter()
            .filter_map(|i| match i {
                Ok(LlmDelta::Text { delta }) => Some(delta.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(vec!["A", "B", "C"], letters);
    }

    #[tokio::test]
    async fn mock_llm_stream_tool_call_complete() {
        let client = MockLlmClient::with_stream(vec![
            LlmDelta::ToolCallStart {
                id: "tc_1".into(),
                name: Some("read_file".into()),
            },
            LlmDelta::ToolCallArgsDelta {
                id: "tc_1".into(),
                delta: "{\"path\":".into(),
            },
            LlmDelta::ToolCallArgsDelta {
                id: "tc_1".into(),
                delta: "\"/tmp\"}".into(),
            },
            LlmDelta::ToolCallComplete {
                id: "tc_1".into(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "/tmp"}),
            },
            LlmDelta::Done {
                stop_reason: LlmStopReason::ToolCall,
                usage: Some(TokenUsageSnapshot {
                    input: 500,
                    output: 20,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                }),
                provider_message_id: None,
            },
        ]);

        let stream = client.chat_stream(test_request()).await.unwrap();
        let items: Vec<_> = stream.collect().await;

        assert_eq!(5, items.len());

        // Verify ToolCallComplete has full arguments
        match &items[3] {
            Ok(LlmDelta::ToolCallComplete {
                id,
                name,
                arguments,
            }) => {
                assert_eq!("tc_1", id);
                assert_eq!("read_file", name);
                assert_eq!("/tmp", arguments["path"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }
    }

    #[tokio::test]
    async fn mock_llm_stream_error_item() {
        let client = MockLlmClient::with_stream_results(vec![
            Ok(LlmDelta::Text { delta: "partial".into() }),
            Err(LlmError::Stream {
                message: "connection lost".into(),
                partial: true,
            }),
        ]);

        let stream = client.chat_stream(test_request()).await.unwrap();
        let items: Vec<_> = stream.collect().await;

        assert_eq!(2, items.len());
        assert!(items[0].is_ok());
        assert!(items[1].is_err());
        match &items[1] {
            Err(LlmError::Stream { partial, .. }) => assert!(partial),
            _ => panic!("expected Stream error"),
        }
    }

    #[tokio::test]
    async fn mock_llm_complete_returns_response() {
        let client = MockLlmClient::with_response(LlmResponse {
            content: vec![LlmContent::Text("Hello!".into())],
            usage: TokenUsageSnapshot {
                input: 10,
                output: 5,
                reasoning: None,
                cache_read: None,
                cache_write: None,
            },
            stop_reason: LlmStopReason::Stop,
            provider_message_id: Some("msg_1".into()),
        });

        let resp = client.complete(test_request()).await.unwrap();
        assert_eq!(1, resp.content.len());
        assert!(matches!(resp.stop_reason, LlmStopReason::Stop));
    }

    #[tokio::test]
    async fn mock_llm_complete_returns_error() {
        let client = MockLlmClient::new();
        // No script queued → should error
        let result = client.complete(test_request()).await;
        assert!(result.is_err());
        match result {
            Err(LlmError::RequestInvalid { message }) => {
                assert!(message.contains("no complete script"));
            }
            _ => panic!("expected RequestInvalid"),
        }
    }

    #[tokio::test]
    async fn mock_llm_health_check_returns_ok() {
        let client = MockLlmClient::new();
        let target = LlmTarget {
            provider: LlmProvider::OpenAI,
            model: "gpt-4o".into(),
            base_url: None,
            api_key: None,
        };
        let result = client.health_check(&target).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn mock_llm_health_check_returns_error() {
        let client = MockLlmClient::new();
        {
            let mut actions = client.actions.lock().unwrap();
            actions.push_back(MockLlmAction::Health(Err(LlmError::Network {
                message: "timeout".into(),
                retryable: true,
            })));
        }
        let target = LlmTarget {
            provider: LlmProvider::Anthropic,
            model: "claude".into(),
            base_url: None,
            api_key: None,
        };
        let result = client.health_check(&target).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_llm_capabilities_return_configured() {
        let client = MockLlmClient::new();
        let target = LlmTarget {
            provider: LlmProvider::Ollama,
            model: "llama3".into(),
            base_url: None,
            api_key: None,
        };
        let caps = client.capabilities(&target);
        assert!(caps.supports_streaming);
        assert!(caps.supports_tools);
        assert_eq!(Some(128_000), caps.max_context_tokens);
    }

    #[tokio::test]
    async fn mock_llm_empty_script_errors_safely() {
        let client = MockLlmClient::new();
        // No stream script queued
        let result = client.chat_stream(test_request()).await;
        assert!(result.is_err());
        match result {
            Err(LlmError::RequestInvalid { message }) => {
                // Must not leak internal details beyond "no script"
                assert!(message.contains("no stream script"));
                assert!(!message.contains("panic"));
            }
            _ => panic!("expected RequestInvalid"),
        }
    }

    #[tokio::test]
    async fn mock_llm_trait_object_streams() {
        // Prove MockLlmClient works behind Arc<dyn LlmClient>
        let client: std::sync::Arc<dyn LlmClient> =
            std::sync::Arc::new(MockLlmClient::with_stream(vec![
                LlmDelta::Text { delta: "works".into() },
                LlmDelta::Done {
                    stop_reason: LlmStopReason::Stop,
                    usage: None,
                    provider_message_id: None,
                },
            ]));

        let stream = client.chat_stream(test_request()).await.unwrap();
        let items: Vec<_> = stream.collect().await;
        assert_eq!(2, items.len());
    }
}
