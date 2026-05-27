//! LLM response DTOs.
//!
//! Streaming deltas and non-streaming responses.
//! All errors go through Result Err, never through delta variants.

use openwand_core::snapshots::TokenUsageSnapshot;
use serde::{Deserialize, Serialize};

use crate::request::LlmContent;

/// Streaming delta from the LLM.
///
/// Important: `ToolCallStart` and `ToolCallArgsDelta` are stream-internal.
/// Only `ToolCallComplete` should be converted into a pending ToolCall
/// and routed through policy's ToolGate. ToolGate cannot evaluate partial JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmDelta {
    /// Streaming text content delta.
    Text { delta: String },

    /// Streaming reasoning/thinking delta.
    Reasoning {
        delta: String,
        /// True if the reasoning content was redacted by the provider.
        redacted: bool,
    },

    /// A tool call has started. Name may arrive late.
    /// Buffer this — do not evaluate through policy yet.
    ToolCallStart {
        id: String,
        name: Option<String>,
    },

    /// Partial JSON argument data for a tool call.
    /// Buffer these — ToolGate cannot evaluate partial JSON.
    ToolCallArgsDelta {
        id: String,
        delta: String,
    },

    /// A complete tool call with full arguments.
    /// This is what openwand-session converts into a pending ToolCall
    /// and routes through openwand-policy.
    ToolCallComplete {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },

    /// Stream completed.
    Done {
        stop_reason: LlmStopReason,
        usage: Option<TokenUsageSnapshot>,
        /// Provider-assigned message ID.
        provider_message_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmStopReason {
    Stop,
    ToolCall,
    Length,
    ContentFilter,
}

/// Non-streaming completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: Vec<LlmContent>,
    pub usage: TokenUsageSnapshot,
    pub stop_reason: LlmStopReason,
    pub provider_message_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_delta_text_roundtrip() {
        let delta = LlmDelta::Text {
            delta: "Hello, ".into(),
        };
        let json = serde_json::to_string(&delta).unwrap();
        let restored: LlmDelta = serde_json::from_str(&json).unwrap();
        match restored {
            LlmDelta::Text { delta } => assert_eq!("Hello, ", delta),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn llm_delta_tool_call_complete_roundtrip() {
        let delta = LlmDelta::ToolCallComplete {
            id: "tc_123".into(),
            name: "read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/test.rs"}),
        };
        let json = serde_json::to_string(&delta).unwrap();
        let restored: LlmDelta = serde_json::from_str(&json).unwrap();
        match restored {
            LlmDelta::ToolCallComplete { id, name, arguments } => {
                assert_eq!("tc_123", id);
                assert_eq!("read_file", name);
                assert_eq!("/tmp/test.rs", arguments["path"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }
    }

    #[test]
    fn llm_delta_done_roundtrip() {
        let delta = LlmDelta::Done {
            stop_reason: LlmStopReason::ToolCall,
            usage: Some(TokenUsageSnapshot {
                input: 1000,
                output: 500,
                reasoning: Some(200),
                cache_read: None,
                cache_write: None,
            }),
            provider_message_id: Some("msg_abc".into()),
        };
        let json = serde_json::to_string(&delta).unwrap();
        let restored: LlmDelta = serde_json::from_str(&json).unwrap();
        match restored {
            LlmDelta::Done {
                stop_reason,
                usage,
                provider_message_id,
            } => {
                assert!(matches!(stop_reason, LlmStopReason::ToolCall));
                assert!(usage.is_some());
                assert_eq!(Some("msg_abc"), provider_message_id.as_deref());
            }
            _ => panic!("expected Done"),
        }
    }

    #[test]
    fn llm_response_roundtrip() {
        let resp = LlmResponse {
            content: vec![LlmContent::Text("Done.".into())],
            usage: TokenUsageSnapshot {
                input: 500,
                output: 10,
                reasoning: None,
                cache_read: None,
                cache_write: None,
            },
            stop_reason: LlmStopReason::Stop,
            provider_message_id: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: LlmResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(1, restored.content.len());
        assert!(matches!(restored.stop_reason, LlmStopReason::Stop));
    }
}
