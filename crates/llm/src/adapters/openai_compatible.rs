//! OpenAI-compatible adapter.
//!
//! Works with OpenAI, LM Studio, Ollama, and any server that implements
//! the `/v1/chat/completions` endpoint with SSE streaming.
//!
//! No Rig dependency — uses reqwest directly for maximum simplicity
//! and minimum generic type complexity.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::Mutex;

use crate::client::{LlmClient, LlmStream};
use crate::error::LlmError;
use crate::request::{
    LlmCapabilities, LlmContent, LlmMessage, LlmRequest, LlmTarget, LlmToolChoice,
};
use crate::response::{LlmDelta, LlmResponse, LlmStopReason};
use crate::tool_buffer::ToolCallBuffer;
use openwand_core::snapshots::TokenUsageSnapshot;

/// OpenAI-compatible provider client.
pub struct OpenAiCompatibleClient {
    http: Client,
    buffer: Arc<Mutex<ToolCallBuffer>>,
}

impl OpenAiCompatibleClient {
    /// Fallible constructor for production callers.
    ///
    /// Returns `Err` if the HTTP client cannot be built (e.g., TLS backend missing).
    /// Production code should call this and propagate the error to the user.
    pub fn try_new() -> Result<Self, LlmError> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::Network {
                message: format!("Failed to build HTTP client: {e}"),
                retryable: false,
            })?;
        Ok(Self {
            http,
            buffer: Arc::new(Mutex::new(ToolCallBuffer::new())),
        })
    }

    /// Panicking constructor for test code only.
    /// Production callers must use `try_new()`.
    pub fn new() -> Self {
        Self::try_new().expect("OpenAiCompatibleClient::new() failed — use try_new() in production")
    }

    /// Build the full URL for chat completions.
    fn completions_url(target: &LlmTarget) -> String {
        let base = target
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{}/chat/completions", base.trim_end_matches('/'))
    }

    /// Convert an OpenWand LlmRequest into an OpenAI-compatible JSON body.
    pub fn build_request_body(request: &LlmRequest) -> serde_json::Value {
        let mut messages = Vec::new();

        // System prompt
        if !request.system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system_prompt
            }));
        }

        // Conversation history
        for msg in &request.messages {
            match msg {
                LlmMessage::User { content } => {
                    let text: String = content
                        .iter()
                        .filter_map(|c| match c {
                            LlmContent::Text(t) => Some(t.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": text
                    }));
                }
                LlmMessage::Assistant { content } => {
                    let mut assistant_content = Vec::new();
                    let mut tool_calls = Vec::new();
                    for c in content {
                        match c {
                            LlmContent::Text(t) => {
                                assistant_content.push(serde_json::json!(t));
                            }
                            LlmContent::ToolCall { id, name, arguments } => {
                                tool_calls.push(serde_json::json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": arguments.to_string()
                                    }
                                }));
                            }
                            _ => {}
                        }
                    }
                    let mut msg = serde_json::json!({
                        "role": "assistant",
                    });
                    let text_content: String = assistant_content
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join("");
                    if !text_content.is_empty() {
                        msg["content"] = serde_json::json!(text_content);
                    }
                    if !tool_calls.is_empty() {
                        msg["tool_calls"] = serde_json::json!(tool_calls);
                    }
                    messages.push(msg);
                }
                LlmMessage::Tool {
                    tool_call_id,
                    content,
                    is_error: _is_error,
                } => {
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call_id,
                        "content": content
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": request.target.model,
            "messages": messages,
            "stream": true,
        });

        // Tools
        if !request.tools.is_empty() {
            let tools: Vec<_> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters_schema,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        // Tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                LlmToolChoice::Auto => serde_json::json!("auto"),
                LlmToolChoice::None => serde_json::json!("none"),
                LlmToolChoice::Required => serde_json::json!("required"),
            };
        }

        // Optional params
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        body
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let url = Self::completions_url(&request.target);
        let body = Self::build_request_body(&request);

        let mut http_req = self.http.post(&url);
        http_req = http_req.json(&body);

        if let Some(ref key) = request.target.api_key {
            http_req = http_req.bearer_auth(key);
        }

        let resp = http_req
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
                retryable: true,
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Provider {
                provider: "openai-compatible".into(),
                message: format!("HTTP {status}: {text}"),
                retryable: status.is_server_error(),
            });
        }

        // Parse SSE stream
        let buffer = self.buffer.clone();
        let stream = parse_sse_stream(resp, buffer);

        Ok(Box::pin(stream))
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let mut stream = self.chat_stream(request).await?;
        let mut content = Vec::new();
        let mut usage = TokenUsageSnapshot {
            input: 0,
            output: 0,
            reasoning: None,
            cache_read: None,
            cache_write: None,
        };
        let mut stop_reason = LlmStopReason::Stop;

        while let Some(delta) = stream.next().await {
            match delta? {
                LlmDelta::Text { delta } => {
                    if !delta.is_empty() {
                        content.push(LlmContent::Text(delta));
                    }
                }
                LlmDelta::ToolCallComplete { id, name, arguments } => {
                    content.push(LlmContent::ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
                LlmDelta::Done {
                    stop_reason: sr,
                    usage: u,
                    ..
                } => {
                    stop_reason = sr;
                    if let Some(u) = u {
                        usage = u;
                    }
                }
                _ => {}
            }
        }

        Ok(LlmResponse {
            content,
            usage,
            stop_reason,
            provider_message_id: None,
        })
    }

    async fn health_check(&self, target: &LlmTarget) -> Result<(), LlmError> {
        let base = target
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/models", base.trim_end_matches('/'));

        let mut req = self.http.get(&url);
        if let Some(ref key) = target.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
                retryable: true,
            })?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::Provider {
                provider: "openai-compatible".into(),
                message: format!("health check failed: HTTP {}", resp.status()),
                retryable: false,
            })
        }
    }

    fn capabilities(&self, _target: &LlmTarget) -> LlmCapabilities {
        LlmCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            supports_vision: false,
            max_context_tokens: None,
            supported_features: vec![
                "streaming".into(),
                "tools".into(),
            ],
        }
    }
}

/// Parse SSE stream from OpenAI-compatible endpoint into LlmDelta stream.
fn parse_sse_stream(
    resp: reqwest::Response,
    buffer: Arc<Mutex<ToolCallBuffer>>,
) -> impl Stream<Item = Result<LlmDelta, LlmError>> {
    let stream = resp.bytes_stream();

    stream
        .scan(
            (String::new(), buffer),
            |(pending, buf), chunk| {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        return std::future::ready(Some(vec![Err(LlmError::Stream {
                            message: e.to_string(),
                            partial: true,
                        })]));
                    }
                };

                pending.push_str(&String::from_utf8_lossy(&chunk));

                let mut deltas = Vec::new();

                // Process complete SSE lines
                while let Some(pos) = pending.find("\n\n") {
                    let block = pending[..pos].to_string();
                    pending.drain(..pos + 2);

                    for line in block.lines() {
                        let line = line.trim();
                        if !line.starts_with("data: ") {
                            continue;
                        }
                        let data = &line[6..];
                        if data == "[DONE]" {
                            deltas.push(Ok(LlmDelta::Done {
                                stop_reason: LlmStopReason::Stop,
                                usage: None,
                                provider_message_id: None,
                            }));
                            continue;
                        }

                        match serde_json::from_str::<SseChunk>(data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(ref delta) = choice.delta {
                                        // Text content
                                        if let Some(ref content) = delta.content {
                                            if !content.is_empty() {
                                                deltas.push(Ok(LlmDelta::Text {
                                                    delta: content.clone(),
                                                }));
                                            }
                                        }

                                        // Tool calls — buffer start/args, emit on flush
                                        if let Some(ref tool_calls) = delta.tool_calls {
                                            for tc in tool_calls {
                                                let id = tc.id.clone().unwrap_or_default();
                                                if let Some(ref func) = tc.function {
                                                    // Feed into the shared buffer
                                                    if let Ok(mut locked_buf) = buf.lock() {
                                                        if let Some(ref name) = func.name {
                                                            let _ = locked_buf.handle_start(
                                                                id.clone(),
                                                                Some(name.clone()),
                                                            );
                                                        }
                                                        if let Some(ref args) = func.arguments {
                                                            if !args.is_empty() {
                                                                let _ = locked_buf.handle_args_delta(
                                                                    id.clone(),
                                                                    args.clone(),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Finish reason
                                    if let Some(ref reason) = choice.finish_reason {
                                        match reason.as_str() {
                                            "tool_calls" => {
                                                // Flush all buffered tool calls into ToolCallComplete deltas
                                                if let Ok(mut locked_buf) = buf.lock() {
                                                    let ids: Vec<String> = locked_buf.drain_ids();
                                                    for id in ids {
                                                        match locked_buf.complete(&id) {
                                                            Ok(complete_delta) => {
                                                                deltas.push(Ok(complete_delta));
                                                            }
                                                            Err(e) => {
                                                                tracing::warn!(
                                                                    "Failed to complete tool call {}: {e}",
                                                                    id
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                                deltas.push(Ok(LlmDelta::Done {
                                                    stop_reason: LlmStopReason::ToolCall,
                                                    usage: None,
                                                    provider_message_id: None,
                                                }));
                                            }
                                            "stop" | "end_turn" => {
                                                deltas.push(Ok(LlmDelta::Done {
                                                    stop_reason: LlmStopReason::Stop,
                                                    usage: None,
                                                    provider_message_id: None,
                                                }));
                                            }
                                            "length" => {
                                                deltas.push(Ok(LlmDelta::Done {
                                                    stop_reason: LlmStopReason::Length,
                                                    usage: None,
                                                    provider_message_id: None,
                                                }));
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                // Usage
                                if let Some(u) = chunk.usage {
                                    deltas.push(Ok(LlmDelta::Done {
                                        stop_reason: LlmStopReason::Stop,
                                        usage: Some(TokenUsageSnapshot {
                                            input: u.prompt_tokens as u64,
                                            output: u.completion_tokens as u64,
                                            reasoning: None,
                                            cache_read: None,
                                            cache_write: None,
                                        }),
                                        provider_message_id: chunk.id.clone(),
                                    }));
                                }
                            }
                            Err(e) => {
                                // Non-fatal: some providers send comments or non-JSON lines
                                tracing::debug!("SSE parse error (non-fatal): {e}");
                            }
                        }
                    }
                }

                std::future::ready(Some(deltas))
            },
        )
        .map(futures::stream::iter)
        .flatten()
}

// ---- SSE deserialization types ----

#[derive(Debug, Deserialize)]
struct SseChunk {
    choices: Vec<SseChoice>,
    usage: Option<SseUsage>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseChoice {
    delta: Option<SseDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    content: Option<String>,
    tool_calls: Option<Vec<SseToolCall>>,
}

#[derive(Debug, Deserialize)]
struct SseToolCall {
    id: Option<String>,
    function: Option<SseFunction>,
}

#[derive(Debug, Deserialize)]
struct SseFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}
