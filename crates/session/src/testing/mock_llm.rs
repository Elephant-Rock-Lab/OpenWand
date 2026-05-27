use openwand_llm::{LlmCapabilities, LlmClient, LlmDelta, LlmError, LlmRequest, LlmResponse, LlmStopReason, LlmStream, LlmTarget};
use openwand_core::snapshots::TokenUsageSnapshot;
use async_trait::async_trait;
use std::collections::VecDeque;
use tokio::sync::Mutex;

/// Mock LLM client that replays a scripted sequence of deltas.
pub struct MockLlmClient {
    script: Mutex<VecDeque<Result<LlmDelta, LlmError>>>,
    requests: Mutex<Vec<LlmRequest>>,
}

impl MockLlmClient {
    /// Create from a scripted sequence of deltas.
    pub fn script(deltas: Vec<Result<LlmDelta, LlmError>>) -> Self {
        Self {
            script: Mutex::new(deltas.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A simple text response followed by Done.
    pub fn text_response(text: &str) -> Self {
        Self::script(vec![
            Ok(LlmDelta::Text { delta: text.into() }),
            Ok(LlmDelta::Done {
                stop_reason: LlmStopReason::Stop,
                usage: Some(TokenUsageSnapshot {
                    input: 10,
                    output: text.len() as u64,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                }),
                provider_message_id: None,
            }),
        ])
    }

    /// A tool call followed by Done with ToolCall stop reason.
    pub fn tool_then_stop(
        tool_call_id: String,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Self {
        Self::script(vec![
            Ok(LlmDelta::ToolCallComplete {
                id: tool_call_id,
                name: tool_name.to_string(),
                arguments,
            }),
            Ok(LlmDelta::Done {
                stop_reason: LlmStopReason::ToolCall,
                usage: Some(TokenUsageSnapshot {
                    input: 20,
                    output: 50,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                }),
                provider_message_id: None,
            }),
        ])
    }

    /// Get all recorded requests.
    pub async fn requests(&self) -> Vec<LlmRequest> {
        self.requests.lock().await.clone()
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        self.requests.lock().await.push(request);

        let deltas: Vec<Result<LlmDelta, LlmError>> =
            self.script.lock().await.drain(..).collect();

        Ok(Box::pin(futures::stream::iter(deltas)))
    }

    async fn complete(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        unimplemented!("01d uses streaming path")
    }

    async fn health_check(&self, _target: &LlmTarget) -> Result<(), LlmError> {
        Ok(())
    }

    fn capabilities(&self, _target: &LlmTarget) -> LlmCapabilities {
        LlmCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            supports_vision: false,
            max_context_tokens: Some(128_000),
            supported_features: vec!["mock".into()],
        }
    }
}
