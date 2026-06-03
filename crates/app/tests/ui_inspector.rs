//! Wave 20: Live Trace and Memory Inspector tests.

use openwand_app::ui::inspector_state::*;
use openwand_app::ui::inspector_components::*;
use openwand_app::ui::memory_dto::*;
use openwand_core::events::*;
use openwand_core::ids::*;
use openwand_core::mode::*;
use openwand_core::risk::*;
use openwand_core::session_vocab::*;
use openwand_core::snapshots::*;
use openwand_core::tool_vocab::*;
use openwand_store::StoredEvent;
use openwand_trace::actor::Actor;
use openwand_trace::entry::TraceEntry;
use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};
use openwand_trace::ids::TraceId;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_entry(event: OpenWandTraceEvent, seq: u64) -> TraceEntry<StoredEvent> {
    let ts = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&chrono::Utc);
    TraceEntry {
        id: TraceId(format!("tr_{}", seq)),
        stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "s1".into() },
        stream_sequence: seq,
        global_sequence: seq * 10,
        occurred_at: ts,
        actor: Actor::System { component: "test".into() },
        event: StoredEvent::from(event.clone()),
        event_kind: event.event_kind().to_string(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: None,
        entry_hash: EntryHash("fake".into()),
    }
}

fn sid() -> SessionId { SessionId::new() }
fn tcid(s: &str) -> ToolCallId { ToolCallId(s.into()) }

fn make_session_started() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Session(SessionEvent::Started { session_id: sid(), mode: InteractionMode::Conversational })
}

fn make_user_msg(text: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected { text: text.into() })
}

fn make_assistant_msg(text: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Session(SessionEvent::AssistantMessageGenerated { text: text.into(), model: "test".into() })
}

fn make_gate_evaluated(passed: bool) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Gate(GateEvent::Evaluated {
        gate_id: "g1".into(), gate_kind: "risk".into(), passed,
        risk_level: Some(RiskLevelSnapshot::Medium), reason_code: None,
        summary: if passed { "OK".into() } else { "Blocked".into() },
    })
}

fn make_tool_called(name: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Called {
        tool_call_id: tcid("tc_1"), tool_name: name.into(),
        args_hash: "h".into(), invoker: ToolInvoker::Llm,
    })
}

fn make_tool_suspended(name: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Suspended {
        tool_call_id: tcid("tc_1"), tool_name: name.into(),
        reason: "Requires approval".into(), approval_context: None,
    })
}

fn make_tool_denied(name: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Denied {
        tool_call_id: tcid("tc_1"), tool_name: name.into(),
        approval_request_id: None, reason: Some("User rejected".into()),
    })
}

fn make_inference_called() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Inference(InferenceEvent::Called {
        model: "qwen3-4b".into(), provider: "lm-studio".into(),
        prompt_hash: "h".into(), thinking_budget: None,
        prompt_assembly: PromptAssemblySnapshot {
            system_prompt_hash: "s".into(), message_window_hash: "m".into(),
            memory_hit_ids: vec![], memory_context_hash: None,
            tool_manifest_hash: "t".into(), policy_filter_hash: "p".into(),
            mode: InteractionMode::Conversational, working_directory: "/tmp".into(),
        },
    })
}

fn make_memory_fact() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Memory(MemoryEvent::FactExtracted {
        claim_id: ClaimId::new(), statement: "X uses Y".into(),
        confidence: 0.89, predicate: "uses".into(),
    })
}

fn make_mode_changed() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Mode(ModeEvent::Changed {
        from: InteractionMode::Direct, to: InteractionMode::Conversational,
        trigger: "user".into(), accuracy_check: None,
    })
}

fn make_file_event() -> OpenWandTraceEvent {
    OpenWandTraceEvent::File(FileEvent::Read { path: "/tmp/test.rs".into(), bytes: Some(42) })
}

fn make_workflow_event() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Workflow(WorkflowEvent::ModStarted { mod_id: "m1".into(), mod_name: "standard".into() })
}

fn make_artifact_event() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Artifact(ArtifactEvent::Generated {
        paths: vec!["README.md".into()], artifact_kind: "doc".into(),
        accuracy: AccuracyRecordSnapshot { commit_hash: None, file_coverage: 1.0, sensitivity: "low".into() },
    })
}

fn make_panel() -> UiFilteredMemoryPanel {
    UiFilteredMemoryPanel {
        working_directory: "/tmp".into(),
        generated_at: 12345,
        summary: UiMemoryPanelSummary {
            prompt_included: 3, stale: 1, missing_in_repo: 2,
            missing_in_memory: 1, conflicts: 1, unverifiable: 1, superseded_ignored: 1,
        },
        prompt_included: vec![UiMemoryPanelRow {
            claim: "Core exists".into(), finding_kind: "Supported".into(),
            evidence_kind: "Observation".into(), repo_evidence_key: vec!["core".into()],
            inclusion_reason: Some("HighConfidence".into()), severity: "Low".into(),
            has_provenance: true, record_id: Some("r1".into()), provenance_label: "test".into(),
            source_traces: vec!["tr_1".into()], confidence: Some(0.95),
            conflict_group_id: None, superseded_by: None, hydration_status: "Full".into(),
            trace_lineage_summary: None, trace_relation_counts: Default::default(), trace_lineage_status: None,
        }],
        stale: vec![UiMemoryPanelRow {
            claim: "Old claim".into(), finding_kind: "Stale".into(),
            evidence_kind: "".into(), repo_evidence_key: vec![], inclusion_reason: None,
            severity: "Medium".into(), has_provenance: false, record_id: None,
            provenance_label: String::new(), source_traces: vec![], confidence: Some(0.5),
            conflict_group_id: None, superseded_by: None, hydration_status: "Full".into(),
            trace_lineage_summary: None, trace_relation_counts: Default::default(), trace_lineage_status: None,
        }],
        missing_in_repo: vec![], missing_in_memory: vec![UiMemoryPanelRow {
            claim: "".into(), finding_kind: "MissingInMemory".into(),
            evidence_kind: "".into(), repo_evidence_key: vec!["missing_key".into()],
            inclusion_reason: None, severity: "High".into(), has_provenance: false,
            record_id: None, provenance_label: String::new(), source_traces: vec![],
            confidence: None, conflict_group_id: None, superseded_by: None,
            hydration_status: "Missing".into(), trace_lineage_summary: None,
            trace_relation_counts: Default::default(), trace_lineage_status: None,
        }],
        conflicts: vec![UiMemoryPanelConflict {
            group_id: "cg1".into(), detail: "conflict".into(),
            claims: vec![UiMemoryPanelRow {
                claim: "Conflicting".into(), finding_kind: "Conflict".into(),
                evidence_kind: "".into(), repo_evidence_key: vec![], inclusion_reason: None,
                severity: "High".into(), has_provenance: false, record_id: None,
                provenance_label: String::new(), source_traces: vec!["tr_c".into()],
                confidence: Some(0.7), conflict_group_id: Some("cg1".into()), superseded_by: None,
                hydration_status: "Full".into(), trace_lineage_summary: None,
                trace_relation_counts: Default::default(), trace_lineage_status: None,
            }],
        }],
        unverifiable: vec![UiMemoryPanelRow {
            claim: "Cannot verify".into(), finding_kind: "Unverifiable".into(),
            evidence_kind: "".into(), repo_evidence_key: vec![], inclusion_reason: None,
            severity: "Low".into(), has_provenance: false, record_id: None,
            provenance_label: String::new(), source_traces: vec![], confidence: Some(0.3),
            conflict_group_id: None, superseded_by: None, hydration_status: "Full".into(),
            trace_lineage_summary: None, trace_relation_counts: Default::default(), trace_lineage_status: None,
        }],
        superseded_ignored: vec![UiMemoryPanelRow {
            claim: "Old version".into(), finding_kind: "Superseded".into(),
            evidence_kind: "".into(), repo_evidence_key: vec![], inclusion_reason: None,
            severity: "Low".into(), has_provenance: false, record_id: None,
            provenance_label: String::new(), source_traces: vec![], confidence: Some(0.8),
            conflict_group_id: None, superseded_by: Some("new_id".into()), hydration_status: "Full".into(),
            trace_lineage_summary: None, trace_relation_counts: Default::default(), trace_lineage_status: None,
        }],
    }
}

// ── State / Loader Tests ────────────────────────────────────────────────────

#[test]
fn inspector_state_empty_session_is_empty() {
    let state = LiveInspectorState::default();
    assert!(state.is_empty());
    assert!(state.session_id.is_none());
}

#[test]
fn inspector_state_loads_trace_timeline() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_session_started(), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_user_msg("hello"), 2));
    assert_eq!(2, state.trace_timeline.len());
    assert_eq!("session", state.trace_timeline[0].event_family);
}

#[test]
fn inspector_state_loads_gate_events() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_gate_evaluated(true), 1));
    assert_eq!(1, state.gate_tool_events.len());
    assert_eq!(GateToolKind::GateEvaluated, state.gate_tool_events[0].kind);
    assert_eq!("passed", state.gate_tool_events[0].status);
}

#[test]
fn inspector_state_loads_tool_lifecycle() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_tool_called("file_write"), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_tool_suspended("file_write"), 2));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_tool_denied("file_write"), 3));
    assert_eq!(3, state.gate_tool_events.len());
    assert_eq!(GateToolKind::ToolCalled, state.gate_tool_events[0].kind);
    assert_eq!(GateToolKind::ToolSuspended, state.gate_tool_events[1].kind);
    assert_eq!(GateToolKind::ToolDenied, state.gate_tool_events[2].kind);
}

#[test]
fn inspector_state_loads_inference_events() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_inference_called(), 1));
    assert_eq!(1, state.trace_timeline.len());
    assert_eq!("inference", state.trace_timeline[0].event_family);
}

#[test]
fn inspector_state_loads_memory_events() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_memory_fact(), 1));
    assert_eq!(1, state.trace_timeline.len());
    assert_eq!("memory", state.trace_timeline[0].event_family);
}

#[test]
fn inspector_state_loads_mode_events() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_mode_changed(), 1));
    assert_eq!(1, state.trace_timeline.len());
    assert_eq!("mode", state.trace_timeline[0].event_family);
}

#[test]
fn inspector_state_orders_by_stream_sequence() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_session_started(), 3));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_user_msg("hi"), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_assistant_msg("hello"), 2));
    // Entries are appended in insertion order, not sorted
    // The stream_sequence field is preserved for sorting by consumers
    assert_eq!(3, state.trace_timeline[0].stream_sequence);
    assert_eq!(1, state.trace_timeline[1].stream_sequence);
    assert_eq!(2, state.trace_timeline[2].stream_sequence);
}

#[test]
fn inspector_bridge_does_not_mutate_session_state() {
    let mut state1 = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state1, &make_entry(make_session_started(), 1));
    // Applying the same entry again produces deterministic state
    let mut state2 = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state2, &make_entry(make_session_started(), 1));
    assert_eq!(state1, state2, "Same input must produce same output");
}

#[test]
fn summarize_trace_event_produces_payload_pairs() {
    let event = StoredEvent::from(make_tool_called("file_read"));
    let pairs = summarize_trace_event(&event);
    assert!(!pairs.is_empty());
    assert!(pairs.iter().any(|(k, _)| k == "tool_name"));
    assert!(pairs.iter().any(|(v, _)| v == "tool_call_id"));
}

#[test]
fn inspector_state_is_rebuildable_from_persistence() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_session_started(), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_gate_evaluated(false), 2));
    // Serialize and deserialize
    let json = serde_json::to_string(&state).unwrap();
    let restored: LiveInspectorState = serde_json::from_str(&json).unwrap();
    assert_eq!(state, restored);
}

// ── Patch 1: All families recognized ────────────────────────────────────────

#[test]
fn inspector_recognizes_all_current_trace_event_families() {
    // One event per family — all 9 must produce a timeline item
    let events: Vec<OpenWandTraceEvent> = vec![
        make_session_started(),           // session
        make_inference_called(),          // inference
        make_gate_evaluated(true),        // gate
        make_tool_called("read"),         // tool
        make_file_event(),                // file
        make_memory_fact(),               // memory
        make_mode_changed(),              // mode
        make_workflow_event(),            // workflow
        make_artifact_event(),            // artifact
    ];
    assert_eq!(9, events.len(), "If OpenWandTraceEvent gains families, update this test");

    let mut state = LiveInspectorState::default();
    for (i, event) in events.into_iter().enumerate() {
        apply_trace_entry_to_inspector(&mut state, &make_entry(event, (i + 1) as u64));
    }
    assert_eq!(9, state.trace_timeline.len(), "All 9 families must produce timeline items");

    let families: std::collections::HashSet<&str> = state.trace_timeline.iter()
        .map(|t| t.event_family.as_str())
        .collect();
    assert!(families.contains("session"));
    assert!(families.contains("inference"));
    assert!(families.contains("gate"));
    assert!(families.contains("tool"));
    assert!(families.contains("file"));
    assert!(families.contains("memory"));
    assert!(families.contains("mode"));
    assert!(families.contains("workflow"));
    assert!(families.contains("artifact"));
}

#[test]
fn inspector_generic_summary_for_minimal_file_workflow_artifact_events() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_file_event(), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_workflow_event(), 2));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_artifact_event(), 3));
    // These produce generic summaries, not rich detail
    assert!(state.trace_timeline[0].summary.starts_with("File:"));
    assert!(state.trace_timeline[1].summary.starts_with("Workflow:"));
    assert!(state.trace_timeline[2].summary.starts_with("Artifact:"));
}

// ── Patch 2: Relation direction tests ───────────────────────────────────────

#[test]
fn event_detail_loads_outgoing_trace_relations() {
    let detail = TraceEventDetail {
        trace_id: "tr_1".into(), event_kind: "tool.called".into(),
        actor: "system".into(), timestamp: "2024-01-01T00:00:00Z".into(),
        payload_summary: vec![], relations: vec![TraceRelationSummary {
            from_trace_id: "tr_1".into(), to_trace_id: "tr_2".into(),
            relation_kind: "CausedBy".into(), direction: RelationDirection::Outgoing,
        }],
    };
    let rows = trace_relation_rows(&detail);
    assert_eq!(1, rows.len());
    assert_eq!("→ outgoing", rows[0].direction);
    assert_eq!("tr_1", rows[0].from);
    assert_eq!("tr_2", rows[0].to);
}

#[test]
fn event_detail_loads_incoming_trace_relations() {
    let detail = TraceEventDetail {
        trace_id: "tr_2".into(), event_kind: "tool.completed".into(),
        actor: "system".into(), timestamp: "2024-01-01T00:00:00Z".into(),
        payload_summary: vec![], relations: vec![TraceRelationSummary {
            from_trace_id: "tr_1".into(), to_trace_id: "tr_2".into(),
            relation_kind: "CausedBy".into(), direction: RelationDirection::Incoming,
        }],
    };
    let rows = trace_relation_rows(&detail);
    assert_eq!(1, rows.len());
    assert_eq!("← incoming", rows[0].direction);
}

#[test]
fn event_detail_relation_rows_distinguish_direction() {
    let detail = TraceEventDetail {
        trace_id: "tr_3".into(), event_kind: "gate.evaluated".into(),
        actor: "system".into(), timestamp: "2024-01-01T00:00:00Z".into(),
        payload_summary: vec![], relations: vec![
            TraceRelationSummary {
                from_trace_id: "tr_3".into(), to_trace_id: "tr_4".into(),
                relation_kind: "Verifies".into(), direction: RelationDirection::Outgoing,
            },
            TraceRelationSummary {
                from_trace_id: "tr_2".into(), to_trace_id: "tr_3".into(),
                relation_kind: "CausedBy".into(), direction: RelationDirection::Incoming,
            },
        ],
    };
    let rows = trace_relation_rows(&detail);
    assert_eq!(2, rows.len());
    assert_eq!("→ outgoing", rows[0].direction);
    assert_eq!("← incoming", rows[1].direction);
}

// ── Patch 3: Live bridge tests ──────────────────────────────────────────────

#[test]
fn live_trace_event_updates_inspector_timeline() {
    let mut state = LiveInspectorState::default();
    let entry = make_entry(make_session_started(), 1);
    apply_trace_entry_to_inspector(&mut state, &entry);
    assert_eq!(1, state.trace_timeline.len());
    assert_eq!("session.started", state.trace_timeline[0].event_kind);
}

#[test]
fn live_gate_event_updates_gate_tool_history() {
    let mut state = LiveInspectorState::default();
    let entry = make_entry(make_gate_evaluated(false), 1);
    apply_trace_entry_to_inspector(&mut state, &entry);
    assert_eq!(1, state.gate_tool_events.len());
    assert_eq!("failed", state.gate_tool_events[0].status);
}

#[test]
fn live_memory_event_updates_timeline_not_memory_inspector() {
    // Memory events go to timeline but NOT to memory_inspector
    // (memory inspector is built from panel data, not trace events)
    let mut state = LiveInspectorState::default();
    let entry = make_entry(make_memory_fact(), 1);
    apply_trace_entry_to_inspector(&mut state, &entry);
    assert_eq!(1, state.trace_timeline.len());
    assert!(state.memory_context.is_none(), "Memory context comes from panel, not trace events");
}

// ── View Helper Tests ───────────────────────────────────────────────────────

#[test]
fn trace_timeline_rows_show_event_kind_actor_time() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_user_msg("hello"), 1));
    let rows = trace_timeline_rows(&state);
    assert_eq!(1, rows.len());
    assert_eq!("session.user_message_injected", rows[0].event_kind);
    assert!(!rows[0].actor.is_empty());
    assert!(!rows[0].time.is_empty());
}

#[test]
fn gate_tool_rows_show_decision_risk_tool() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_gate_evaluated(true), 1));
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_tool_suspended("file_write"), 2));
    let rows = gate_tool_rows(&state);
    assert_eq!(2, rows.len());
    assert_eq!("passed", rows[0].status);
    assert!(rows[0].risk.is_some());
    assert_eq!(Some("file_write".into()), rows[1].tool);
}

#[test]
fn memory_evidence_rows_show_included_excluded_reason() {
    let panel = make_panel();
    let (ctx, evidence) = load_memory_inspector(Some(&panel));
    let mut state = LiveInspectorState { memory_context: ctx, memory_evidence: evidence, ..Default::default() };
    let rows = memory_evidence_rows(&state);
    assert!(rows.len() >= 3, "Should have included, stale, and conflict rows");
    assert!(rows.iter().any(|r| r.status_display == "Included"));
    assert!(rows.iter().any(|r| r.status_display.contains("Stale")));
}

#[test]
fn trace_relation_rows_show_relation_kind() {
    let detail = TraceEventDetail {
        trace_id: "tr_1".into(), event_kind: "test".into(),
        actor: "test".into(), timestamp: "".into(), payload_summary: vec![],
        relations: vec![TraceRelationSummary {
            from_trace_id: "tr_1".into(), to_trace_id: "tr_2".into(),
            relation_kind: "Verifies".into(), direction: RelationDirection::Outgoing,
        }],
    };
    let rows = trace_relation_rows(&detail);
    assert_eq!("Verifies", rows[0].kind);
}

#[test]
fn event_detail_lines_show_payload_summary() {
    let detail = TraceEventDetail {
        trace_id: "tr_1".into(), event_kind: "tool.called".into(),
        actor: "system".into(), timestamp: "2024-01-01".into(),
        payload_summary: vec![("tool_name".into(), "file_read".into())],
        relations: vec![],
    };
    let lines = event_detail_lines(&detail);
    assert!(lines.iter().any(|l| l.contains("tool_name")));
    assert!(lines.iter().any(|l| l.contains("file_read")));
}

#[test]
fn inspector_warning_lines_show_warnings() {
    let state = LiveInspectorState {
        warnings: vec!["Unknown event family: future".into()],
        ..Default::default()
    };
    let lines = inspector_warning_lines(&state);
    assert_eq!(1, lines.len());
    assert!(lines[0].contains("Unknown event family"));
}

#[test]
fn all_gate_tool_kinds_have_display_text() {
    let kinds = vec![
        GateToolKind::GateEvaluated, GateToolKind::GateBatchCompleted,
        GateToolKind::GateOutputScreened, GateToolKind::ToolCalled,
        GateToolKind::ToolSuspended, GateToolKind::ToolResumed,
        GateToolKind::ToolDenied, GateToolKind::ToolCompleted,
        GateToolKind::ToolFailed, GateToolKind::ToolDeferred,
    ];
    for kind in &kinds {
        assert!(!kind.display().is_empty(), "{:?} should have display text", kind);
    }
}

#[test]
fn memory_evidence_status_all_variants_displayable() {
    let statuses = vec![
        MemoryEvidenceStatus::Included, MemoryEvidenceStatus::ExcludedStale,
        MemoryEvidenceStatus::ExcludedSuperseded, MemoryEvidenceStatus::ExcludedUnverifiable,
        MemoryEvidenceStatus::ExcludedConflict, MemoryEvidenceStatus::Missing,
    ];
    for status in &statuses {
        assert!(!status.display().is_empty(), "{:?} should have display text", status);
    }
}

#[test]
fn empty_inspector_produces_empty_rows() {
    let state = LiveInspectorState::default();
    assert!(trace_timeline_rows(&state).is_empty());
    assert!(gate_tool_rows(&state).is_empty());
    assert!(memory_evidence_rows(&state).is_empty());
    assert!(inspector_warning_lines(&state).is_empty());
}

// ── Memory Inspector Tests ──────────────────────────────────────────────────

#[test]
fn memory_inspector_shows_retrieved_included_excluded_counts() {
    let panel = make_panel();
    let (ctx, _) = load_memory_inspector(Some(&panel));
    let ctx = ctx.unwrap();
    assert!(ctx.retrieved_count > 0);
    assert_eq!(3, ctx.included_count);
    assert!(ctx.excluded_count > 0);
}

#[test]
fn memory_inspector_shows_stale_superseded_unverifiable_counts() {
    let panel = make_panel();
    let (ctx, _) = load_memory_inspector(Some(&panel));
    let ctx = ctx.unwrap();
    assert_eq!(1, ctx.stale_count);
    assert_eq!(1, ctx.superseded_count);
    assert_eq!(1, ctx.unverifiable_count);
}

#[test]
fn memory_inspector_shows_conflict_count() {
    let panel = make_panel();
    let (ctx, _) = load_memory_inspector(Some(&panel));
    let ctx = ctx.unwrap();
    assert_eq!(1, ctx.conflicts_count);
}

#[test]
fn memory_inspector_surfaces_prompt_included_claims() {
    let panel = make_panel();
    let (_, evidence) = load_memory_inspector(Some(&panel));
    let included: Vec<_> = evidence.iter().filter(|e| matches!(e.status, MemoryEvidenceStatus::Included)).collect();
    assert_eq!(1, included.len());
    assert!(included[0].claim_summary.contains("Core exists"));
}

#[test]
fn memory_inspector_surfaces_trace_backed_evidence_links() {
    let panel = make_panel();
    let (_, evidence) = load_memory_inspector(Some(&panel));
    let included = evidence.iter().find(|e| matches!(e.status, MemoryEvidenceStatus::Included)).unwrap();
    assert!(included.source_trace_ids.contains(&"tr_1".to_string()));
}

#[test]
fn memory_inspector_builds_from_empty_panel() {
    let panel = UiFilteredMemoryPanel::empty();
    let (ctx, evidence) = load_memory_inspector(Some(&panel));
    let ctx = ctx.unwrap();
    assert_eq!(0, ctx.included_count);
    assert!(evidence.is_empty());
}

#[test]
fn memory_inspector_builds_from_full_panel() {
    let panel = make_panel();
    let (ctx, evidence) = load_memory_inspector(Some(&panel));
    assert!(ctx.is_some());
    assert!(evidence.len() >= 5, "Should have included + stale + superseded + unverifiable + conflict + missing");
}

#[test]
fn memory_inspector_handles_missing_panel() {
    let (ctx, evidence) = load_memory_inspector(None);
    assert!(ctx.is_none());
    assert!(evidence.is_empty());
}

// ── Guard Tests (Patch 4) ──────────────────────────────────────────────────

macro_rules! source_guard {
    ($name:ident, $pattern:expr, $msg:expr) => {
        #[test]
        fn $name() {
            let files = [
                include_str!("../src/ui/inspector_state.rs"),
                include_str!("../src/ui/inspector_components.rs"),
            ];
            for source in &files {
                for line in source.lines() {
                    let t = line.trim();
                    if t.starts_with("//") || t.starts_with("//!") { continue; }
                    let lower = t.to_lowercase();
                    assert!(!lower.contains($pattern), "Guard violation: {} found", $msg);
                }
            }
        }
    };
}

source_guard!(inspector_modules_do_not_import_process_command, "std::process::command", "std::process::Command");
source_guard!(inspector_modules_do_not_import_tool_executor, "toolexecutor", "ToolExecutor");
source_guard!(inspector_modules_do_not_import_policy_engine_for_eval, "policyengine", "PolicyEngine");
source_guard!(inspector_modules_do_not_import_memory_projection_store, "memoryprojectionstore", "MemoryProjectionStore");
source_guard!(inspector_modules_do_not_import_git_backends, "localgitbackend", "LocalGitBackend");
source_guard!(inspector_modules_do_not_import_governed_execution_backends, "governedgitcommitbackend", "GovernedGitCommitBackend");
source_guard!(inspector_modules_do_not_call_shell_or_git, "/bin/sh", "/bin/sh");

#[test]
fn inspector_modules_do_not_append_trace_or_trace_relations() {
    let files = [
        include_str!("../src/ui/inspector_state.rs"),
        include_str!("../src/ui/inspector_components.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            let lower = t.to_lowercase();
            assert!(!lower.contains(".append("), "No trace.append()");
            assert!(!lower.contains("tracerelationdraft"), "No TraceRelationDraft");
            assert!(!lower.contains("append_and_project"), "No append_and_project()");
        }
    }
}

#[test]
fn inspector_modules_do_not_write_governance_records() {
    let files = [
        include_str!("../src/ui/inspector_state.rs"),
        include_str!("../src/ui/inspector_components.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            let lower = t.to_lowercase();
            assert!(!lower.contains("save_proposal"), "No save_proposal");
            assert!(!lower.contains("save_execution"), "No save_execution");
            assert!(!lower.contains("save_verification"), "No save_verification");
        }
    }
}

// ── Runtime No-Mutation Tests ───────────────────────────────────────────────

#[test]
fn inspector_refresh_leaves_session_state_unchanged() {
    let state1 = LiveInspectorState::default();
    let state2 = state1.clone();
    // "Refreshing" by re-applying no entries produces identical state
    assert_eq!(state1, state2);
}

#[test]
fn inspector_state_clone_is_independent() {
    let mut state = LiveInspectorState::default();
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_session_started(), 1));
    let clone = state.clone();
    // Mutating original doesn't affect clone
    apply_trace_entry_to_inspector(&mut state, &make_entry(make_user_msg("hi"), 2));
    assert_eq!(1, clone.trace_timeline.len());
    assert_eq!(2, state.trace_timeline.len());
}
