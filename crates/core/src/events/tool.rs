use serde::{Deserialize, Serialize};

use crate::ids::ToolCallId;
use crate::snapshots::ApprovalContextSnapshot;
use crate::tool_vocab::{ToolInvoker, ToolResultStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
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
        /// Full approval context for crash recovery.
        /// None for pre-03d events (backward compatible via #[serde(default)]).
        #[serde(default)]
        approval_context: Option<ApprovalContextSnapshot>,
    },
    Resumed {
        tool_call_id: ToolCallId,
        tool_name: String,
        resolution: String,
        /// Links to the approval request that authorized this execution.
        #[serde(default)]
        approval_request_id: Option<crate::ApprovalRequestId>,
    },
    Denied {
        tool_call_id: ToolCallId,
        tool_name: String,
        /// Links to the approval request that was rejected.
        #[serde(default)]
        approval_request_id: Option<crate::ApprovalRequestId>,
        /// Denial reason (e.g. "approval_context_too_large", "user_rejected").
        #[serde(default)]
        reason: Option<String>,
    },
    /// A tool call that was deferred because another tool in the same batch
    /// required approval and suspended the run. Terminal for this proposal;
    /// the model may re-propose in a later turn.
    Deferred {
        tool_call_id: ToolCallId,
        tool_name: String,
        reason: String,
        /// The tool call that caused the suspension blocking this one.
        #[serde(default)]
        blocked_by_tool_call_id: Option<ToolCallId>,
        /// The approval request that caused the suspension.
        #[serde(default)]
        blocked_by_approval_request_id: Option<crate::ApprovalRequestId>,
        /// Original position in the LLM's tool call batch (0-indexed).
        #[serde(default)]
        original_order_index: Option<u32>,
        /// Hash of arguments for audit without persisting full args.
        #[serde(default)]
        args_hash: Option<String>,
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
            Self::Deferred { .. } => "tool.deferred",
        }
    }
}
