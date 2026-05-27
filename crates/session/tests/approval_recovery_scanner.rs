//! Recovery scanner tests for approval persistence.
//!
//! Tests build_recovery_index with synthetic trace entries.

use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
use openwand_core::ids::{GateId, ToolCallId, ApprovalRequestId};
use openwand_core::snapshots::ApprovalContextSnapshot;
use openwand_session::approval_recovery::{
    build_recovery_index, ApprovalRecoveryConflict, ApprovalRecoveryIndex,
};
use openwand_store::StoredEvent;
use openwand_trace::entry::TraceEntry;
use openwand_trace::ids::TraceId;
use openwand_trace::stream::{TraceStreamId, TraceStreamScope, EntryHash};

fn make_entry(event: OpenWandTraceEvent) -> TraceEntry<StoredEvent> {
    TraceEntry {
        id: TraceId::new(),
        stream_id: TraceStreamId {
            scope: TraceStreamScope::Session,
            id: "test".into(),
        },
        stream_sequence: 0,
        global_sequence: 0,
        occurred_at: chrono::Utc::now(),
        actor: openwand_trace::actor::Actor::System {
            component: "test".into(),
        },
        event: StoredEvent::from(event),
        event_kind: "test".into(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: None,
        entry_hash: EntryHash("test".into()),
    }
}

fn make_context() -> ApprovalContextSnapshot {
    ApprovalContextSnapshot {
        approval_request_id: ApprovalRequestId::new(),
        gate_id: GateId::new(),
        step: 1,
        tool_call_id: ToolCallId::new(),
        tool_name: "local__file_write".into(),
        arguments: serde_json::json!({"path": "test.txt", "content": "hello"}),
        args_hash: "sha256:abc".into(),
        declared_effect: openwand_core::ToolEffect::Write,
        risk_level: openwand_core::RiskLevelSnapshot::Medium,
        confirmation_level: openwand_core::ConfirmationLevel::Approve,
        reason_code: "write-requires-approve".into(),
        policy_summary: "Write requires approval".into(),
        requested_action_summary: "Write test.txt".into(),
        rollback_plan: None,
        metadata: serde_json::Value::Null,
    }
}

#[test]
fn recovery_scanner_finds_unresolved() {
    let ctx = make_context();
    let tc_id = ctx.tool_call_id.clone();
    let entry = make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
        tool_call_id: tc_id.clone(),
        tool_name: "local__file_write".into(),
        reason: "awaiting approval".into(),
        approval_context: Some(ctx),
    }));

    let index = build_recovery_index(&[entry]);
    assert_eq!(1, index.pending.len());
    assert_eq!("local__file_write", index.pending[0].tool_name);
    assert!(index.deferred.is_empty());
    assert!(index.uncertain.is_empty());
    assert!(index.conflicts.is_empty());
    assert!(index.has_single_pending_approval());
    assert!(!index.is_recovery_blocked());
}

#[test]
fn recovery_scanner_ignores_resumed() {
    let ctx = make_context();
    let tc_id = ctx.tool_call_id.clone();
    let ar_id = ctx.approval_request_id.clone();

    let entries = vec![
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            reason: "awaiting approval".into(),
            approval_context: Some(ctx),
        })),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Resumed {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            resolution: "approved".into(),
            approval_request_id: Some(ar_id),
        })),
    ];

    let index = build_recovery_index(&entries);
    assert!(index.pending.is_empty(), "Resumed tool should not be pending");
}

#[test]
fn recovery_scanner_ignores_denied() {
    let ctx = make_context();
    let tc_id = ctx.tool_call_id.clone();

    let entries = vec![
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            reason: "awaiting approval".into(),
            approval_context: Some(ctx),
        })),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Denied {
            tool_call_id: tc_id,
            tool_name: "local__file_write".into(),
            approval_request_id: None,
            reason: Some("user_rejected".into()),
        })),
    ];

    let index = build_recovery_index(&entries);
    assert!(index.pending.is_empty(), "Denied tool should not be pending");
}

#[test]
fn recovery_scanner_detects_uncertain() {
    let tc_id = ToolCallId::new();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            args_hash: "h".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        })),
    ];

    let index = build_recovery_index(&entries);
    assert_eq!(1, index.uncertain.len(), "Called without terminal should be uncertain");
    assert!(index.is_recovery_blocked());
}

#[test]
fn recovery_scanner_completed_is_not_uncertain() {
    let tc_id = ToolCallId::new();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            args_hash: "h".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        })),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Completed {
            tool_call_id: tc_id,
            tool_name: "local__file_write".into(),
            status: openwand_core::tool_vocab::ToolResultStatus::Success,
            result_summary: "ok".into(),
            duration_ms: 100,
        })),
    ];

    let index = build_recovery_index(&entries);
    assert!(index.uncertain.is_empty(), "Completed tool should not be uncertain");
}

#[test]
fn recovery_scanner_failed_is_not_uncertain() {
    let tc_id = ToolCallId::new();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: tc_id.clone(),
            tool_name: "local__file_write".into(),
            args_hash: "h".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        })),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Failed {
            tool_call_id: tc_id,
            tool_name: "local__file_write".into(),
            error: "disk full".into(),
        })),
    ];

    let index = build_recovery_index(&entries);
    assert!(index.uncertain.is_empty(), "Failed tool should not be uncertain");
}

#[test]
fn recovery_scanner_collects_deferred() {
    let tc_id = ToolCallId::new();
    let ar_id = ApprovalRequestId::new();
    let entry = make_entry(OpenWandTraceEvent::Tool(ToolEvent::Deferred {
        tool_call_id: tc_id,
        tool_name: "local__write_B".into(),
        reason: "deferred: another approval pending".into(),
        blocked_by_tool_call_id: None,
        blocked_by_approval_request_id: Some(ar_id.clone()),
        original_order_index: Some(1),
        args_hash: Some("h2".into()),
    }));

    let index = build_recovery_index(&[entry]);
    assert_eq!(1, index.deferred.len());
    assert_eq!("local__write_B", index.deferred[0].tool_name);
    assert_eq!(Some(ar_id), index.deferred[0].blocked_by_approval_request_id);
    assert_eq!(Some(1), index.deferred[0].original_order_index);
}

#[test]
fn recovery_scanner_flags_missing_approval_context() {
    let tc_id = ToolCallId::new();
    let entry = make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
        tool_call_id: tc_id.clone(),
        tool_name: "local__file_write".into(),
        reason: "awaiting approval".into(),
        approval_context: None, // Pre-03d event
    }));

    let index = build_recovery_index(&[entry]);
    assert!(index.pending.is_empty(), "No context = not recoverable");
    assert_eq!(1, index.conflicts.len());
    match &index.conflicts[0] {
        ApprovalRecoveryConflict::SuspendedMissingApprovalContext { tool_call_id } => {
            assert_eq!(&tc_id, tool_call_id);
        }
        other => panic!("Expected SuspendedMissingApprovalContext, got: {other:?}"),
    }
    assert!(index.is_recovery_blocked());
}

#[test]
fn recovery_scanner_flags_multiple_unresolved() {
    let mut entries = vec![];
    for i in 0..2 {
        let ctx = make_context();
        let tc_id = ctx.tool_call_id.clone();
        entries.push(make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: tc_id,
            tool_name: format!("local__write_{i}"),
            reason: "awaiting approval".into(),
            approval_context: Some(ctx),
        })));
    }

    let index = build_recovery_index(&entries);
    assert_eq!(2, index.pending.len());
    assert!(index.is_recovery_blocked());

    // Should have MultipleUnresolvedApprovals conflict
    assert!(index
        .conflicts
        .iter()
        .any(|c| matches!(c, ApprovalRecoveryConflict::MultipleUnresolvedApprovals { .. })));
}
