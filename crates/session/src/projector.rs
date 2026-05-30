use crate::loro_state::LoroSessionState;
use crate::tool::ToolResult;
use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
use openwand_trace::TraceId;

/// Projector: applies trace events to Loro document.
pub struct LoroProjector {
    state: LoroSessionState,
}

impl LoroProjector {
    pub fn new(state: LoroSessionState) -> Self {
        Self { state }
    }

    /// Borrow the projected state.
    pub fn state(&self) -> &LoroSessionState {
        &self.state
    }

    /// Apply a trace event to the Loro projection.
    pub fn apply(
        &mut self,
        trace_id: TraceId,
        event: &OpenWandTraceEvent,
    ) -> Result<(), String> {
        let kind = event.event_kind();

        match kind {
            "session.user_message_injected" => {
                if let Some(text) = extract_text_from_event(event) {
                    self.state
                        .append_user_message(&text, Some(trace_id.0.as_str()))?;
                }
            }
            "inference.completed" => {
                if let Some(text) = extract_text_from_event(event) {
                    self.state
                        .append_assistant_message(&text, Some(trace_id.0.as_str()))?;
                }
            }
            "tool.completed" | "tool.failed" => {
                if let Some(result) = extract_tool_result_from_event(event) {
                    self.state
                        .append_tool_result(&result, Some(trace_id.0.as_str()))?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

fn extract_text_from_event(event: &OpenWandTraceEvent) -> Option<String> {
    let json = serde_json::to_value(event).ok()?;
    json.get("payload")
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

fn extract_tool_result_from_event(event: &OpenWandTraceEvent) -> Option<ToolResult> {
    match event {
        OpenWandTraceEvent::Tool(ToolEvent::Completed {
            tool_call_id,
            tool_name,
            result_summary,
            duration_ms,
            ..
        }) => Some(ToolResult {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            output: result_summary.clone(),
            is_error: false,
            duration_ms: *duration_ms,
        }),
        OpenWandTraceEvent::Tool(ToolEvent::Failed {
            tool_call_id,
            tool_name,
            error,
        }) => Some(ToolResult {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            output: error.clone(),
            is_error: true,
            duration_ms: 0,
        }),
        _ => None,
    }
}
