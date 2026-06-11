//! Live trace and memory inspector — read-only projection over trace, memory, and session state.
//!
//! The inspector observes and explains. It never mutates.
//!
//! Architecture:
//!   TraceStore → load_inspector_from_trace() → LiveInspectorState (all 9 families)
//!   TraceStore::scan_relations → event detail drawer
//!   UiFilteredMemoryPanel → load_memory_inspector() → MemoryInspectorContext + evidence
//!
//! Patch 1: All known OpenWandTraceEvent families are recognized and safely summarized.
//!          Families with minimal current payloads render generic summaries.
//!          Unsupported future families produce warnings, not panics.
//! Patch 2: Event detail loads both outgoing and incoming relations.
//! Patch 3: apply_trace_entry_to_inspector() works for both batch and live paths.
//! Patch 4: Inspector never appends trace, trace relations, or mutates projections.

use chrono::{DateTime, Utc};
use openwand_core::events::{
    FileEvent, GateEvent, InferenceEvent, MemoryEvent, ModeEvent, OpenWandTraceEvent,
    SessionEvent, ToolEvent, WorkflowEvent, ArtifactEvent,
};
use openwand_store::StoredEvent;
use openwand_trace::entry::TraceEntry;
use openwand_trace::relation::{TraceRelation, TraceRelationKind};
use openwand_trace::store::TraceStore;
use openwand_trace::query::{RelationQuery, TraceQuery};
use openwand_trace::ids::TraceId;
use serde::{Deserialize, Serialize};

use crate::ui::memory_dto::UiFilteredMemoryPanel;

// ── Trace Timeline ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceTimelineItem {
    pub trace_id: String,
    pub event_family: String,
    pub event_kind: String,
    pub actor: String,
    pub timestamp: String,
    pub summary: String,
    pub stream_sequence: u64,
}

// ── Gate / Tool Timeline ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateToolTimelineItem {
    pub trace_id: String,
    pub kind: GateToolKind,
    pub status: String,
    pub risk_level: Option<String>,
    pub tool_name: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GateToolKind {
    GateEvaluated,
    GateBatchCompleted,
    GateOutputScreened,
    ToolCalled,
    ToolSuspended,
    ToolResumed,
    ToolDenied,
    ToolCompleted,
    ToolFailed,
    ToolDeferred,
}

impl GateToolKind {
    pub fn display(&self) -> &'static str {
        match self {
            Self::GateEvaluated => "Gate Evaluated",
            Self::GateBatchCompleted => "Gate Batch",
            Self::GateOutputScreened => "Output Screened",
            Self::ToolCalled => "Tool Called",
            Self::ToolSuspended => "Tool Suspended",
            Self::ToolResumed => "Tool Resumed",
            Self::ToolDenied => "Tool Denied",
            Self::ToolCompleted => "Tool Completed",
            Self::ToolFailed => "Tool Failed",
            Self::ToolDeferred => "Tool Deferred",
        }
    }
}

// ── Memory Inspector ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryInspectorContext {
    pub retrieved_count: usize,
    pub included_count: usize,
    pub excluded_count: usize,
    pub stale_count: usize,
    pub superseded_count: usize,
    pub unverifiable_count: usize,
    pub conflicts_count: usize,
    pub prompt_context_available: bool,
    pub consistency_report_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEvidenceItem {
    pub claim_summary: String,
    pub status: MemoryEvidenceStatus,
    pub reason: String,
    pub source_trace_ids: Vec<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryEvidenceStatus {
    Included,
    ExcludedStale,
    ExcludedSuperseded,
    ExcludedUnverifiable,
    ExcludedConflict,
    Missing,
}

impl MemoryEvidenceStatus {
    pub fn display(&self) -> &'static str {
        match self {
            Self::Included => "Included",
            Self::ExcludedStale => "Excluded (Stale)",
            Self::ExcludedSuperseded => "Excluded (Superseded)",
            Self::ExcludedUnverifiable => "Excluded (Unverifiable)",
            Self::ExcludedConflict => "Excluded (Conflict)",
            Self::Missing => "Missing in Memory",
        }
    }
}

// ── Trace Relations ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceRelationSummary {
    pub from_trace_id: String,
    pub to_trace_id: String,
    pub relation_kind: String,
    pub direction: RelationDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationDirection {
    Outgoing,
    Incoming,
}

// ── Event Detail ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceEventDetail {
    pub trace_id: String,
    pub event_kind: String,
    pub actor: String,
    pub timestamp: String,
    pub payload_summary: Vec<(String, String)>,
    pub relations: Vec<TraceRelationSummary>,
}

// ── Root State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveInspectorState {
    pub session_id: Option<String>,
    pub trace_timeline: Vec<TraceTimelineItem>,
    pub gate_tool_events: Vec<GateToolTimelineItem>,
    pub memory_context: Option<MemoryInspectorContext>,
    pub memory_evidence: Vec<MemoryEvidenceItem>,
    pub selected_event: Option<TraceEventDetail>,
    pub warnings: Vec<String>,
}

impl Default for LiveInspectorState {
    fn default() -> Self {
        Self {
            session_id: None,
            trace_timeline: Vec::new(),
            gate_tool_events: Vec::new(),
            memory_context: None,
            memory_evidence: Vec::new(),
            selected_event: None,
            warnings: Vec::new(),
        }
    }
}

impl LiveInspectorState {
    pub fn is_empty(&self) -> bool {
        self.trace_timeline.is_empty()
            && self.gate_tool_events.is_empty()
            && self.memory_context.is_none()
            && self.memory_evidence.is_empty()
            && self.selected_event.is_none()
            && self.warnings.is_empty()
    }
}

// ── Batch Loader ────────────────────────────────────────────────────────────

/// Load inspector state from persisted trace entries.
/// Reads ALL event families and produces timeline + gate/tool items.
pub async fn load_inspector_from_trace(
    trace: &dyn TraceStore<StoredEvent>,
    session_id: &str,
) -> Result<LiveInspectorState, String> {
    let stream_id = openwand_trace::stream::TraceStreamId {
        scope: openwand_trace::stream::TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    let mut state = LiveInspectorState {
        session_id: Some(session_id.to_string()),
        ..Default::default()
    };

    let mut cursor = None;
    loop {
        let query = TraceQuery {
            stream_id: Some(stream_id.clone()),
            limit: Some(100),
            cursor,
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
            apply_trace_entry_to_inspector(&mut state, entry);
        }

        if page.entries.len() < 100 {
            break;
        }
        cursor = page.next_cursor;
    }

    Ok(state)
}

/// Load event detail with both outgoing and incoming trace relations.
/// Patch 2: loads both directions.
pub async fn load_event_detail(
    trace: &dyn TraceStore<StoredEvent>,
    trace_id: &str,
) -> Result<TraceEventDetail, String> {
    let tid = TraceId(trace_id.to_string());

    // Load the entry itself
    let entry = trace
        .get_with_relations(tid.clone())
        .await
        .map_err(|e| format!("{e}"))?
        .ok_or_else(|| format!("Trace entry not found: {}", trace_id))?;

    let payload = summarize_trace_event(&entry.entry.event);

    // Load outgoing relations (from this entry)
    let outgoing = trace
        .scan_relations(RelationQuery {
            from: Some(tid.clone()),
            to: None,
            kind: None,
            depth: None,
            limit: Some(50),
        })
        .await
        .map_err(|e| format!("{e}"))?;

    // Load incoming relations (to this entry)
    let incoming = trace
        .scan_relations(RelationQuery {
            from: None,
            to: Some(tid.clone()),
            kind: None,
            depth: None,
            limit: Some(50),
        })
        .await
        .map_err(|e| format!("{e}"))?;

    let mut relations = Vec::new();
    for r in &outgoing {
        relations.push(TraceRelationSummary {
            from_trace_id: r.from.0.clone(),
            to_trace_id: r.to.0.clone(),
            relation_kind: format!("{:?}", r.kind),
            direction: RelationDirection::Outgoing,
        });
    }
    for r in &incoming {
        relations.push(TraceRelationSummary {
            from_trace_id: r.from.0.clone(),
            to_trace_id: r.to.0.clone(),
            relation_kind: format!("{:?}", r.kind),
            direction: RelationDirection::Incoming,
        });
    }

    Ok(TraceEventDetail {
        trace_id: trace_id.to_string(),
        event_kind: entry.entry.event_kind.clone(),
        actor: format!("{:?}", entry.entry.actor),
        timestamp: entry.entry.occurred_at.to_rfc3339(),
        payload_summary: payload,
        relations,
    })
}

// ── Live Bridge (Patch 3) ───────────────────────────────────────────────────

/// Apply a single trace entry to inspector state.
/// Used by both batch loader and live event stream.
pub fn apply_trace_entry_to_inspector(
    state: &mut LiveInspectorState,
    entry: &TraceEntry<StoredEvent>,
) {
    let trace_id = entry.id.0.clone();
    let event_kind = entry.event_kind.clone();
    let actor = format!("{:?}", entry.actor);
    let timestamp = entry.occurred_at.to_rfc3339();
    let seq = entry.stream_sequence;
    let family = entry.event.event_family().to_string();

    // Timeline item — all families produce one
    let summary = summarize_event_kind(&entry.event);
    state.trace_timeline.push(TraceTimelineItem {
        trace_id,
        event_family: family,
        event_kind: event_kind.clone(),
        actor: actor.clone(),
        timestamp: timestamp.clone(),
        summary,
        stream_sequence: seq,
    });

    // Gate/tool-specific detail
    match &*entry.event {
        OpenWandTraceEvent::Gate(gate) => apply_gate_event(state, gate, &entry.id.0),
        OpenWandTraceEvent::Tool(tool) => apply_tool_event(state, tool, &entry.id.0),
        _ => {}
    }
}

fn apply_gate_event(state: &mut LiveInspectorState, gate: &GateEvent, trace_id: &str) {
    match gate {
        GateEvent::Evaluated { passed, risk_level, summary, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::GateEvaluated,
                status: if *passed { "passed".into() } else { "failed".into() },
                risk_level: risk_level.as_ref().map(|r| format!("{:?}", r)),
                tool_name: None,
                reason: Some(summary.clone()),
            });
        }
        GateEvent::BatchCompleted { total, passed, failed, overall_risk } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::GateBatchCompleted,
                status: format!("{}/{} passed", passed, total),
                risk_level: Some(format!("{:?}", overall_risk)),
                tool_name: None,
                reason: Some(format!("{} failed", failed)),
            });
        }
        GateEvent::OutputScreened { passed, forbidden_hits, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::GateOutputScreened,
                status: if *passed { "passed".into() } else { "blocked".into() },
                risk_level: None,
                tool_name: None,
                reason: if forbidden_hits.is_empty() {
                    None
                } else {
                    Some(forbidden_hits.join(", "))
                },
            });
        }
    }
}

fn apply_tool_event(state: &mut LiveInspectorState, tool: &ToolEvent, trace_id: &str) {
    match tool {
        ToolEvent::Called { tool_name, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolCalled,
                status: "called".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: None,
            });
        }
        ToolEvent::Completed { tool_name, result_summary, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolCompleted,
                status: "completed".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: Some(result_summary.clone()),
            });
        }
        ToolEvent::Failed { tool_name, error, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolFailed,
                status: "failed".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: Some(error.clone()),
            });
        }
        ToolEvent::Suspended { tool_name, reason, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolSuspended,
                status: "suspended".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: Some(reason.clone()),
            });
        }
        ToolEvent::Resumed { tool_name, resolution, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolResumed,
                status: "resumed".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: Some(resolution.clone()),
            });
        }
        ToolEvent::Denied { tool_name, reason, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolDenied,
                status: "denied".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: reason.clone(),
            });
        }
        ToolEvent::Deferred { tool_name, reason, .. } => {
            state.gate_tool_events.push(GateToolTimelineItem {
                trace_id: trace_id.to_string(),
                kind: GateToolKind::ToolDeferred,
                status: "deferred".into(),
                risk_level: None,
                tool_name: Some(tool_name.clone()),
                reason: Some(reason.clone()),
            });
        }
    }
}

// ── Event Summaries ─────────────────────────────────────────────────────────

fn summarize_event_kind(event: &OpenWandTraceEvent) -> String {
    match event {
        OpenWandTraceEvent::Session(e) => match e {
            SessionEvent::Started { .. } => "Session started".into(),
            SessionEvent::Ended { reason, .. } => format!("Session ended: {:?}", reason),
            SessionEvent::StepStarted { step } => format!("Step {} started", step),
            SessionEvent::StepCompleted { step, .. } => format!("Step {} completed", step),
            SessionEvent::UserMessageInjected { text } => format!("User: {}", truncate(text, 60)),
            SessionEvent::AssistantMessageGenerated { text, .. } => format!("Assistant: {}", truncate(text, 60)),
        },
        OpenWandTraceEvent::Inference(e) => match e {
            InferenceEvent::Called { model, .. } => format!("Inference called: {}", model),
            InferenceEvent::Completed { .. } => "Inference completed".into(),
            InferenceEvent::Failed { model, error, .. } => format!("Inference failed: {} — {}", model, truncate(error, 50)),
            InferenceEvent::CapabilityContextAssembled { included_skill_ids, included_goal_ids, .. } => {
                format!("Capability context assembled: {} skills, {} goals", included_skill_ids.len(), included_goal_ids.len())
            }
        },
        OpenWandTraceEvent::Gate(e) => match e {
            GateEvent::Evaluated { passed, summary, .. } => {
                format!("Gate {}: {}", if *passed { "✓" } else { "✗" }, summary)
            }
            GateEvent::BatchCompleted { total, passed, .. } => {
                format!("Gate batch: {}/{} passed", passed, total)
            }
            GateEvent::OutputScreened { passed, .. } => {
                format!("Output screened: {}", if *passed { "clean" } else { "blocked" })
            }
        },
        OpenWandTraceEvent::Tool(e) => match e {
            ToolEvent::Called { tool_name, .. } => format!("Tool called: {}", tool_name),
            ToolEvent::Completed { tool_name, .. } => format!("Tool completed: {}", tool_name),
            ToolEvent::Failed { tool_name, .. } => format!("Tool failed: {}", tool_name),
            ToolEvent::Suspended { tool_name, .. } => format!("Tool suspended: {}", tool_name),
            ToolEvent::Resumed { tool_name, .. } => format!("Tool resumed: {}", tool_name),
            ToolEvent::Denied { tool_name, .. } => format!("Tool denied: {}", tool_name),
            ToolEvent::Deferred { tool_name, .. } => format!("Tool deferred: {}", tool_name),
        },
        OpenWandTraceEvent::Memory(e) => match e {
            MemoryEvent::FactExtracted { statement, .. } => format!("Memory extracted: {}", truncate(statement, 50)),
            MemoryEvent::FactAccepted { .. } => "Memory fact accepted".into(),
            MemoryEvent::FactRejected { .. } => "Memory fact rejected".into(),
            MemoryEvent::EpisodeRecorded { .. } => "Memory episode recorded".into(),
            _ => format!("Memory: {}", event.event_kind()),
        },
        OpenWandTraceEvent::File(e) => format!("File: {}", event.event_kind()),
        OpenWandTraceEvent::Mode(e) => format!("Mode: {}", event.event_kind()),
        OpenWandTraceEvent::Workflow(e) => format!("Workflow: {}", event.event_kind()),
        OpenWandTraceEvent::Artifact(e) => format!("Artifact: {}", event.event_kind()),
    }
}

/// Extract key-value payload summary from a trace event.
/// Used by event detail drawer.
pub fn summarize_trace_event(event: &StoredEvent) -> Vec<(String, String)> {
    let inner: &OpenWandTraceEvent = event;
    match inner {
        OpenWandTraceEvent::Session(e) => match e {
            SessionEvent::Started { session_id, mode } => vec![
                ("session_id".into(), session_id.to_string()),
                ("mode".into(), format!("{:?}", mode)),
            ],
            SessionEvent::Ended { reason, total_steps, .. } => vec![
                ("reason".into(), format!("{:?}", reason)),
                ("total_steps".into(), total_steps.to_string()),
            ],
            SessionEvent::UserMessageInjected { text } => vec![
                ("text".into(), truncate(text, 200)),
            ],
            SessionEvent::AssistantMessageGenerated { text, model } => vec![
                ("model".into(), model.clone()),
                ("text".into(), truncate(text, 200)),
            ],
            SessionEvent::StepStarted { step } => vec![("step".into(), step.to_string())],
            SessionEvent::StepCompleted { step, stop_reason } => vec![
                ("step".into(), step.to_string()),
                ("stop_reason".into(), stop_reason.clone()),
            ],
        },
        OpenWandTraceEvent::Gate(e) => match e {
            GateEvent::Evaluated { gate_id, passed, risk_level, summary, .. } => {
                let mut pairs = vec![
                    ("gate_id".into(), gate_id.clone()),
                    ("passed".into(), passed.to_string()),
                    ("summary".into(), summary.clone()),
                ];
                if let Some(r) = risk_level {
                    pairs.push(("risk_level".into(), format!("{:?}", r)));
                }
                pairs
            }
            GateEvent::BatchCompleted { total, passed, failed, .. } => vec![
                ("total".into(), total.to_string()),
                ("passed".into(), passed.to_string()),
                ("failed".into(), failed.to_string()),
            ],
            GateEvent::OutputScreened { passed, forbidden_hits, fallback_used, .. } => vec![
                ("passed".into(), passed.to_string()),
                ("forbidden_hits".into(), forbidden_hits.join(", ")),
                ("fallback_used".into(), fallback_used.to_string()),
            ],
        },
        OpenWandTraceEvent::Tool(e) => match e {
            ToolEvent::Called { tool_call_id, tool_name, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
            ],
            ToolEvent::Completed { tool_call_id, tool_name, result_summary, duration_ms, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
                ("duration_ms".into(), duration_ms.to_string()),
                ("result".into(), truncate(result_summary, 200)),
            ],
            ToolEvent::Failed { tool_call_id, tool_name, error, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
                ("error".into(), truncate(error, 200)),
            ],
            ToolEvent::Suspended { tool_call_id, tool_name, reason, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
                ("reason".into(), reason.clone()),
            ],
            ToolEvent::Resumed { tool_call_id, tool_name, resolution, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
                ("resolution".into(), resolution.clone()),
            ],
            ToolEvent::Denied { tool_call_id, tool_name, reason, .. } => {
                let mut pairs = vec![
                    ("tool_call_id".into(), tool_call_id.0.clone()),
                    ("tool_name".into(), tool_name.clone()),
                ];
                if let Some(r) = reason {
                    pairs.push(("reason".into(), r.clone()));
                }
                pairs
            },
            ToolEvent::Deferred { tool_call_id, tool_name, reason, .. } => vec![
                ("tool_call_id".into(), tool_call_id.0.clone()),
                ("tool_name".into(), tool_name.clone()),
                ("reason".into(), reason.clone()),
            ],
        },
        // Generic summary for families with minimal current payloads
        _ => vec![("event_kind".into(), inner.event_kind().to_string())],
    }
}

// ── Memory Inspector Loader ─────────────────────────────────────────────────

/// Build memory inspector context from existing memory panel data.
/// No raw store queries. Reads the same governed data the memory panel shows.
pub fn load_memory_inspector(panel: Option<&UiFilteredMemoryPanel>) -> (Option<MemoryInspectorContext>, Vec<MemoryEvidenceItem>) {
    match panel {
        Some(panel) => {
            let ctx = MemoryInspectorContext {
                retrieved_count: panel.summary.total(),
                included_count: panel.summary.prompt_included,
                excluded_count: panel.summary.stale
                    + panel.summary.superseded_ignored
                    + panel.summary.unverifiable,
                stale_count: panel.summary.stale,
                superseded_count: panel.summary.superseded_ignored,
                unverifiable_count: panel.summary.unverifiable,
                conflicts_count: panel.summary.conflicts,
                prompt_context_available: !panel.prompt_included.is_empty(),
                consistency_report_available: panel.generated_at > 0,
            };

            let mut evidence = Vec::new();

            // Prompt-included claims
            for row in &panel.prompt_included {
                evidence.push(MemoryEvidenceItem {
                    claim_summary: truncate(&row.claim, 100),
                    status: MemoryEvidenceStatus::Included,
                    reason: row.inclusion_reason.as_ref().map(|r| format!("{:?}", r)).unwrap_or_default(),
                    source_trace_ids: row.source_traces.clone(),
                    confidence: row.confidence,
                });
            }

            // Stale
            for row in &panel.stale {
                evidence.push(MemoryEvidenceItem {
                    claim_summary: truncate(&row.claim, 100),
                    status: MemoryEvidenceStatus::ExcludedStale,
                    reason: "Stale claim".into(),
                    source_trace_ids: row.source_traces.clone(),
                    confidence: row.confidence,
                });
            }

            // Superseded
            for row in &panel.superseded_ignored {
                evidence.push(MemoryEvidenceItem {
                    claim_summary: truncate(&row.claim, 100),
                    status: MemoryEvidenceStatus::ExcludedSuperseded,
                    reason: row.superseded_by.as_ref().map(|s| format!("Superseded by {}", s)).unwrap_or_default(),
                    source_trace_ids: row.source_traces.clone(),
                    confidence: row.confidence,
                });
            }

            // Unverifiable
            for row in &panel.unverifiable {
                evidence.push(MemoryEvidenceItem {
                    claim_summary: truncate(&row.claim, 100),
                    status: MemoryEvidenceStatus::ExcludedUnverifiable,
                    reason: "Unverifiable".into(),
                    source_trace_ids: row.source_traces.clone(),
                    confidence: row.confidence,
                });
            }

            // Conflicts
            for group in &panel.conflicts {
                for row in &group.claims {
                    evidence.push(MemoryEvidenceItem {
                        claim_summary: truncate(&row.claim, 100),
                        status: MemoryEvidenceStatus::ExcludedConflict,
                        reason: format!("Conflict group: {}", group.group_id),
                        source_trace_ids: row.source_traces.clone(),
                        confidence: row.confidence,
                    });
                }
            }

            // Missing in memory
            for row in &panel.missing_in_memory {
                let key = row.repo_evidence_key.first().cloned().unwrap_or_default();
                evidence.push(MemoryEvidenceItem {
                    claim_summary: key,
                    status: MemoryEvidenceStatus::Missing,
                    reason: "Missing in memory".into(),
                    source_trace_ids: vec![],
                    confidence: None,
                });
            }

            (Some(ctx), evidence)
        }
        None => (None, Vec::new()),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
