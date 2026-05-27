//! Approval-state rebuild from trace.
//!
//! Reconstructs Loro approval state (waiting_approval, recovery_blocked)
//! from SQLite trace after a process restart.
//!
//! Scope is minimal: only approval-related state, not full Loro message parity.

use crate::approval_recovery::{build_recovery_index, ApprovalRecoveryIndex};
use crate::loro_state::LoroSessionState;
use crate::SessionError;
use openwand_store::StoredEvent;
use openwand_trace::TraceStore;

/// Rebuild approval state in Loro from trace.
///
/// This scans the session's trace stream, builds a recovery index,
/// and applies the appropriate state to Loro:
/// - Single pending approval → set waiting_approval
/// - Conflicts / multiple pending / uncertain → set recovery_blocked
/// - No pending → clear both states
pub async fn rebuild_approval_state(
    trace: &dyn TraceStore<StoredEvent>,
    loro: &LoroSessionState,
) -> Result<ApprovalRecoveryIndex, SessionError> {
    use openwand_trace::TraceQuery;

    let page = trace.scan(TraceQuery::default()).await.map_err(SessionError::Trace)?;

    let index = build_recovery_index(&page.entries);

    apply_recovery_state(&index, loro)?;

    Ok(index)
}

/// Apply recovery state to Loro from a pre-built index.
pub fn apply_recovery_state(
    index: &ApprovalRecoveryIndex,
    loro: &LoroSessionState,
) -> Result<(), SessionError> {
    // Clear both states first
    let _ = loro.clear_waiting_approval();
    let _ = loro.clear_recovery_blocked();

    if index.is_recovery_blocked() {
        let reason = format_recovery_blocked_reason(index);
        loro.set_recovery_blocked(&reason, index.pending.len() + index.uncertain.len())
            .map_err(SessionError::Internal)?;
    } else if index.has_single_pending_approval() {
        let pending = &index.pending[0];
        loro.set_waiting_approval(&pending.context, &pending.reason)
            .map_err(SessionError::Internal)?;
    }
    // else: no pending, both states cleared

    Ok(())
}

fn format_recovery_blocked_reason(index: &ApprovalRecoveryIndex) -> String {
    let mut parts = Vec::new();

    if !index.conflicts.is_empty() {
        parts.push(format!("{} conflict(s)", index.conflicts.len()));
    }
    if index.pending.len() > 1 {
        parts.push(format!("{} unresolved approvals", index.pending.len()));
    }
    if !index.uncertain.is_empty() {
        parts.push(format!("{} uncertain execution(s)", index.uncertain.len()));
    }

    if parts.is_empty() {
        "unknown recovery issue".into()
    } else {
        format!("Recovery blocked: {}", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_core::ids::{ApprovalRequestId, GateId, ToolCallId};
    use openwand_core::snapshots::ApprovalContextSnapshot;
    use openwand_store::StoredEvent;
    use openwand_trace::entry::TraceEntry;
    use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};

    fn make_entry(event: OpenWandTraceEvent) -> TraceEntry<StoredEvent> {
        TraceEntry {
            id: openwand_trace::TraceId::new(),
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

    fn fresh_loro() -> LoroSessionState {
        let doc = loro::LoroDoc::new();
        LoroSessionState::new(&doc)
    }

    #[test]
    fn loro_set_and_get_waiting_approval() {
        let loro = fresh_loro();
        let ctx = make_context();
        let expected_name = ctx.tool_name.clone();

        loro.set_waiting_approval(&ctx, "test reason").unwrap();

        let result = loro.get_waiting_approval().unwrap();
        assert!(result.is_some());
        let ui = result.unwrap();
        assert_eq!(expected_name, ui.tool_name);
    }

    #[test]
    fn loro_clear_waiting_approval() {
        let loro = fresh_loro();
        let ctx = make_context();

        loro.set_waiting_approval(&ctx, "test").unwrap();
        assert!(loro.get_waiting_approval().unwrap().is_some());

        loro.clear_waiting_approval().unwrap();
        assert!(loro.get_waiting_approval().unwrap().is_none());
    }

    #[test]
    fn loro_set_recovery_blocked() {
        let loro = fresh_loro();

        assert!(!loro.is_recovery_blocked().unwrap());

        loro.set_recovery_blocked("test conflict", 2).unwrap();
        assert!(loro.is_recovery_blocked().unwrap());

        loro.clear_recovery_blocked().unwrap();
        assert!(!loro.is_recovery_blocked().unwrap());
    }

    #[test]
    fn rebuild_pending_suspension_sets_waiting_approval() {
        let loro = fresh_loro();
        let ctx = make_context();
        let tc_id = ctx.tool_call_id.clone();

        let entries = vec![make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: tc_id,
            tool_name: "local__file_write".into(),
            reason: "awaiting approval".into(),
            approval_context: Some(ctx),
        }))];

        let index = build_recovery_index(&entries);
        apply_recovery_state(&index, &loro).unwrap();

        assert!(!loro.is_recovery_blocked().unwrap());
        assert!(loro.get_waiting_approval().unwrap().is_some());
    }

    #[test]
    fn rebuild_cleared_after_resumed() {
        let loro = fresh_loro();
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
                tool_call_id: tc_id,
                tool_name: "local__file_write".into(),
                resolution: "approved".into(),
                approval_request_id: Some(ar_id),
            })),
        ];

        let index = build_recovery_index(&entries);
        apply_recovery_state(&index, &loro).unwrap();

        assert!(!loro.is_recovery_blocked().unwrap());
        assert!(loro.get_waiting_approval().unwrap().is_none());
    }

    #[test]
    fn rebuild_cleared_after_denied() {
        let loro = fresh_loro();
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
        apply_recovery_state(&index, &loro).unwrap();

        assert!(!loro.is_recovery_blocked().unwrap());
        assert!(loro.get_waiting_approval().unwrap().is_none());
    }

    #[test]
    fn rebuild_multiple_unresolved_sets_recovery_blocked() {
        let loro = fresh_loro();
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
        apply_recovery_state(&index, &loro).unwrap();

        assert!(loro.is_recovery_blocked().unwrap());
        // No waiting approval when blocked
        assert!(loro.get_waiting_approval().unwrap().is_none());
    }

    #[test]
    fn rebuild_old_suspended_without_context_sets_recovery_blocked() {
        let loro = fresh_loro();
        let tc_id = ToolCallId::new();

        // Pre-03d event: no approval_context
        let entries = vec![make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: tc_id,
            tool_name: "local__file_write".into(),
            reason: "awaiting approval".into(),
            approval_context: None,
        }))];

        let index = build_recovery_index(&entries);
        apply_recovery_state(&index, &loro).unwrap();

        assert!(loro.is_recovery_blocked().unwrap());
        assert!(loro.get_waiting_approval().unwrap().is_none());
    }
}
