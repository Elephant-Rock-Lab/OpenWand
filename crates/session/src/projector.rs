use crate::loro_state::LoroSessionState;
use crate::tool::ToolResult;
use openwand_core::events::OpenWandTraceEvent;
use openwand_trace::TraceId;

/// Projector: applies trace events to Loro document.
pub struct LoroProjector {
    state: LoroSessionState,
}

impl LoroProjector {
    pub fn new(state: LoroSessionState) -> Self {
        Self { state }
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
    let json = serde_json::to_value(event).ok()?;
    let payload = json.get("payload")?;

    Some(ToolResult {
        tool_call_id: openwand_core::ToolCallId(
            payload
                .get("tool_call_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
        ),
        tool_name: payload
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        output: payload
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        is_error: payload
            .get("is_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        duration_ms: payload
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}
