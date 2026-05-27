use serde::{Deserialize, Serialize};

use crate::ids::ToolCallId;
use crate::tool_vocab::{ToolInvoker, ToolResultStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolEvent {
    Called {
        tool_call_id: ToolCallId,
        tool_name: String,
        args_hash: String,
        invoker: ToolInvoker,
    },
    Completed {
        tool_call_id: ToolCallId,
        tool_name: String,
        status: ToolResultStatus,
        result_summary: String,
        duration_ms: u64,
    },
    Failed {
        tool_call_id: ToolCallId,
        tool_name: String,
        error: String,
    },
    Suspended {
        tool_call_id: ToolCallId,
        tool_name: String,
        reason: String,
    },
    Resumed {
        tool_call_id: ToolCallId,
        tool_name: String,
        resolution: String,
    },
    Denied {
        tool_call_id: ToolCallId,
        tool_name: String,
    },
}

impl ToolEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "tool.called",
            Self::Completed { .. } => "tool.completed",
            Self::Failed { .. } => "tool.failed",
            Self::Suspended { .. } => "tool.suspended",
            Self::Resumed { .. } => "tool.resumed",
            Self::Denied { .. } => "tool.denied",
        }
    }
}
