//! Tool result and call context types.

use crate::normalize_output;
use openwand_core::{SessionId, ToolCallId};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub output: String,
    pub is_error: bool,
    pub duration_ms: u64,
    pub truncated: bool,
    pub original_size: Option<usize>,
}

impl ToolResult {
    pub fn success(
        tool_call_id: ToolCallId,
        tool_name: String,
        raw_output: String,
        duration_ms: u64,
    ) -> Self {
        let (output, truncated, original_size) = normalize_output(raw_output);
        Self {
            tool_call_id,
            tool_name,
            output,
            is_error: false,
            duration_ms,
            truncated,
            original_size,
        }
    }

    pub fn error(
        tool_call_id: ToolCallId,
        tool_name: String,
        error_message: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            output: error_message,
            is_error: true,
            duration_ms,
            truncated: false,
            original_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub working_directory: String,
    pub session_id: SessionId,
    pub cancellation: CancellationToken,
}
