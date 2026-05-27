//! Loro CRDT projection layer for session state.
//!
//! IMPORTANT: Loro is a rebuildable projection. Trace is authoritative.
//! If Loro projection fails, trace remains the source of truth.

use crate::message::{Message, MessageContent, MessageRole};
use crate::tool::ToolResult;
use loro::{LoroDoc, LoroList, LoroMap};

/// Typed access to session state in a LoroDoc.
pub struct LoroSessionState {
    doc: LoroDoc,
}

impl LoroSessionState {
    pub fn new(doc: &LoroDoc) -> Self {
        let root = doc.get_map("session");
        // Ensure the messages list container exists
        root.insert_container("messages", LoroList::new()).ok();
        Self { doc: doc.clone() }
    }

    fn messages_list(&self) -> LoroList {
        let root = self.doc.get_map("session");
        // Try to get existing list, or create it
        match root.get("messages") {
            Some(loro::ValueOrContainer::Container(loro::Container::List(l))) => l,
            _ => {
                root.insert_container("messages", LoroList::new())
                    .expect("failed to create messages list")
            }
        }
    }

    /// Get all messages from the Loro document.
    pub fn messages(&self) -> Result<Vec<Message>, String> {
        let list = self.messages_list();
        let deep_value = list.get_deep_value();

        let loro_list = match deep_value {
            loro::LoroValue::List(l) => l,
            _ => return Ok(vec![]),
        };

        let mut result = Vec::new();
        for value in loro_list.iter() {
            if let Some(msg) = parse_loro_message(value) {
                result.push(msg);
            }
        }
        Ok(result)
    }

    /// Append a user message.
    pub fn append_user_message(
        &self,
        text: &str,
        trace_id: Option<impl AsRef<str>>,
    ) -> Result<(), String> {
        let list = self.messages_list();
        let map = list
            .push_container(LoroMap::new())
            .map_err(|e| e.to_string())?;
        map.insert("role", "user").map_err(|e| e.to_string())?;
        map.insert("text", text).map_err(|e| e.to_string())?;
        if let Some(tid) = trace_id {
            map.insert("trace_id", tid.as_ref())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Append an assistant message.
    pub fn append_assistant_message(
        &self,
        text: &str,
        trace_id: Option<impl AsRef<str>>,
    ) -> Result<(), String> {
        let list = self.messages_list();
        let map = list
            .push_container(LoroMap::new())
            .map_err(|e| e.to_string())?;
        map.insert("role", "assistant")
            .map_err(|e| e.to_string())?;
        map.insert("text", text).map_err(|e| e.to_string())?;
        if let Some(tid) = trace_id {
            map.insert("trace_id", tid.as_ref())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Append a tool result message.
    pub fn append_tool_result(
        &self,
        result: &ToolResult,
        trace_id: Option<impl AsRef<str>>,
    ) -> Result<(), String> {
        let list = self.messages_list();
        let map = list
            .push_container(LoroMap::new())
            .map_err(|e| e.to_string())?;
        map.insert("role", "tool").map_err(|e| e.to_string())?;
        map.insert("tool_call_id", result.tool_call_id.as_str())
            .map_err(|e| e.to_string())?;
        map.insert("tool_name", result.tool_name.as_str())
            .map_err(|e| e.to_string())?;
        map.insert("output", result.output.as_str())
            .map_err(|e| e.to_string())?;
        map.insert("is_error", result.is_error)
            .map_err(|e| e.to_string())?;
        if let Some(tid) = trace_id {
            map.insert("trace_id", tid.as_ref())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Mark the projection as stale (Loro projection failure).
    pub fn mark_projection_stale(
        &self,
        _trace_id: openwand_trace::TraceId,
        reason: String,
    ) -> Result<(), String> {
        let root = self.doc.get_map("session");
        root.insert("projection_stale", true)
            .map_err(|e| e.to_string())?;
        root.insert("projection_stale_reason", reason)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get the text of the last user message.
    /// Used for memory retrieval context.
    pub fn last_user_message_text(&self) -> Option<String> {
        let messages = self.messages().ok()?;
        messages
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::User)
            .and_then(|m| match &m.content {
                MessageContent::Text { text } => Some(text.clone()),
                _ => None,
            })
    }

    /// Check if projection is marked stale.
    pub fn projection_is_stale(&self) -> Result<bool, String> {
        let root = self.doc.get_map("session");
        let deep = root.get_deep_value();
        match deep.get_by_key("projection_stale") {
            Some(loro::LoroValue::Bool(b)) => Ok(*b),
            _ => Ok(false),
        }
    }
}

fn extract_str(value: &loro::LoroValue) -> Option<&str> {
    match value {
        loro::LoroValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn extract_bool(value: &loro::LoroValue) -> bool {
    match value {
        loro::LoroValue::Bool(b) => *b,
        _ => false,
    }
}

fn parse_loro_message(value: &loro::LoroValue) -> Option<Message> {
    let map = match value {
        loro::LoroValue::Map(m) => m,
        _ => return None,
    };

    let role_str = extract_str(map.get("role")?)?;
    let trace_id = map
        .get("trace_id")
        .and_then(extract_str)
        .map(|s| openwand_trace::TraceId(s.to_string()));

    match role_str {
        "user" => {
            let text = extract_str(map.get("text")?)?.to_string();
            Some(Message {
                role: MessageRole::User,
                content: MessageContent::Text { text },
                trace_id,
            })
        }
        "assistant" => {
            let text = extract_str(map.get("text")?)?.to_string();
            Some(Message {
                role: MessageRole::Assistant,
                content: MessageContent::Text { text },
                trace_id,
            })
        }
        "tool" => {
            let tool_call_id = extract_str(map.get("tool_call_id")?)?.to_string();
            let tool_name = extract_str(map.get("tool_name")?)?.to_string();
            let output = extract_str(map.get("output")?)?.to_string();
            let is_error = map.get("is_error").map(extract_bool).unwrap_or(false);
            Some(Message {
                role: MessageRole::Tool,
                content: MessageContent::ToolResult {
                    tool_call_id: openwand_core::ToolCallId(tool_call_id),
                    tool_name,
                    result: output,
                    is_error,
                },
                trace_id,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::ToolCallId;

    #[test]
    fn loro_state_append_and_read_messages() {
        let doc = LoroDoc::new();
        let state = LoroSessionState::new(&doc);

        state
            .append_user_message("hello", Some("trace_1"))
            .unwrap();
        state
            .append_assistant_message("world", Some("trace_2"))
            .unwrap();

        let messages = state.messages().unwrap();
        assert_eq!(2, messages.len());
        assert_eq!(MessageRole::User, messages[0].role);
        assert_eq!(MessageRole::Assistant, messages[1].role);
        assert!(messages[0].trace_id.is_some());
        assert!(messages[1].trace_id.is_some());
    }

    #[test]
    fn loro_state_tool_result() {
        let doc = LoroDoc::new();
        let state = LoroSessionState::new(&doc);

        let result = crate::tool::ToolResult {
            tool_call_id: ToolCallId("tc_1".into()),
            tool_name: "local__file_read".into(),
            output: "file contents".into(),
            is_error: false,
            duration_ms: 42,
        };
        state
            .append_tool_result(&result, Some("trace_3"))
            .unwrap();

        let messages = state.messages().unwrap();
        assert_eq!(1, messages.len());
        assert_eq!(MessageRole::Tool, messages[0].role);
    }

    #[test]
    fn loro_state_stale_marker() {
        let doc = LoroDoc::new();
        let state = LoroSessionState::new(&doc);

        assert!(!state.projection_is_stale().unwrap());

        state
            .mark_projection_stale(
                openwand_trace::TraceId("trace_stale".into()),
                "projection error".into(),
            )
            .unwrap();

        assert!(state.projection_is_stale().unwrap());
    }
}
