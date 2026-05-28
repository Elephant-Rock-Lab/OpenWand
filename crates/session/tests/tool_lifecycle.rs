//! Wave 03f: Tool lifecycle closure tests.
//!
//! Proves that every tool call reaching policy evaluation produces a complete,
//! correctly ordered lifecycle chain in trace.

use openwand_core::events::{GateEvent, OpenWandTraceEvent, ToolEvent};
use openwand_core::ids::ToolCallId;
use openwand_core::mode::InteractionMode;
use openwand_session::approval_recovery::{
    validate_tool_lifecycle, LifecycleValidationMode,
};
use openwand_session::config::RunConfig;
use openwand_session::testing::harness::SessionHarness;
use openwand_session::ApprovalDecision;
use openwand_store::StoredEvent;
use openwand_trace::entry::TraceEntry;
use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};

fn default_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Direct,
        ..Default::default()
    }
}

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

// ---- Lifecycle path tests ----

#[tokio::test]
async fn tool_lifecycle_allowed_success_is_closed() {
    let harness = SessionHarness::read_file_tool_turn();

    harness
        .runner
        .run_turn("Read README.md".into(), default_config())
        .await
        .unwrap();

    let kinds = harness.trace.event_kinds().await;

    // Must have: gate.evaluated → tool.called → tool.completed
    let gate_idx = kinds.iter().position(|k| k == "gate.evaluated").expect("gate.evaluated missing");
    let called_idx = kinds.iter().position(|k| k == "tool.called").expect("tool.called missing");
    let completed_idx = kinds.iter().position(|k| k == "tool.completed").expect("tool.completed missing");

    assert!(called_idx > gate_idx, "tool.called must follow gate.evaluated");
    assert!(completed_idx > called_idx, "tool.completed must follow tool.called");

    // No tool.failed or tool.denied
    assert!(!kinds.iter().any(|k| k == "tool.failed"), "no tool.failed for success");
    assert!(!kinds.iter().any(|k| k == "tool.denied"), "no tool.denied for success");
}

#[tokio::test]
async fn tool_lifecycle_allowed_failure_is_closed() {
    let harness = SessionHarness::tool_turn_with_tool_error("local__read_file", "tool not found");

    harness
        .runner
        .run_turn("Delete everything".into(), default_config())
        .await
        .unwrap();

    let kinds = harness.trace.event_kinds().await;

    // Must have: gate.evaluated → tool.called → tool.failed
    assert!(kinds.iter().any(|k| k == "gate.evaluated"), "gate.evaluated missing");
    assert!(kinds.iter().any(|k| k == "tool.called"), "tool.called missing");
    assert!(kinds.iter().any(|k| k == "tool.failed"), "tool.failed missing");
    assert!(!kinds.iter().any(|k| k == "tool.completed"), "no tool.completed for failure");
}

#[tokio::test]
async fn tool_blocked_records_denied_without_called() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    harness
        .runner
        .run_turn("Write a file".into(), default_config())
        .await
        .unwrap();

    let kinds = harness.trace.event_kinds().await;

    // Must have: gate.evaluated → tool.denied
    assert!(kinds.iter().any(|k| k == "gate.evaluated"), "gate.evaluated missing");
    assert!(kinds.iter().any(|k| k == "tool.denied"), "tool.denied missing");

    // No execution events
    assert!(!kinds.iter().any(|k| k == "tool.called"), "no tool.called for blocked");
    assert!(!kinds.iter().any(|k| k == "tool.completed"), "no tool.completed for blocked");
    assert!(!kinds.iter().any(|k| k == "tool.failed"), "no tool.failed for blocked");
}

#[tokio::test]
async fn tool_suspended_approval_records_closed_chain() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // Suspend
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Approve
    harness
        .runner
        .resolve_approval(ApprovalDecision::approve(), conversational_config())
        .await
        .unwrap();

    let kinds = harness.trace.event_kinds().await;

    // Must have: gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed
    let gate_idx = kinds.iter().position(|k| k == "gate.evaluated").unwrap();
    let suspended_idx = kinds.iter().position(|k| k == "tool.suspended").unwrap();
    let resumed_idx = kinds.iter().position(|k| k == "tool.resumed").unwrap();
    let called_idx = kinds.iter().position(|k| k == "tool.called").unwrap();
    let completed_idx = kinds.iter().position(|k| k == "tool.completed").unwrap();

    assert!(suspended_idx > gate_idx);
    assert!(resumed_idx > suspended_idx);
    assert!(called_idx > resumed_idx);
    assert!(completed_idx > called_idx);
}

#[tokio::test]
async fn tool_rejected_approval_records_denied_chain() {
    let harness = SessionHarness::write_tool_requires_confirmation();

    // Suspend
    harness
        .runner
        .run_turn("Write a file".into(), conversational_config())
        .await
        .unwrap();

    // Reject
    harness
        .runner
        .resolve_approval(ApprovalDecision::reject(), conversational_config())
        .await
        .unwrap();

    let kinds = harness.trace.event_kinds().await;

    // Must have: gate.evaluated → tool.suspended → tool.denied
    assert!(kinds.iter().any(|k| k == "gate.evaluated"));
    assert!(kinds.iter().any(|k| k == "tool.suspended"));
    assert!(kinds.iter().any(|k| k == "tool.denied"));

    // No execution events
    assert!(!kinds.iter().any(|k| k == "tool.called"), "no tool.called for rejection");
    assert!(!kinds.iter().any(|k| k == "tool.completed"), "no tool.completed for rejection");
    assert!(!kinds.iter().any(|k| k == "tool.failed"), "no tool.failed for rejection");
}

// ---- Scanner tests with synthetic malformed traces ----

fn make_entry(event: OpenWandTraceEvent, kind: &str) -> TraceEntry<StoredEvent> {
    TraceEntry {
        id: openwand_trace::TraceId::new(),
        stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "test".into() },
        stream_sequence: 0,
        global_sequence: 0,
        occurred_at: chrono::Utc::now(),
        actor: openwand_trace::actor::Actor::System { component: "test".into() },
        event: StoredEvent::from(event),
        event_kind: kind.into(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: None,
        entry_hash: EntryHash("test".into()),
    }
}

fn tc_id() -> ToolCallId {
    ToolCallId::new()
}

#[test]
fn scanner_allows_correct_success_lifecycle() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: true,
            risk_level: None,
            reason_code: Some("allowed".into()),
            summary: "allowed".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: id.clone(),
            tool_name: "local__read".into(),
            args_hash: "sha256:abc".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        }), "tool.called"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Completed {
            tool_call_id: id,
            tool_name: "local__read".into(),
            status: openwand_core::tool_vocab::ToolResultStatus::Success,
            result_summary: "ok".into(),
            duration_ms: 10,
        }), "tool.completed"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::RequireClosedLifecycle);
    assert!(violations.is_empty(), "Expected no violations, got: {:?}", violations);
}

#[test]
fn scanner_detects_missing_called() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: true,
            risk_level: None,
            reason_code: Some("allowed".into()),
            summary: "allowed".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Completed {
            tool_call_id: id,
            tool_name: "local__read".into(),
            status: openwand_core::tool_vocab::ToolResultStatus::Success,
            result_summary: "ok".into(),
            duration_ms: 10,
        }), "tool.completed"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::RequireClosedLifecycle);
    assert!(!violations.is_empty(), "Should detect missing tool.called");
    assert!(violations[0].reason.contains("without tool.called"));
}

#[test]
fn scanner_detects_missing_terminal() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: true,
            risk_level: None,
            reason_code: Some("allowed".into()),
            summary: "allowed".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: id,
            tool_name: "local__read".into(),
            args_hash: "sha256:abc".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        }), "tool.called"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::RequireClosedLifecycle);
    assert!(!violations.is_empty(), "Should detect missing terminal");
    assert!(violations[0].reason.contains("no terminal event"));
}

#[test]
fn scanner_detects_duplicate_terminal() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: true,
            risk_level: None,
            reason_code: Some("allowed".into()),
            summary: "allowed".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Called {
            tool_call_id: id.clone(),
            tool_name: "local__read".into(),
            args_hash: "sha256:abc".into(),
            invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
        }), "tool.called"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Completed {
            tool_call_id: id.clone(),
            tool_name: "local__read".into(),
            status: openwand_core::tool_vocab::ToolResultStatus::Success,
            result_summary: "ok".into(),
            duration_ms: 10,
        }), "tool.completed"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Failed {
            tool_call_id: id,
            tool_name: "local__read".into(),
            error: "oops".into(),
        }), "tool.failed"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::RequireClosedLifecycle);
    assert!(!violations.is_empty(), "Should detect duplicate terminal");
    assert!(violations[0].reason.contains("completed") && violations[0].reason.contains("failed"));
}

#[test]
fn scanner_allow_open_pending_mode_accepts_suspended() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: false,
            risk_level: None,
            reason_code: Some("require_confirmation".into()),
            summary: "needs approval".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: id,
            tool_name: "local__write".into(),
            reason: "awaiting approval".into(),
            approval_context: None,
        }), "tool.suspended"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::AllowOpenPendingApprovals);
    assert!(violations.is_empty(), "Open pending should be allowed, got: {:?}", violations);
}

#[test]
fn scanner_require_closed_mode_rejects_suspended() {
    let id = tc_id();
    let entries = vec![
        make_entry(OpenWandTraceEvent::Gate(GateEvent::Evaluated {
            gate_id: id.to_string(),
            gate_kind: "policy".into(),
            passed: false,
            risk_level: None,
            reason_code: Some("require_confirmation".into()),
            summary: "needs approval".into(),
        }), "gate.evaluated"),
        make_entry(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: id,
            tool_name: "local__write".into(),
            reason: "awaiting approval".into(),
            approval_context: None,
        }), "tool.suspended"),
    ];

    let violations = validate_tool_lifecycle(&entries, LifecycleValidationMode::RequireClosedLifecycle);
    assert!(!violations.is_empty(), "RequireClosed should flag open pending");
    assert!(violations[0].reason.contains("no terminal event"));
}
