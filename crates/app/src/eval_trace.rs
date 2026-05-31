//! Trace evidence reader for evaluation.
//!
//! Scans a session's trace stream and groups events by family into
//! typed structs that collectors consume. Never mutates the trace.
//! Each entry preserves trace_id for provenance and carries the raw
//! payload as serde_json::Value so collectors can extract typed fields.

use chrono::{DateTime, Utc};
use openwand_core::events::OpenWandTraceEvent;
use openwand_store::StoredEvent;
use openwand_trace::{TraceQuery, TraceStore, TraceStreamId, TraceStreamScope};
use serde::{Deserialize, Serialize};

/// A single trace event reference with full provenance and typed payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvidenceEntry {
    /// Stable trace ID for provenance linkage.
    pub trace_id: String,
    /// Dotted event kind, e.g. "tool.called", "gate.evaluated".
    pub event_kind: String,
    /// When this event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Human-readable summary for reports.
    pub summary: String,
    /// Raw event payload for typed extraction by collectors.
    pub payload: serde_json::Value,
}

/// Grouped trace evidence for a session, organized by event family.
#[derive(Debug, Clone, Default)]
pub struct EvalTraceEvidence {
    pub session_id: String,
    pub inference_events: Vec<TraceEvidenceEntry>,
    pub gate_events: Vec<TraceEvidenceEntry>,
    pub tool_events: Vec<TraceEvidenceEntry>,
    pub file_events: Vec<TraceEvidenceEntry>,
    pub memory_events: Vec<TraceEvidenceEntry>,
    pub session_events: Vec<TraceEvidenceEntry>,
    pub mode_events: Vec<TraceEvidenceEntry>,
    pub workflow_events: Vec<TraceEvidenceEntry>,
    pub artifact_events: Vec<TraceEvidenceEntry>,
}

impl EvalTraceEvidence {
    /// Total count of evidence entries across all families.
    pub fn total_events(&self) -> usize {
        self.inference_events.len()
            + self.gate_events.len()
            + self.tool_events.len()
            + self.file_events.len()
            + self.memory_events.len()
            + self.session_events.len()
            + self.mode_events.len()
            + self.workflow_events.len()
            + self.artifact_events.len()
    }

    /// Whether any tool events exist (used by anti-vacuous-pass).
    pub fn has_tool_events(&self) -> bool {
        !self.tool_events.is_empty()
    }

    /// Whether any inference events exist (used by anti-vacuous-pass).
    pub fn has_inference_events(&self) -> bool {
        !self.inference_events.is_empty()
    }

    /// Whether any gate events exist.
    pub fn has_gate_events(&self) -> bool {
        !self.gate_events.is_empty()
    }

    /// Whether any file events exist.
    pub fn has_file_events(&self) -> bool {
        !self.file_events.is_empty()
    }

    /// Get tool events of a specific kind (e.g., "tool.called").
    pub fn tool_events_by_kind(&self, kind: &str) -> Vec<&TraceEvidenceEntry> {
        self.tool_events.iter().filter(|e| e.event_kind == kind).collect()
    }

    /// Get gate events of a specific kind.
    pub fn gate_events_by_kind(&self, kind: &str) -> Vec<&TraceEvidenceEntry> {
        self.gate_events.iter().filter(|e| e.event_kind == kind).collect()
    }

    /// Get inference events of a specific kind.
    pub fn inference_events_by_kind(&self, kind: &str) -> Vec<&TraceEvidenceEntry> {
        self.inference_events.iter().filter(|e| e.event_kind == kind).collect()
    }

    /// Get file events of a specific kind.
    pub fn file_events_by_kind(&self, kind: &str) -> Vec<&TraceEvidenceEntry> {
        self.file_events.iter().filter(|e| e.event_kind == kind).collect()
    }
}

/// Scan a session's trace and group events by family.
/// Returns empty evidence (not error) if the session has no events.
pub async fn scan_trace_evidence(
    store: &dyn TraceStore<StoredEvent>,
    session_id: &str,
) -> EvalTraceEvidence {
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    let mut evidence = EvalTraceEvidence {
        session_id: session_id.to_string(),
        ..Default::default()
    };

    // Paginated scan — collect all entries
    let mut cursor: Option<openwand_trace::TraceId> = None;
    loop {
        let mut query = TraceQuery {
            stream_id: Some(stream_id.clone()),
            limit: Some(500),
            ..Default::default()
        };
        query.cursor = cursor;

        let page = match store.scan(query).await {
            Ok(p) => p,
            Err(_) => return evidence, // Return what we have on error
        };

        for entry in &page.entries {
            let trace_id = entry.id.to_string();
            let event_kind = entry.event.0.event_kind().to_string();
            let occurred_at = entry.occurred_at;
            let family = entry.event.0.event_family();
            let payload = serde_json::to_value(&entry.event.0).unwrap_or(serde_json::Value::Null);

            let summary = summarize_event(family, &event_kind, &payload);

            let evidence_entry = TraceEvidenceEntry {
                trace_id,
                event_kind,
                occurred_at,
                summary,
                payload,
            };

            match family {
                "inference" => evidence.inference_events.push(evidence_entry),
                "gate" => evidence.gate_events.push(evidence_entry),
                "tool" => evidence.tool_events.push(evidence_entry),
                "file" => evidence.file_events.push(evidence_entry),
                "memory" => evidence.memory_events.push(evidence_entry),
                "session" => evidence.session_events.push(evidence_entry),
                "mode" => evidence.mode_events.push(evidence_entry),
                "workflow" => evidence.workflow_events.push(evidence_entry),
                "artifact" => evidence.artifact_events.push(evidence_entry),
                _ => {} // Unknown families ignored
            }
        }

        if page.next_cursor.is_none() || page.entries.is_empty() {
            break;
        }
        cursor = page.next_cursor;
    }

    evidence
}

/// Produce a human-readable summary for a trace event.
fn summarize_event(family: &str, kind: &str, payload: &serde_json::Value) -> String {
    match family {
        "tool" => {
            let tool_name = payload.get("payload")
                .and_then(|p| {
                    // Try each variant
                    for variant in ["Called", "Completed", "Failed", "Suspended", "Resumed", "Denied"] {
                        if let Some(v) = p.get(variant) {
                            return v.get("tool_name").and_then(|t| t.as_str()).map(String::from);
                        }
                    }
                    None
                })
                .unwrap_or_else(|| "unknown_tool".to_string());
            format!("{} {}", kind, tool_name)
        }
        "gate" => {
            let passed = payload.get("payload")
                .and_then(|p| {
                    if let Some(e) = p.get("Evaluated") {
                        e.get("passed").and_then(|v| v.as_bool())
                    } else {
                        None
                    }
                })
                .map(|b| if b { "passed" } else { "blocked" })
                .unwrap_or("evaluated");
            format!("{} ({})", kind, passed)
        }
        "inference" => {
            let model = payload.get("payload")
                .and_then(|p| {
                    if let Some(c) = p.get("Called") {
                        c.get("model").and_then(|v| v.as_str()).map(String::from)
                    } else if let Some(c) = p.get("Completed") {
                        c.get("model").and_then(|v| v.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "unknown".to_string());
            format!("{} ({})", kind, model)
        }
        "file" => {
            let path = payload.get("payload")
                .and_then(|p| {
                    for variant in ["Read", "Written", "Patched"] {
                        if let Some(v) = p.get(variant) {
                            return v.get("path").and_then(|t| t.as_str()).map(String::from);
                        }
                    }
                    None
                })
                .unwrap_or_else(|| "unknown_path".to_string());
            format!("{} {}", kind, path)
        }
        _ => kind.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_trace_evidence_groups_by_family() {
        let mut evidence = EvalTraceEvidence::default();
        evidence.session_id = "test".to_string();
        evidence.tool_events.push(TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: Utc::now(),
            summary: "tool.called file_read".to_string(),
            payload: serde_json::Value::Null,
        });
        evidence.gate_events.push(TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "gate.evaluated".to_string(),
            occurred_at: Utc::now(),
            summary: "gate.evaluated (passed)".to_string(),
            payload: serde_json::Value::Null,
        });

        assert_eq!(1, evidence.tool_events.len());
        assert_eq!(1, evidence.gate_events.len());
        assert_eq!(0, evidence.inference_events.len());
        assert_eq!(2, evidence.total_events());
    }

    #[test]
    fn eval_trace_evidence_filters_by_kind() {
        let mut evidence = EvalTraceEvidence::default();
        evidence.tool_events.push(TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: Utc::now(),
            summary: "tool.called file_read".to_string(),
            payload: serde_json::Value::Null,
        });
        evidence.tool_events.push(TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "tool.completed".to_string(),
            occurred_at: Utc::now(),
            summary: "tool.completed file_read".to_string(),
            payload: serde_json::Value::Null,
        });

        assert_eq!(1, evidence.tool_events_by_kind("tool.called").len());
        assert_eq!(1, evidence.tool_events_by_kind("tool.completed").len());
        assert_eq!(0, evidence.tool_events_by_kind("tool.failed").len());
    }

    #[test]
    fn eval_trace_evidence_preserves_trace_ids() {
        let mut evidence = EvalTraceEvidence::default();
        evidence.gate_events.push(TraceEvidenceEntry {
            trace_id: "gate_001".to_string(),
            event_kind: "gate.evaluated".to_string(),
            occurred_at: Utc::now(),
            summary: "test".to_string(),
            payload: serde_json::json!({"test": true}),
        });

        assert_eq!("gate_001", evidence.gate_events[0].trace_id);
        assert!(evidence.gate_events[0].payload.is_object());
    }

    #[test]
    fn eval_trace_evidence_returns_empty_for_missing_session() {
        // Without a real trace store, we just test the default struct
        let evidence = EvalTraceEvidence::default();
        assert_eq!(0, evidence.total_events());
        assert!(!evidence.has_tool_events());
        assert!(!evidence.has_inference_events());
    }

    #[test]
    fn eval_trace_evidence_never_requires_provider() {
        // This test documents that EvalTraceEvidence is pure data —
        // no network, no API keys, no provider needed.
        let evidence = EvalTraceEvidence {
            session_id: "offline".to_string(),
            ..Default::default()
        };
        assert_eq!(0, evidence.total_events());
    }

    #[test]
    fn eval_trace_evidence_has_helpers() {
        let mut evidence = EvalTraceEvidence::default();
        assert!(!evidence.has_tool_events());
        assert!(!evidence.has_inference_events());
        assert!(!evidence.has_gate_events());
        assert!(!evidence.has_file_events());

        evidence.inference_events.push(TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "inference.called".to_string(),
            occurred_at: Utc::now(),
            summary: "test".to_string(),
            payload: serde_json::Value::Null,
        });
        assert!(evidence.has_inference_events());
    }

    #[test]
    fn eval_trace_summarize_event_produces_readable_output() {
        let tool_payload = serde_json::json!({
            "family": "tool",
            "payload": {
                "Called": {
                    "tool_call_id": "tc1",
                    "tool_name": "file_read",
                    "args_hash": "h1",
                    "invoker": "Llm"
                }
            }
        });
        let summary = summarize_event("tool", "tool.called", &tool_payload);
        assert!(summary.contains("file_read"));

        let gate_payload = serde_json::json!({
            "family": "gate",
            "payload": {
                "Evaluated": {
                    "gate_id": "g1",
                    "gate_kind": "risk",
                    "passed": true,
                    "risk_level": "Low",
                    "summary": "ok"
                }
            }
        });
        let summary = summarize_event("gate", "gate.evaluated", &gate_payload);
        assert!(summary.contains("passed"));
    }
}
