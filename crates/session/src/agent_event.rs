use openwand_core::SessionId;
use serde::{Deserialize, Serialize};

use openwand_core::ToolCallId;

/// Transient events emitted during a run for UI consumption.
/// Not authoritative — not recorded in trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    RunStarted { session_id: SessionId },
    PhaseEntered { session_id: SessionId, phase: String, step: u64 },
    TextDelta { session_id: SessionId, delta: String },
    ToolCallStarted {
        session_id: SessionId,
        tool_name: String,
        tool_call_id: ToolCallId,
    },
    ToolCallCompleted {
        session_id: SessionId,
        tool_name: String,
        tool_call_id: ToolCallId,
        result_preview: String,
        is_error: bool,
    },
    /// Runner is suspended waiting for user approval of a tool call.
    ApprovalRequested {
        session_id: SessionId,
        tool_name: String,
        tool_call_id: ToolCallId,
        reason: String,
    },
    /// User's approval decision has been processed.
    ApprovalResolved {
        session_id: SessionId,
        tool_name: String,
        tool_call_id: ToolCallId,
        approved: bool,
    },
    RunCompleted {
        session_id: SessionId,
        stop_reason: String,
    },
}
