//! Integration tests for operation replay verifier (Wave 93A).
//!
//! Proves:
//! 1. The replay verifier is read-only (source-level guard).
//! 2. Desktop operation expectations produce correct results.
//! 3. App-level expectations match existing event_kind constants.
//! 4. The verifier does not mutate entries.
//! 5. Report wording does not overclaim.

use openwand_trace::replay::{
    OperationExpectation, OperationReplayVerifier, ReplayResult,
    FindingSeverity,
};
use openwand_trace::entry::TraceEntry;
use openwand_trace::stream::{EntryHash, TraceStreamId, TraceStreamScope};
use openwand_trace::actor::Actor;
use openwand_trace::ids::TraceId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestEvent(String);

fn make_entry(
    global_seq: u64,
    scope: TraceStreamScope,
    stream_id: &str,
    stream_seq: u64,
    event_kind: &str,
    entry_hash: &str,
) -> TraceEntry<TestEvent> {
    TraceEntry {
        id: TraceId::new(),
        stream_id: TraceStreamId { scope, id: stream_id.into() },
        stream_sequence: stream_seq,
        global_sequence: global_seq,
        occurred_at: chrono::Utc::now(),
        actor: Actor::User,
        event: TestEvent(event_kind.into()),
        event_kind: event_kind.into(),
        event_schema_version: 1,
        trace_schema_version: 1,
        prev_hash: if stream_seq > 1 {
            Some(EntryHash(format!("prev_{}", stream_seq - 1)))
        } else {
            None
        },
        entry_hash: EntryHash(entry_hash.into()),
    }
}

// ── App-level expectation presets ──

fn workflow_run_initiation_expectation() -> OperationExpectation {
    OperationExpectation {
        operation_id: "workflow_run_initiation".into(),
        request_event_kind: "workflow.gate_passed".into(),
        governed_event_kind: "workflow.action_executed".into(),
        stream_scope: Some(TraceStreamScope::Workflow),
        stream_id: None,
    }
}

fn approval_resolution_expectation() -> OperationExpectation {
    OperationExpectation {
        operation_id: "approval_resolution".into(),
        request_event_kind: "tool.suspended".into(),
        governed_event_kind: "tool.resumed".into(),
        stream_scope: Some(TraceStreamScope::Session),
        stream_id: None,
    }
}

fn evidence_export_expectation() -> OperationExpectation {
    OperationExpectation {
        operation_id: "evidence_export".into(),
        request_event_kind: "evidence.export_requested".into(),
        governed_event_kind: "evidence.export_completed".into(),
        stream_scope: None,
        stream_id: None,
    }
}

// ── Guard tests ──

#[cfg(test)]
mod authority_guards {
    use super::*;

    /// Guard: replay module does not mutate, repair, append, or execute.
    #[test]
    fn replay_verifier_is_read_only() {
        let src = include_str!("../../trace/src/replay.rs");
        // Build search strings dynamically to avoid self-matching
        let s = String::from("append");
        let lp = "(";
        let append_pat = format!(".{s}{lp}");
        assert!(!src.contains(&append_pat), "verifier must not append trace");
        assert!(!src.contains("fn repair"), "verifier must not repair");
        assert!(!src.contains("fn execute_tool"), "must not execute tools");
        assert!(!src.contains("fn approve"), "must not approve");
        assert!(!src.contains("fn export_evidence"), "must not export evidence");
        assert!(!src.contains("fn dispatch"), "must not dispatch");
        assert!(!src.contains("std::fs::write"), "must not write files");
    }

    /// Guard: replay module exports the correct public API.
    #[test]
    fn replay_public_api_is_correct() {
        let src = include_str!("../../trace/src/replay.rs");
        assert!(src.contains("pub struct OperationReplayVerifier"));
        assert!(src.contains("pub struct OperationExpectation"));
        assert!(src.contains("pub struct ReplayReport"));
        assert!(src.contains("pub enum ReplayResult"));
        // Must NOT export Unsupported
        assert!(!src.contains("Unsupported"));
    }

    /// Guard: report wording does not overclaim.
    #[test]
    fn replay_wording_does_not_overclaim() {
        let src = include_str!("../../trace/src/replay.rs");
        let doc_section = src.split("#[cfg(test)]").next().unwrap_or("");

        let bad_phrases = [
            "operation was authorized",
            "operation executed correctly",
            "evidence is valid",
            "workflow was safe",
            "proven correct",
            "cryptographic",
        ];
        for phrase in &bad_phrases {
            assert!(
                !doc_section.contains(phrase),
                "module doc must not overclaim: '{}'",
                phrase
            );
        }
    }
}

// ── Desktop operation correspondence tests ──

#[cfg(test)]
mod desktop_operation_tests {
    use super::*;

    #[test]
    fn full_desktop_lifecycle_all_operations_pass() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h3"),
            make_entry(4, TraceStreamScope::Session, "s1", 2, "tool.resumed", "h4"),
            make_entry(5, TraceStreamScope::Global, "g1", 1, "evidence.export_requested", "h5"),
            make_entry(6, TraceStreamScope::Global, "g1", 2, "evidence.export_completed", "h6"),
        ];
        let exps = vec![
            workflow_run_initiation_expectation(),
            approval_resolution_expectation(),
            evidence_export_expectation(),
        ];
        let report = OperationReplayVerifier::replay(&entries, &exps);
        assert_eq!(report.result, ReplayResult::Pass);
        assert_eq!(report.operations_checked, 3);
        assert_eq!(report.events_scanned, 6);
    }

    #[test]
    fn partial_lifecycle_single_operation_pass() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[
            workflow_run_initiation_expectation(),
        ]);
        assert_eq!(report.result, ReplayResult::Pass);
    }

    #[test]
    fn approval_request_observed_but_not_resolved() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, "session.step_started", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[
            approval_resolution_expectation(),
        ]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.has_errors());
    }

    #[test]
    fn mixed_pass_and_fail_reports_fail() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h3"),
            // Missing: tool.resumed
        ];
        let exps = vec![
            workflow_run_initiation_expectation(),
            approval_resolution_expectation(),
        ];
        let report = OperationReplayVerifier::replay(&entries, &exps);
        assert_eq!(report.result, ReplayResult::Fail);
    }

    #[test]
    fn ordering_violation_governed_before_request() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.action_executed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.gate_passed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[
            workflow_run_initiation_expectation(),
        ]);
        assert_eq!(report.result, ReplayResult::Fail);
        let finding = report.findings.iter()
            .find(|f| f.severity == FindingSeverity::Error)
            .expect("should have error finding");
        assert!(finding.detail.contains("before request"));
    }

    #[test]
    fn stream_scope_filter_prevents_cross_scope_match() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 1, "workflow.action_executed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[
            workflow_run_initiation_expectation(),
        ]);
        assert_eq!(report.result, ReplayResult::Inconclusive);
    }

    #[test]
    fn replay_preserves_entry_integrity() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
        ];
        let original_kinds: Vec<_> = entries.iter().map(|e| e.event_kind.clone()).collect();
        let original_hashes: Vec<_> = entries.iter().map(|e| e.entry_hash.0.clone()).collect();

        let _ = OperationReplayVerifier::replay(&entries, &[
            workflow_run_initiation_expectation(),
        ]);

        assert_eq!(
            entries.iter().map(|e| e.event_kind.clone()).collect::<Vec<_>>(),
            original_kinds,
        );
        assert_eq!(
            entries.iter().map(|e| e.entry_hash.0.clone()).collect::<Vec<_>>(),
            original_hashes,
        );
    }
}
