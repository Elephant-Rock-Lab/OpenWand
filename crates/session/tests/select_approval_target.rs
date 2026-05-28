//! Tests for the pure select_approval_target function.
//!
//! These tests do not need mocks, trace, tools, or async runtime.
//! They test the selection logic over index data + cache hint + decision.

use openwand_core::ids::{ApprovalRequestId, GateId, ToolCallId};
use openwand_core::snapshots::ApprovalContextSnapshot;
use openwand_core::ToolEffect;
use openwand_session::approval_recovery::{
    ApprovalRecoveryIndex, PendingApprovalRecovery, ResolvedApprovalRecovery, ResolvedApprovalKind,
};
use openwand_session::runner::{
    select_approval_target, ApprovalDecision, ApprovalResolution, ApprovalSource,
};
use openwand_trace::TraceId;

fn make_pending(name: &str) -> PendingApprovalRecovery {
    PendingApprovalRecovery {
        suspended_trace_id: TraceId::new(),
        context: ApprovalContextSnapshot {
            approval_request_id: ApprovalRequestId::new(),
            gate_id: GateId::new(),
            step: 1,
            tool_call_id: ToolCallId::new(),
            tool_name: name.into(),
            arguments: serde_json::json!({}),
            args_hash: "sha256:test".into(),
            declared_effect: ToolEffect::Write,
            risk_level: openwand_core::RiskLevelSnapshot::Medium,
            confirmation_level: openwand_core::ConfirmationLevel::Approve,
            reason_code: "test".into(),
            policy_summary: "test".into(),
            requested_action_summary: "test".into(),
            rollback_plan: None,
            metadata: serde_json::Value::Null,
        },
        tool_name: name.into(),
        reason: "test".into(),
    }
}

fn empty_index() -> ApprovalRecoveryIndex {
    ApprovalRecoveryIndex {
        pending: vec![],
        resolved: vec![],
        deferred: vec![],
        uncertain: vec![],
        conflicts: vec![],
    }
}

fn index_with_pending(pendings: Vec<PendingApprovalRecovery>) -> ApprovalRecoveryIndex {
    ApprovalRecoveryIndex {
        pending: pendings,
        resolved: vec![],
        deferred: vec![],
        uncertain: vec![],
        conflicts: vec![],
    }
}

fn index_with_resolved(
    pendings: Vec<PendingApprovalRecovery>,
    resolved: Vec<ResolvedApprovalRecovery>,
) -> ApprovalRecoveryIndex {
    ApprovalRecoveryIndex {
        pending: pendings,
        resolved,
        deferred: vec![],
        uncertain: vec![],
        conflicts: vec![],
    }
}

// ---- Selector tests ----

#[test]
fn unscoped_single_pending_no_cache_returns_recovered() {
    let p = make_pending("local__write_A");
    let index = index_with_pending(vec![p.clone()]);
    let decision = ApprovalDecision::approve();

    let (target, source) = select_approval_target(&index, None, &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(ApprovalSource::Recovered, source);
}

#[test]
fn unscoped_single_pending_with_matching_cache_returns_live() {
    let p = make_pending("local__write_A");
    let arid = p.context.approval_request_id.clone();
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::approve();

    let (target, source) = select_approval_target(&index, Some(arid), &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(ApprovalSource::Live, source);
}

#[test]
fn unscoped_single_pending_with_stale_cache_returns_recovered() {
    let p = make_pending("local__write_A");
    let stale_arid = ApprovalRequestId::new(); // different arid
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::approve();

    let (target, source) = select_approval_target(&index, Some(stale_arid), &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(ApprovalSource::Recovered, source);
}

#[test]
fn unscoped_no_pending_returns_error() {
    let index = empty_index();
    let decision = ApprovalDecision::approve();

    let result = select_approval_target(&index, None, &decision);
    assert!(result.is_err());
}

#[test]
fn unscoped_multiple_pending_returns_error() {
    let p1 = make_pending("local__write_A");
    let p2 = make_pending("local__write_B");
    let index = index_with_pending(vec![p1, p2]);
    let decision = ApprovalDecision::approve();

    let result = select_approval_target(&index, None, &decision);
    assert!(result.is_err());
}

#[test]
fn explicit_arid_found_in_pending_returns_recovered() {
    let p = make_pending("local__write_A");
    let arid = p.context.approval_request_id.clone();
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::for_approval(arid.clone(), ApprovalResolution::Approve);

    let (target, source) = select_approval_target(&index, None, &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(arid, target.context.approval_request_id);
    assert_eq!(ApprovalSource::Recovered, source);
}

#[test]
fn explicit_arid_with_matching_cache_returns_live() {
    let p = make_pending("local__write_A");
    let arid = p.context.approval_request_id.clone();
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::for_approval(arid.clone(), ApprovalResolution::Approve);

    let (target, source) = select_approval_target(&index, Some(arid), &decision).unwrap();

    assert_eq!(ApprovalSource::Live, source);
}

#[test]
fn explicit_arid_with_different_cache_returns_stale_cache() {
    let p = make_pending("local__write_A");
    let arid = p.context.approval_request_id.clone();
    let cache_arid = ApprovalRequestId::new(); // different
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::for_approval(arid, ApprovalResolution::Approve);

    let (target, source) = select_approval_target(&index, Some(cache_arid), &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(ApprovalSource::StaleCache, source);
}

#[test]
fn explicit_arid_not_found_returns_error() {
    let p = make_pending("local__write_A");
    let index = index_with_pending(vec![p]);
    let wrong_arid = ApprovalRequestId::new();
    let decision = ApprovalDecision::for_approval(wrong_arid, ApprovalResolution::Approve);

    let result = select_approval_target(&index, None, &decision);
    assert!(result.is_err());
}

#[test]
fn explicit_arid_never_falls_back_to_single_pending() {
    // Safety test: explicit arid that doesn't exist should NOT resolve to the
    // single pending approval. It must fail.
    let p = make_pending("local__write_A");
    let index = index_with_pending(vec![p]);
    let wrong_arid = ApprovalRequestId::new();
    let decision = ApprovalDecision::for_approval(wrong_arid, ApprovalResolution::Approve);

    let result = select_approval_target(&index, None, &decision);
    assert!(result.is_err(), "Explicit arid must not fall back to unscoped resolution");
}

#[test]
fn reject_decision_works_same_as_approve_for_selection() {
    let p = make_pending("local__write_A");
    let arid = p.context.approval_request_id.clone();
    let index = index_with_pending(vec![p]);
    let decision = ApprovalDecision::for_approval(arid, ApprovalResolution::Reject { reason: Some("unsafe".into()) });

    let (target, source) = select_approval_target(&index, None, &decision).unwrap();

    assert_eq!("local__write_A", target.tool_name);
    assert_eq!(ApprovalSource::Recovered, source);
}

// ---- Index tests ----

#[test]
fn recovery_index_populates_resolved_from_resumed() {
    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_store::StoredEvent;
    use openwand_trace::entry::TraceEntry;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};

    let arid = ApprovalRequestId::new();
    let tc_id = ToolCallId::new();

    let entry = TraceEntry {
        id: TraceId::new(),
        stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "test".into() },
        stream_sequence: 0,
        global_sequence: 0,
        occurred_at: chrono::Utc::now(),
        actor: openwand_trace::actor::Actor::System { component: "test".into() },
        event: StoredEvent::from(OpenWandTraceEvent::Tool(ToolEvent::Resumed {
            tool_call_id: tc_id.clone(),
            tool_name: "local__write_A".into(),
            resolution: "approved".into(),
            approval_request_id: Some(arid.clone()),
        })),
        event_kind: "tool.resumed".into(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: None,
        entry_hash: EntryHash("test".into()),
    };

    let index = openwand_session::approval_recovery::build_recovery_index(&[entry]);

    assert_eq!(1, index.resolved.len());
    assert_eq!(arid, index.resolved[0].approval_request_id);
    assert_eq!(tc_id, index.resolved[0].tool_call_id);
    assert_eq!(ResolvedApprovalKind::Approved, index.resolved[0].kind);
}

#[test]
fn recovery_index_populates_resolved_from_denied() {
    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_store::StoredEvent;
    use openwand_trace::entry::TraceEntry;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};

    let arid = ApprovalRequestId::new();
    let tc_id = ToolCallId::new();

    let entry = TraceEntry {
        id: TraceId::new(),
        stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "test".into() },
        stream_sequence: 0,
        global_sequence: 0,
        occurred_at: chrono::Utc::now(),
        actor: openwand_trace::actor::Actor::System { component: "test".into() },
        event: StoredEvent::from(OpenWandTraceEvent::Tool(ToolEvent::Denied {
            tool_call_id: tc_id.clone(),
            tool_name: "local__write_A".into(),
            approval_request_id: Some(arid.clone()),
            reason: Some("unsafe".into()),
        })),
        event_kind: "tool.denied".into(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: None,
        entry_hash: EntryHash("test".into()),
    };

    let index = openwand_session::approval_recovery::build_recovery_index(&[entry]);

    assert_eq!(1, index.resolved.len());
    assert_eq!(ResolvedApprovalKind::Denied, index.resolved[0].kind);
}
