//! Tool-call buffer — accumulate partial deltas into complete, policy-evaluable calls.
//!
//! The LLM design is explicit: ToolGate cannot evaluate partial JSON arguments.
//! This buffer accumulates `ToolCallStart` + `ToolCallArgsDelta` fragments
//! and emits `ToolCallComplete` only when the adapter calls `complete()`.

use std::collections::HashMap;

use crate::error::LlmError;
use crate::response::LlmDelta;

#[derive(Debug, Clone)]
struct BufferedToolCall {
    name: Option<String>,
    args_chunks: Vec<String>,
}

/// Accumulates provider-style partial tool-call deltas into complete calls.
///
/// Usage in adapter code:
/// ```ignore
/// for delta in provider_stream {
///     match buffer.handle_delta(delta)? {
///         Some(LlmDelta::ToolCallComplete { .. }) => { /* emit to session */ }
///         Some(other) => { /* pass through */ }
///         None => { /* buffered, not ready yet */ }
///     }
/// }
/// ```
#[derive(Debug, Default)]
pub struct ToolCallBuffer {
    calls: HashMap<String, BufferedToolCall>,
}

impl ToolCallBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process any delta. Returns:
    /// - `Ok(Some(delta))` for pass-through deltas (Text, Reasoning, ToolCallComplete, Done)
    /// - `Ok(None)` for buffered deltas (ToolCallStart, ToolCallArgsDelta)
    /// - `Err(LlmError)` for malformed input
    pub fn handle_delta(&mut self, delta: LlmDelta) -> Result<Option<LlmDelta>, LlmError> {
        match delta {
            LlmDelta::ToolCallStart { id, name } => {
                self.handle_start(id, name)?;
                Ok(None)
            }
            LlmDelta::ToolCallArgsDelta { id, delta } => {
                self.handle_args_delta(id, delta)?;
                Ok(None)
            }
            // Pass through everything else
            other => Ok(Some(other)),
        }
    }

    /// Record a tool call start. Creates or updates a buffer entry.
    /// Returns Ok(()) — no ToolCallComplete emitted.
    pub fn handle_start(
        &mut self,
        id: String,
        name: Option<String>,
    ) -> Result<(), LlmError> {
        match self.calls.get_mut(&id) {
            Some(existing) => {
                // Update name only if currently None
                if existing.name.is_none() && name.is_some() {
                    existing.name = name;
                } else if existing.name.is_some() && name.is_some() && existing.name != name {
                    return Err(LlmError::Decode {
                        message: format!(
                            "Tool call {} name conflict: existing {:?}, new {:?}",
                            id, existing.name, name
                        ),
                    });
                }
            }
            None => {
                self.calls.insert(
                    id.clone(),
                    BufferedToolCall {
                        name,
                        args_chunks: Vec::new(),
                    },
                );
            }
        }
        Ok(())
    }

    /// Append argument chunk to a buffered tool call.
    /// Returns Ok(()) — no ToolCallComplete emitted.
    pub fn handle_args_delta(
        &mut self,
        id: String,
        delta: String,
    ) -> Result<(), LlmError> {
        match self.calls.get_mut(&id) {
            Some(buffered) => {
                buffered.args_chunks.push(delta);
                Ok(())
            }
            None => Err(LlmError::Decode {
                message: format!(
                    "ToolCallArgsDelta for unknown tool call id '{id}' — missing ToolCallStart"
                ),
            }),
        }
    }

    /// Finalize a buffered tool call. Parses accumulated JSON arguments.
    /// On success: removes buffer entry, returns ToolCallComplete.
    /// On malformed JSON: returns error, removes broken entry.
    pub fn complete(&mut self, id: &str) -> Result<LlmDelta, LlmError> {
        let buffered = self.calls.remove(id).ok_or_else(|| LlmError::Decode {
            message: format!("complete() called for unknown tool call id '{id}'"),
        })?;

        let name = buffered.name.ok_or_else(|| LlmError::Decode {
            message: format!("Tool call '{id}' has no name"),
        })?;

        let args_str: String = buffered.args_chunks.concat();
        let arguments: serde_json::Value = if args_str.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(&args_str).map_err(|e| LlmError::Decode {
                message: format!(
                    "Tool call '{}' has malformed JSON arguments: {}",
                    id, e
                ),
            })?
        };

        Ok(LlmDelta::ToolCallComplete {
            id: id.to_string(),
            name,
            arguments,
        })
    }

    /// Return all buffered call IDs and clear the map.
    /// Used to flush pending tool calls when finish_reason="tool_calls" arrives.
    pub fn drain_ids(&mut self) -> Vec<String> {
        self.calls.keys().cloned().collect()
    }

    /// Clear all buffered calls.
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::snapshots::TokenUsageSnapshot;

    #[test]
    fn tool_buffer_start_returns_none() {
        let mut buf = ToolCallBuffer::new();
        let result = buf
            .handle_delta(LlmDelta::ToolCallStart {
                id: "tc_1".into(),
                name: Some("read_file".into()),
            })
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn tool_buffer_args_delta_returns_none() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("read_file".into()))
            .unwrap();

        let result = buf
            .handle_delta(LlmDelta::ToolCallArgsDelta {
                id: "tc_1".into(),
                delta: "{\"path\":".into(),
            })
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn tool_buffer_complete_emits_tool_call_complete() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("read_file".into()))
            .unwrap();
        buf.handle_args_delta("tc_1".into(), "{\"path\":\"/tmp\"}".into())
            .unwrap();

        let result = buf.complete("tc_1").unwrap();
        match result {
            LlmDelta::ToolCallComplete {
                id,
                name,
                arguments,
            } => {
                assert_eq!("tc_1", id);
                assert_eq!("read_file", name);
                assert_eq!("/tmp", arguments["path"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }
    }

    #[test]
    fn tool_buffer_multiple_argument_chunks() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("bash".into()))
            .unwrap();
        buf.handle_args_delta("tc_1".into(), "{\"comma".into())
            .unwrap();
        buf.handle_args_delta("tc_1".into(), "nd\":\"ls".into())
            .unwrap();
        buf.handle_args_delta("tc_1".into(), " -la\"}".into())
            .unwrap();

        let result = buf.complete("tc_1").unwrap();
        match result {
            LlmDelta::ToolCallComplete { arguments, .. } => {
                assert_eq!("ls -la", arguments["command"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }
    }

    #[test]
    fn tool_buffer_missing_start_errors() {
        let mut buf = ToolCallBuffer::new();
        let result = buf.handle_args_delta("tc_99".into(), "{}".into());
        assert!(result.is_err());
        match result {
            Err(LlmError::Decode { message }) => {
                assert!(message.contains("tc_99"));
                assert!(message.contains("missing ToolCallStart"));
            }
            _ => panic!("expected Decode error"),
        }
    }

    #[test]
    fn tool_buffer_missing_name_errors() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), None).unwrap();
        buf.handle_args_delta("tc_1".into(), "{}".into()).unwrap();

        let result = buf.complete("tc_1");
        assert!(result.is_err());
        match result {
            Err(LlmError::Decode { message }) => {
                assert!(message.contains("no name"));
            }
            _ => panic!("expected Decode error"),
        }
    }

    #[test]
    fn tool_buffer_malformed_json_errors() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("bad".into()))
            .unwrap();
        buf.handle_args_delta("tc_1".into(), "not valid json{{{".into())
            .unwrap();

        let result = buf.complete("tc_1");
        assert!(result.is_err(), "Malformed JSON must not produce ToolCallComplete");
        match result {
            Err(LlmError::Decode { message }) => {
                assert!(message.contains("malformed JSON"));
            }
            _ => panic!("expected Decode error"),
        }
    }

    #[test]
    fn tool_buffer_malformed_json_does_not_emit_partial_call() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("bad".into()))
            .unwrap();
        buf.handle_args_delta("tc_1".into(), "broken".into()).unwrap();

        // complete() should fail and remove the entry
        let result = buf.complete("tc_1");
        assert!(result.is_err());

        // Second attempt should also fail (entry removed)
        let result2 = buf.complete("tc_1");
        assert!(result2.is_err());
        match result2 {
            Err(LlmError::Decode { message }) => {
                assert!(message.contains("unknown"));
            }
            _ => panic!("expected Decode error for removed entry"),
        }
    }

    #[test]
    fn tool_buffer_conflicting_name_errors() {
        let mut buf = ToolCallBuffer::new();
        buf.handle_start("tc_1".into(), Some("read_file".into()))
            .unwrap();

        let result = buf.handle_start("tc_1".into(), Some("write_file".into()));
        assert!(result.is_err());
        match result {
            Err(LlmError::Decode { message }) => {
                assert!(message.contains("conflict"));
            }
            _ => panic!("expected Decode error"),
        }
    }

    #[test]
    fn tool_buffer_passes_text_through() {
        let mut buf = ToolCallBuffer::new();
        let result = buf
            .handle_delta(LlmDelta::Text {
                delta: "hello".into(),
            })
            .unwrap();
        match result {
            Some(LlmDelta::Text { delta }) => assert_eq!("hello", delta),
            _ => panic!("expected Text to pass through"),
        }
    }

    #[test]
    fn tool_buffer_passes_done_through() {
        let mut buf = ToolCallBuffer::new();
        let result = buf
            .handle_delta(LlmDelta::Done {
                stop_reason: crate::response::LlmStopReason::Stop,
                usage: Some(TokenUsageSnapshot {
                    input: 10,
                    output: 5,
                    reasoning: None,
                    cache_read: None,
                    cache_write: None,
                }),
                provider_message_id: None,
            })
            .unwrap();
        match result {
            Some(LlmDelta::Done { .. }) => {}
            _ => panic!("expected Done to pass through"),
        }
    }

    #[test]
    fn tool_buffer_multiple_calls_independent() {
        let mut buf = ToolCallBuffer::new();

        // Buffer two separate tool calls
        buf.handle_start("tc_1".into(), Some("read".into()))
            .unwrap();
        buf.handle_start("tc_2".into(), Some("write".into()))
            .unwrap();

        buf.handle_args_delta("tc_1".into(), "{\"p\":\"a\"}".into())
            .unwrap();
        buf.handle_args_delta("tc_2".into(), "{\"p\":\"b\"}".into())
            .unwrap();

        // Complete tc_2 first
        let result2 = buf.complete("tc_2").unwrap();
        match result2 {
            LlmDelta::ToolCallComplete { id, name, arguments } => {
                assert_eq!("tc_2", id);
                assert_eq!("write", name);
                assert_eq!("b", arguments["p"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }

        // tc_1 is still buffered and completes fine
        let result1 = buf.complete("tc_1").unwrap();
        match result1 {
            LlmDelta::ToolCallComplete { id, name, arguments } => {
                assert_eq!("tc_1", id);
                assert_eq!("read", name);
                assert_eq!("a", arguments["p"]);
            }
            _ => panic!("expected ToolCallComplete"),
        }
    }
}
