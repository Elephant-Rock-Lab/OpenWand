use openwand_llm::{LlmCapabilities, LlmClient, LlmDelta, LlmError, LlmRequest, LlmResponse, LlmStopReason, LlmStream, LlmTarget};
use openwand_core::snapshots::TokenUsageSnapshot;
use async_trait::async_trait;
use std::collections::VecDeque;
use tokio::sync::Mutex;

/// Mock LLM client that replays scripted turns.
///
/// Each "turn" is a sequence of deltas ending with a `Done` delta.
/// On each `chat_stream` call, one turn is consumed.
pub struct MockLlmClient {
    turns: Mutex<VecDeque<Vec<Result<LlmDelta, LlmError>>>>,
    requests: Mutex<Vec<LlmRequest>>,
}

impl MockLlmClient {
    /// Create from pre-split turns (each inner Vec is one turn ending in Done).
    fn from_turns(turns: Vec<Vec<Result<LlmDelta, LlmError>>>) -> Self {
        Self {
            turns: Mutex::new(turns.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A simple text response followed by Done.
    pub fn text_response(text: &str) -> Self {
        Self::from_turns(vec![vec![
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
        ]])
    }

    /// A tool call followed by Done with ToolCall stop reason.
    pub fn tool_then_stop(
        tool_call_id: String,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Self {
        Self::from_turns(vec![vec![
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
        ]])
    }

    /// Multiple tool calls in one turn, followed by Done with ToolCall stop reason.
    pub fn multi_tool_then_stop(
        calls: Vec<(String, String, serde_json::Value)>,
    ) -> Self {
        let mut deltas: Vec<Result<LlmDelta, LlmError>> = calls
            .into_iter()
            .map(|(id, name, args)| {
                Ok(LlmDelta::ToolCallComplete {
                    id,
                    name,
                    arguments: args,
                })
            })
            .collect();
        deltas.push(Ok(LlmDelta::Done {
            stop_reason: LlmStopReason::ToolCall,
            usage: Some(TokenUsageSnapshot {
                input: 20,
                output: 50,
                reasoning: None,
                cache_read: None,
                cache_write: None,
            }),
            provider_message_id: None,
        }));
        Self::from_turns(vec![deltas])
    }

    /// First turn returns a tool call, second turn returns text.
    /// For testing rejection→continuation: denial feeds back, model continues.
    pub fn tool_then_text_after_denial(
        tool_call_id: String,
        tool_name: &str,
        arguments: serde_json::Value,
        fallback_text: &str,
    ) -> Self {
        Self::from_turns(vec![
            // Turn 1: tool call
            vec![
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
            ],
            // Turn 2: text response after denial
            vec![
                Ok(LlmDelta::Text { delta: fallback_text.into() }),
                Ok(LlmDelta::Done {
                    stop_reason: LlmStopReason::Stop,
                    usage: Some(TokenUsageSnapshot {
                        input: 30,
                        output: fallback_text.len() as u64,
                        reasoning: None,
                        cache_read: None,
                        cache_write: None,
                    }),
                    provider_message_id: None,
                }),
            ],
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

        let turn = self
            .turns
            .lock()
            .await
            .pop_front()
            .unwrap_or_default();

        Ok(Box::pin(futures::stream::iter(turn)))
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
