use openwand_core::SessionId;
use serde::{Deserialize, Serialize};

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
        is_error: bool,
    },
    RunCompleted {
        session_id: SessionId,
        stop_reason: String,
    },
}

// We need ToolCallId from core
use openwand_core::ToolCallId;
