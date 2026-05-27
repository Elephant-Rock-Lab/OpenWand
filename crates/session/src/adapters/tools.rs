use crate::tool::ToolCall;
use openwand_core::SessionId;
use openwand_tools::result::ToolCallContext;
use tokio_util::sync::CancellationToken;

/// Convert session ToolCall to tools-layer ToolCall.
impl From<&ToolCall> for openwand_tools::executor::ToolCall {
    fn from(call: &ToolCall) -> Self {
        Self {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        }
    }
}

/// Convert tools-layer ToolResult to session ToolResult.
impl From<openwand_tools::result::ToolResult> for crate::tool::ToolResult {
    fn from(result: openwand_tools::result::ToolResult) -> Self {
        Self {
            tool_call_id: result.tool_call_id,
            tool_name: result.tool_name,
            output: result.output,
            is_error: result.is_error,
            duration_ms: result.duration_ms,
        }
    }
}

/// Build a tools-layer ToolCallContext from session parameters.
pub fn build_tool_context(
    session_id: SessionId,
    working_directory: String,
    cancellation: CancellationToken,
) -> ToolCallContext {
    ToolCallContext {
        working_directory,
        session_id,
        cancellation,
    }
}
