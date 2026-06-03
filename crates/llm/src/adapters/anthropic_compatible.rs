//! Anthropic-compatible adapter.
//!
//! Implements the Anthropic Messages API with SSE streaming.
//! Uses content blocks (text, tool_use, thinking) instead of OpenAI's
//! separate tool_calls array.
//!
//! Key differences from OpenAI:
//! - Endpoint: POST /v1/messages (not /v1/chat/completions)
//! - Auth: x-api-key header (not Bearer)
//! - SSE events: message_start/content_block_start/content_block_delta/message_delta/message_stop
//! - Tool calls: content_block_start type=tool_use + input_json_delta fragments
//! - Thinking: content_block_start type=thinking + thinking_delta fragments
//!
//! All output is normalized into the same LlmDelta variants used by OpenAI-compatible.

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

const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic-compatible provider client.
pub struct AnthropicCompatibleClient {
    http: Client,
    buffer: Arc<Mutex<ToolCallBuffer>>,
}

impl AnthropicCompatibleClient {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            buffer: Arc::new(Mutex::new(ToolCallBuffer::new())),
        }
    }

    /// Build the full URL for the messages endpoint.
    fn messages_url(target: &LlmTarget) -> String {
        let base = target
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");
        format!("{}/v1/messages", base.trim_end_matches('/'))
    }

    /// Convert an OpenWand LlmRequest into an Anthropic messages API JSON body.
    fn build_request_body(request: &LlmRequest) -> serde_json::Value {
        let mut messages = Vec::new();

        // Conversation history (no system — that's a separate field)
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
                    let mut blocks = Vec::new();
                    for c in content {
                        match c {
                            LlmContent::Text(t) => {
                                blocks.push(serde_json::json!({
                                    "type": "text",
                                    "text": t
                                }));
                            }
                            LlmContent::ToolCall { id, name, arguments } => {
                                blocks.push(serde_json::json!({
                                    "type": "tool_use",
                                    "id": id,
                                    "name": name,
                                    "input": arguments
                                }));
                            }
                            _ => {}
                        }
                    }
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": blocks
                    }));
                }
                LlmMessage::Tool {
                    tool_call_id,
                    content,
                    is_error,
                } => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": content,
                            "is_error": *is_error,
                        }]
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": request.target.model,
            "messages": messages,
            "stream": true,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        // System prompt (Anthropic: top-level field, not a message)
        if !request.system_prompt.is_empty() {
            body["system"] = serde_json::json!(request.system_prompt);
        }

        // Tools
        if !request.tools.is_empty() {
            let tools: Vec<_> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters_schema,
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tools);
        }

        // Tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                LlmToolChoice::Auto => serde_json::json!({"type": "auto"}),
                LlmToolChoice::None => serde_json::json!({"type": "none"}),
                LlmToolChoice::Required => serde_json::json!({"type": "any"}),
            };
        }

        // Temperature
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        // Thinking/reasoning budget
        if let Some(tb) = &request.thinking_budget {
            match tb {
                openwand_core::session_vocab::ThinkingBudgetSnapshot::Tokens(budget) => {
                    body["thinking"] = serde_json::json!({
                        "type": "enabled",
                        "budget_tokens": *budget as u64,
                    });
                }
                openwand_core::session_vocab::ThinkingBudgetSnapshot::Off => {
                    body["thinking"] = serde_json::json!({"type": "disabled"});
                }
                _ => {
                    // Low/Medium/High/Max — use reasonable defaults
                    let budget = match tb {
                        openwand_core::session_vocab::ThinkingBudgetSnapshot::Low => 4096u64,
                        openwand_core::session_vocab::ThinkingBudgetSnapshot::Medium => 10000u64,
                        openwand_core::session_vocab::ThinkingBudgetSnapshot::High => 32768u64,
                        openwand_core::session_vocab::ThinkingBudgetSnapshot::Max => 65536u64,
                        _ => 10000u64,
                    };
                    body["thinking"] = serde_json::json!({
                        "type": "enabled",
                        "budget_tokens": budget,
                    });
                }
            }
        }

        body
    }
}

#[async_trait]
impl LlmClient for AnthropicCompatibleClient {
    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        let url = Self::messages_url(&request.target);
        let body = Self::build_request_body(&request);

        let mut http_req = self.http.post(&url);
        http_req = http_req.json(&body);

        // Anthropic auth: x-api-key header
        if let Some(ref key) = request.target.api_key {
            http_req = http_req.header("x-api-key", key);
        }

        // Required headers
        http_req = http_req.header("anthropic-version", ANTHROPIC_API_VERSION);
        http_req = http_req.header("content-type", "application/json");

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
            // Error must not include Authorization header or API key (Patch 4)
            let safe_message = sanitize_error_message(&text);
            return Err(LlmError::Provider {
                provider: "anthropic".into(),
                message: format!("HTTP {status}: {safe_message}"),
                retryable: status.is_server_error() || status.as_u16() == 429,
            });
        }

        let buffer = self.buffer.clone();
        let stream = parse_anthropic_sse(resp, buffer);

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
        let mut provider_message_id = None;

        while let Some(delta) = stream.next().await {
            match delta? {
                LlmDelta::Text { delta } => {
                    if !delta.is_empty() {
                        content.push(LlmContent::Text(delta));
                    }
                }
                LlmDelta::Reasoning { delta, .. } => {
                    if !delta.is_empty() {
                        content.push(LlmContent::Reasoning(delta));
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
                    provider_message_id: mid,
                } => {
                    stop_reason = sr;
                    if let Some(u) = u {
                        usage = u;
                    }
                    if mid.is_some() {
                        provider_message_id = mid;
                    }
                }
                _ => {}
            }
        }

        Ok(LlmResponse {
            content,
            usage,
            stop_reason,
            provider_message_id,
        })
    }

    async fn health_check(&self, target: &LlmTarget) -> Result<(), LlmError> {
        // Anthropic doesn't have a /models endpoint like OpenAI.
        // Send a minimal request to verify connectivity.
        let base = target
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com");

        let url = format!("{}/v1/messages", base.trim_end_matches('/'));

        let mut req = self.http.post(&url);
        if let Some(ref key) = target.api_key {
            req = req.header("x-api-key", key);
        }
        req = req.header("anthropic-version", ANTHROPIC_API_VERSION);
        req = req.header("content-type", "application/json");

        let body = serde_json::json!({
            "model": target.model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                message: e.to_string(),
                retryable: true,
            })?;

        if resp.status().is_success() || resp.status().as_u16() == 400 {
            // 400 might mean "bad model name" but the server is reachable
            Ok(())
        } else {
            let status = resp.status();
            Err(LlmError::Provider {
                provider: "anthropic".into(),
                message: format!("health check failed: HTTP {status}"),
                retryable: false,
            })
        }
    }

    fn capabilities(&self, _target: &LlmTarget) -> LlmCapabilities {
        LlmCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: true,
            supports_vision: false,
            max_context_tokens: None,
            supported_features: vec![
                "streaming".into(),
                "tools".into(),
                "reasoning".into(),
            ],
        }
    }
}

/// Sanitize error messages to remove API keys and authorization headers (Patch 4).
fn sanitize_error_message(raw: &str) -> String {
    let mut cleaned = raw.to_string();

    // Remove x-api-key header values
    if let Some(pos) = cleaned.find("x-api-key") {
        // Truncate around the key area
        let end = std::cmp::min(pos + 30, cleaned.len());
        cleaned = format!("{}[redacted]{}", &cleaned[..pos], &cleaned[end..]);
    }

    // Remove any sk-ant- prefixed tokens
    let re = regex_or_manual_strip(&cleaned, "sk-ant-");
    if let Some(stripped) = re {
        cleaned = stripped;
    }

    cleaned
}

/// Manual strip of API key patterns (no regex dependency).
fn regex_or_manual_strip(text: &str, prefix: &str) -> Option<String> {
    if let Some(start) = text.find(prefix) {
        let end_pos = std::cmp::min(start + prefix.len() + 40, text.len());
        Some(format!("{}[redacted]{}", &text[..start], &text[end_pos..]))
    } else {
        None
    }
}

/// Parse Anthropic SSE stream into LlmDelta stream.
fn parse_anthropic_sse(
    resp: reqwest::Response,
    buffer: Arc<Mutex<ToolCallBuffer>>,
) -> impl Stream<Item = Result<LlmDelta, LlmError>> {
    let stream = resp.bytes_stream();
    let mut pending_tool_id: Option<String> = None;

    stream
        .scan(
            (String::new(), buffer, pending_tool_id),
            |(pending, buf, pending_tool_id), chunk| {
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

                // Process complete SSE blocks
                while let Some(pos) = pending.find("\n\n") {
                    let block = pending[..pos].to_string();
                    pending.drain(..pos + 2);

                    for line in block.lines() {
                        let line = line.trim();

                        // Parse event type
                        if let Some(event_type) = line.strip_prefix("event: ") {
                            // We'll use the data line instead; event type is informational
                            let _ = event_type;
                            continue;
                        }

                        let data = match line.strip_prefix("data: ") {
                            Some(d) => d,
                            None => continue,
                        };

                        // Parse the Anthropic SSE event
                        match serde_json::from_str::<serde_json::Value>(data) {
                            Ok(event) => {
                                let event_type = event["type"].as_str().unwrap_or("");

                                match event_type {
                                    "message_start" => {
                                        // Extract message ID
                                        // Usage scaffold is here but final usage comes in message_delta
                                    }

                                    "content_block_start" => {
                                        let block = &event["content_block"];
                                        let block_type = block["type"].as_str().unwrap_or("");

                                        match block_type {
                                            "tool_use" => {
                                                let id = block["id"].as_str().unwrap_or_default().to_string();
                                                let name = block["name"].as_str().unwrap_or_default().to_string();

                                                *pending_tool_id = Some(id.clone());

                                                if let Ok(mut locked_buf) = buf.lock() {
                                                    let _ = locked_buf.handle_start(
                                                        id.clone(),
                                                        Some(name),
                                                    );
                                                }
                                            }
                                            "thinking" => {
                                                // Thinking block started — we'll receive thinking_delta events
                                            }
                                            _ => {}
                                        }
                                    }

                                    "content_block_delta" => {
                                        let delta = &event["delta"];
                                        let delta_type = delta["type"].as_str().unwrap_or("");

                                        match delta_type {
                                            "text_delta" => {
                                                if let Some(text) = delta["text"].as_str() {
                                                    if !text.is_empty() {
                                                        deltas.push(Ok(LlmDelta::Text {
                                                            delta: text.to_string(),
                                                        }));
                                                    }
                                                }
                                            }
                                            "input_json_delta" => {
                                                // Tool call argument fragment
                                                if let Some(partial) =
                                                    delta["partial_json"].as_str()
                                                {
                                                    if !partial.is_empty() {
                                                        if let Some(ref id) = *pending_tool_id {
                                                            if let Ok(mut locked_buf) = buf.lock() {
                                                                let _ = locked_buf
                                                                    .handle_args_delta(
                                                                        id.clone(),
                                                                        partial.to_string(),
                                                                    );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            "thinking_delta" => {
                                                if let Some(thinking) =
                                                    delta["thinking"].as_str()
                                                {
                                                    if !thinking.is_empty() {
                                                        deltas.push(Ok(LlmDelta::Reasoning {
                                                            delta: thinking.to_string(),
                                                            redacted: false,
                                                        }));
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }

                                    "content_block_stop" => {
                                        // End of a content block — for tool_use, finalize the call
                                        let index = event["index"].as_u64();
                                        // If we had a pending tool call, flush it now
                                        if let Some(ref id) = *pending_tool_id {
                                            if let Ok(mut locked_buf) = buf.lock() {
                                                match locked_buf.complete(id) {
                                                    Ok(complete_delta) => {
                                                        deltas.push(Ok(complete_delta));
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!(
                                                            "Failed to complete Anthropic tool call {}: {e}",
                                                            id
                                                        );
                                                    }
                                                }
                                            }
                                            *pending_tool_id = None;
                                        }
                                    }

                                    "message_delta" => {
                                        let delta = &event["delta"];
                                        let usage = &event["usage"];

                                        // Stop reason
                                        let stop_reason =
                                            delta["stop_reason"].as_str().unwrap_or("end_turn");
                                        let llm_stop_reason = match stop_reason {
                                            "end_turn" | "stop" => LlmStopReason::Stop,
                                            "tool_use" => LlmStopReason::ToolCall,
                                            "max_tokens" => LlmStopReason::Length,
                                            _ => LlmStopReason::Stop,
                                        };

                                        // Usage (output tokens only here)
                                        let output_tokens =
                                            usage["output_tokens"].as_u64().unwrap_or(0);

                                        deltas.push(Ok(LlmDelta::Done {
                                            stop_reason: llm_stop_reason,
                                            usage: Some(TokenUsageSnapshot {
                                                input: 0, // Input tokens come in message_start
                                                output: output_tokens,
                                                reasoning: None,
                                                cache_read: None,
                                                cache_write: None,
                                            }),
                                            provider_message_id: None,
                                        }));
                                    }

                                    "message_stop" => {
                                        // End of message — nothing additional needed
                                    }

                                    "ping" => {
                                        // Keepalive, ignore
                                    }

                                    "error" => {
                                        let error = &event["error"];
                                        let message = error["message"]
                                            .as_str()
                                            .unwrap_or("unknown error");
                                        deltas.push(Err(LlmError::Provider {
                                            provider: "anthropic".into(),
                                            message: message.to_string(),
                                            retryable: false,
                                        }));
                                    }

                                    _ => {
                                        tracing::debug!(
                                            "Unknown Anthropic SSE event type: {event_type}"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Anthropic SSE parse error (non-fatal): {e}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{LlmProvider, LlmToolDef};

    #[test]
    fn anthropic_adapter_builds_messages_format_body() {
        let request = LlmRequest {
            target: LlmTarget {
                provider: LlmProvider::Anthropic,
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
                api_key: Some("sk-ant-test".into()),
            },
            system_prompt: "You are helpful.".into(),
            messages: vec![LlmMessage::User {
                content: vec![LlmContent::Text("Hello".into())],
            }],
            tools: vec![LlmToolDef {
                name: "read_file".into(),
                description: "Read a file".into(),
                parameters_schema: serde_json::json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }),
            }],
            thinking_budget: None,
            max_tokens: Some(1024),
            temperature: Some(0.7),
            tool_choice: Some(LlmToolChoice::Auto),
            provider_options: serde_json::json!({}),
        };

        let body = AnthropicCompatibleClient::build_request_body(&request);

        // System prompt is top-level, not in messages
        assert_eq!("You are helpful.", body["system"].as_str().unwrap());
        assert_eq!(1, body["messages"].as_array().unwrap().len());

        // Tools use input_schema, not parameters
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(1, tools.len());
        assert_eq!("read_file", tools[0]["name"].as_str().unwrap());
        assert!(tools[0]["input_schema"].is_object());

        // Tool choice uses Anthropic format
        assert_eq!("auto", body["tool_choice"]["type"].as_str().unwrap());

        // Max tokens is present
        assert_eq!(1024, body["max_tokens"].as_u64().unwrap());
    }

    #[test]
    fn anthropic_adapter_tool_result_in_user_message() {
        let request = LlmRequest {
            target: LlmTarget {
                provider: LlmProvider::Anthropic,
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
                api_key: None,
            },
            system_prompt: String::new(),
            messages: vec![LlmMessage::Tool {
                tool_call_id: "tu_123".into(),
                content: "file contents".into(),
                is_error: false,
            }],
            tools: vec![],
            thinking_budget: None,
            max_tokens: None,
            temperature: None,
            tool_choice: None,
            provider_options: serde_json::json!({}),
        };

        let body = AnthropicCompatibleClient::build_request_body(&request);

        // Tool result should be in a user message with tool_result content block
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(1, messages.len());
        assert_eq!("user", messages[0]["role"].as_str().unwrap());
        let content = messages[0]["content"].as_array().unwrap();
        assert_eq!(1, content.len());
        assert_eq!("tool_result", content[0]["type"].as_str().unwrap());
        assert_eq!("tu_123", content[0]["tool_use_id"].as_str().unwrap());
    }

    #[test]
    fn anthropic_adapter_thinking_budget_in_body() {
        let request = LlmRequest {
            target: LlmTarget {
                provider: LlmProvider::Anthropic,
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
                api_key: None,
            },
            system_prompt: String::new(),
            messages: vec![LlmMessage::User {
                content: vec![LlmContent::Text("Think about it".into())],
            }],
            tools: vec![],
            thinking_budget: Some(openwand_core::session_vocab::ThinkingBudgetSnapshot::Tokens(10000)),
            max_tokens: None,
            temperature: None,
            tool_choice: None,
            provider_options: serde_json::json!({}),
        };

        let body = AnthropicCompatibleClient::build_request_body(&request);
        assert_eq!("enabled", body["thinking"]["type"].as_str().unwrap());
        assert_eq!(10000, body["thinking"]["budget_tokens"].as_u64().unwrap());
    }

    #[test]
    fn sanitize_error_message_removes_api_key() {
        let raw = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key: sk-ant-api03-secret-key-here-1234567890"}}"#;
        let cleaned = sanitize_error_message(raw);
        assert!(!cleaned.contains("sk-ant-api03-secret-key"), "Must redact API key");
    }

    #[test]
    fn sanitize_error_message_preserves_safe_text() {
        let raw = r#"{"type":"error","error":{"type":"rate_limit_error","message":"rate limit exceeded"}}"#;
        let cleaned = sanitize_error_message(raw);
        assert!(cleaned.contains("rate limit exceeded"));
    }
}
