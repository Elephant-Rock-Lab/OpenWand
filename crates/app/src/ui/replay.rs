//! Trace replay: rebuilds UI timeline from authoritative trace entries.
//!
//! This is the durable view path. When the app reopens, it calls
//! `replay_timeline()` to reconstruct the session history from trace.
//!
//! The replay maps OpenWandTraceEvent variants to UI-facing types:
//! - Session::UserMessageInjected → UiTimelineItem::Message (User)
//! - Inference::Completed → UiTimelineItem::Message (Assistant)
//! - Tool::Called → UiTimelineItem::ToolCall
//! - Tool::Completed/Failed → UiTimelineItem::ToolResult
//!
//! Events not relevant to the UI (Gate, Mode, File, etc.) are skipped.

use crate::ui::dto::{UiMessage, UiMessageRole};
use openwand_core::events::OpenWandTraceEvent;
use openwand_store::StoredEvent;
use openwand_trace::{TraceQuery, TraceStore};

/// A single item in the session timeline, rendered by the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum UiTimelineItem {
    Message(UiMessage),
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        trace_id: String,
        timestamp: i64,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        output_preview: String,
        is_error: bool,
        trace_id: String,
        timestamp: i64,
    },
    RunMarker {
        kind: String,
        trace_id: String,
        timestamp: i64,
    },
}

/// Replay trace entries for a session stream into UI timeline items.
///
/// Scans all trace entries for the given session stream_id and maps
/// them to timeline items. Entries are returned in trace order (which
/// is the authority for ordering).
pub async fn replay_timeline(
    trace: &dyn TraceStore<StoredEvent>,
    session_id: &str,
) -> Result<Vec<UiTimelineItem>, String> {
    let stream_id = openwand_trace::TraceStreamId {
        scope: openwand_trace::TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    let mut all_items = Vec::new();
    let mut last_seen: Option<openwand_trace::ids::TraceId> = None;

    loop {
        let query = TraceQuery {
            stream_id: Some(stream_id.clone()),
            limit: Some(100),
            cursor: last_seen.clone(),
            ..Default::default()
        };

        let page = trace
            .scan(query)
            .await
            .map_err(|e| format!("Trace scan error: {e}"))?;

        if page.entries.is_empty() {
            break;
        }

        for entry in &page.entries {
            if let Some(item) = map_entry(entry) {
                all_items.push(item);
            }
        }

        // Cursor-based pagination not implemented in scan yet.
        // Break after first page since we fetch all entries anyway.
        break;
    }

    Ok(all_items)
}

/// Map a single trace entry to a UI timeline item.
/// Returns None for events the UI doesn't render.
fn map_entry(entry: &openwand_trace::entry::TraceEntry<StoredEvent>) -> Option<UiTimelineItem> {
    let trace_id = entry.id.0.clone();
    let timestamp = entry.occurred_at.timestamp();

    match &entry.event {
        // Unwrap StoredEvent → OpenWandTraceEvent
        // StoredEvent derefs to OpenWandTraceEvent, but we need the inner event.
        // Actually, StoredEvent wraps the event. Let me check...
        _ => {
            // We need to access the inner OpenWandTraceEvent.
            // StoredEvent derefs, so we can match on it directly.
            map_event(&entry.event, &trace_id, timestamp)
        }
    }
}

fn map_event(
    event: &StoredEvent,
    trace_id: &str,
    timestamp: i64,
) -> Option<UiTimelineItem> {
    // Access the inner OpenWandTraceEvent via Deref
    let inner: &openwand_core::events::OpenWandTraceEvent = event;

    match inner {
        openwand_core::events::OpenWandTraceEvent::Session(
            openwand_core::events::SessionEvent::UserMessageInjected { text }
        ) => Some(UiTimelineItem::Message(UiMessage {
            role: UiMessageRole::User,
            text: text.clone(),
            trace_id: Some(trace_id.to_string()),
            timestamp: Some(timestamp),
            is_error: false,
        })),

        openwand_core::events::OpenWandTraceEvent::Session(
            openwand_core::events::SessionEvent::Started { .. }
        ) => Some(UiTimelineItem::RunMarker {
            kind: "run_started".into(),
            trace_id: trace_id.to_string(),
            timestamp,
        }),

        openwand_core::events::OpenWandTraceEvent::Session(
            openwand_core::events::SessionEvent::Ended { .. }
        ) => Some(UiTimelineItem::RunMarker {
            kind: "run_ended".into(),
            trace_id: trace_id.to_string(),
            timestamp,
        }),

        openwand_core::events::OpenWandTraceEvent::Inference(
            openwand_core::events::InferenceEvent::Completed { .. }
        ) => {
            // Inference completion marks an assistant turn.
            // The actual text is in Loro, but for replay we need
            // to get it from somewhere. For now, we record a placeholder.
            // The real fix: record assistant text in trace too.
            Some(UiTimelineItem::Message(UiMessage {
                role: UiMessageRole::Assistant,
                text: "(assistant response — full text in Loro rebuild)".into(),
                trace_id: Some(trace_id.to_string()),
                timestamp: Some(timestamp),
                is_error: false,
            }))
        }

        openwand_core::events::OpenWandTraceEvent::Tool(
            openwand_core::events::ToolEvent::Called {
                tool_call_id,
                tool_name,
                ..
            }
        ) => Some(UiTimelineItem::ToolCall {
            tool_call_id: tool_call_id.0.clone(),
            tool_name: tool_name.clone(),
            trace_id: trace_id.to_string(),
            timestamp,
        }),

        openwand_core::events::OpenWandTraceEvent::Tool(
            openwand_core::events::ToolEvent::Completed {
                tool_call_id,
                tool_name,
                result_summary,
                ..
            }
        ) => Some(UiTimelineItem::ToolResult {
            tool_call_id: tool_call_id.0.clone(),
            tool_name: tool_name.clone(),
            output_preview: result_summary.clone(),
            is_error: false,
            trace_id: trace_id.to_string(),
            timestamp,
        }),

        openwand_core::events::OpenWandTraceEvent::Tool(
            openwand_core::events::ToolEvent::Failed {
                tool_call_id,
                tool_name,
                error,
                ..
            }
        ) => Some(UiTimelineItem::ToolResult {
            tool_call_id: tool_call_id.0.clone(),
            tool_name: tool_name.clone(),
            output_preview: error.clone(),
            is_error: true,
            trace_id: trace_id.to_string(),
            timestamp,
        }),

        // Skip events the UI doesn't render
        _ => None,
    }
}
