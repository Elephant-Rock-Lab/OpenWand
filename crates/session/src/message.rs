use openwand_core::ToolCallId;
use serde::{Deserialize, Serialize};

/// Role of a session message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

/// Content of a session message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text { text: String },
    ToolResult {
        tool_call_id: ToolCallId,
        tool_name: String,
        result: String,
        is_error: bool,
    },
}

/// A message in the session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    pub trace_id: Option<openwand_trace::TraceId>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text { text: text.into() },
            trace_id: None,
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text { text: text.into() },
            trace_id: None,
        }
    }

    pub fn tool_result(
        tool_call_id: ToolCallId,
        tool_name: impl Into<String>,
        result: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::ToolResult {
                tool_call_id,
                tool_name: tool_name.into(),
                result: result.into(),
                is_error,
            },
            trace_id: None,
        }
    }
}
