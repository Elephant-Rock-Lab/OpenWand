//! Generic, read-only operation replay verifier.
//!
//! Wave 93A: Checks whether configured event-kind expectations appear in
//! the expected order within trace entries. The verifier is generic — it
//! knows nothing about desktop operations, workflow domains, or approval
//! semantics. App-level code defines expectations using event_kind strings.
//!
//! Non-negotiable invariant: The replay verifier reads trace entries. It
//! does not execute, repair, append, approve, export, dispatch, or mutate.
//! It does not claim more integrity than event-order correspondence supports.

use crate::entry::TraceEntry;
use crate::stream::TraceStreamScope;

// ── Report Types ───────────────────────────────────────────

/// Overall replay verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayResult {
    /// Every expectation with an observed request has the expected governed
    /// event after it.
    Pass,
    /// At least one observed request lacks a governed event, or ordering
    /// is invalid.
    Fail,
    /// No request events were observed for the expectation set.
    Inconclusive,
}

/// Severity of an individual replay finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingSeverity {
    /// Informational note (e.g., "no request observed").
    Info,
    /// Suggests a problem but does not alone prove failure.
    Warning,
    /// Proves replay failure for this expectation.
    Error,
}

/// A single replay finding.
#[derive(Debug, Clone)]
pub struct ReplayFinding {
    pub severity: FindingSeverity,
    pub operation_id: String,
    pub detail: String,
    pub entry_id: Option<String>,
}

/// Full replay report.
#[derive(Debug, Clone)]
pub struct ReplayReport {
    pub result: ReplayResult,
    pub findings: Vec<ReplayFinding>,
    pub operations_checked: usize,
    pub events_scanned: usize,
}

impl ReplayReport {
    /// Convenience: were there any error-severity findings?
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == FindingSeverity::Error)
    }
}

// ── Expectation ────────────────────────────────────────────

/// A single operation correspondence expectation.
///
/// The verifier will search trace entries for a `request_event_kind`
/// followed by a `governed_event_kind` in the correct order.
///
/// This is generic — it contains no domain vocabulary. App-level code
/// constructs expectations using event_kind strings from the domain.
#[derive(Debug, Clone)]
pub struct OperationExpectation {
    /// Identifier for this expectation (e.g., "workflow_run_initiation").
    pub operation_id: String,

    /// Event kind that represents the request/initiation.
    pub request_event_kind: String,

    /// Event kind that represents the governed response.
    pub governed_event_kind: String,

    /// If set, only entries in this stream scope are considered.
    pub stream_scope: Option<TraceStreamScope>,

    /// If set, only entries in this exact stream id are considered.
    /// Requires `stream_scope` to also match if set.
    pub stream_id: Option<String>,
}

// ── Verifier ───────────────────────────────────────────────

/// Read-only operation replay verifier.
///
/// Checks event-kind correspondence and ordering. Does NOT recompute hashes,
/// execute operations, resolve approvals, export evidence, or mutate state.
///
/// Result semantics:
/// - Pass: Every expectation with an observed request has a governed event
///   after it.
/// - Fail: At least one observed request lacks a governed event, or ordering
///   is invalid.
/// - Inconclusive: No request events were observed for the expectation set.
///
/// For mixed expectations:
/// - Any Fail => report Fail
/// - No Fail + at least one Pass => report Pass
/// - No Fail + no Pass => report Inconclusive
pub struct OperationReplayVerifier;

impl OperationReplayVerifier {
    /// Verify operation expectations against a slice of trace entries.
    ///
    /// Entries are sorted by `global_sequence` internally for deterministic
    /// processing. The input slice is NOT mutated.
    ///
    /// For an empty trace, returns Inconclusive with 0 operations checked.
    pub fn replay<E>(
        entries: &[TraceEntry<E>],
        expectations: &[OperationExpectation],
    ) -> ReplayReport {
        let mut findings = Vec::new();
        let mut has_pass = false;
        let mut has_fail = false;

        // Validate expectations
        for exp in expectations {
            if exp.request_event_kind.is_empty() || exp.governed_event_kind.is_empty() {
                findings.push(ReplayFinding {
                    severity: FindingSeverity::Error,
                    operation_id: exp.operation_id.clone(),
                    detail: "expectation has empty request or governed event kind".into(),
                    entry_id: None,
                });
                has_fail = true;
            }
        }

        if entries.is_empty() && !has_fail {
            for exp in expectations {
                findings.push(ReplayFinding {
                    severity: FindingSeverity::Info,
                    operation_id: exp.operation_id.clone(),
                    detail: "no request events observed (empty trace)".into(),
                    entry_id: None,
                });
            }
            return ReplayReport {
                result: ReplayResult::Inconclusive,
                findings,
                operations_checked: expectations.len(),
                events_scanned: 0,
            };
        }

        // Sort entries by global_sequence for deterministic processing
        let mut sorted: Vec<&TraceEntry<E>> = entries.iter().collect();
        sorted.sort_by_key(|e| e.global_sequence);

        for exp in expectations {
            // Skip already-failed (malformed) expectations
            if exp.request_event_kind.is_empty() || exp.governed_event_kind.is_empty() {
                continue;
            }

            let result = Self::check_single_expectation(&sorted, exp, &mut findings);

            match result {
                ExpectationOutcome::Pass => has_pass = true,
                ExpectationOutcome::Fail => has_fail = true,
                ExpectationOutcome::Inconclusive => {}
            }
        }

        let result = if has_fail {
            ReplayResult::Fail
        } else if has_pass {
            ReplayResult::Pass
        } else {
            ReplayResult::Inconclusive
        };

        ReplayReport {
            result,
            findings,
            operations_checked: expectations.len(),
            events_scanned: entries.len(),
        }
    }

    /// Check a single expectation against sorted trace entries.
    fn check_single_expectation<E>(
        sorted: &[&TraceEntry<E>],
        exp: &OperationExpectation,
        findings: &mut Vec<ReplayFinding>,
    ) -> ExpectationOutcome {
        // Filter by stream_scope and stream_id if provided
        let filtered: Vec<&&TraceEntry<E>> = sorted
            .iter()
            .filter(|e| {
                // Check stream_scope
                if let Some(scope) = &exp.stream_scope
                    && &e.stream_id.scope != scope
                {
                    return false;
                }
                // Check stream_id
                if let Some(id) = &exp.stream_id
                    && e.stream_id.id != *id
                {
                    return false;
                }
                true
            })
            .collect();

        // Step 1: Find the earliest request event
        let request_entry = filtered
            .iter()
            .find(|e| e.event_kind == exp.request_event_kind);

        let request = match request_entry {
            None => {
                findings.push(ReplayFinding {
                    severity: FindingSeverity::Info,
                    operation_id: exp.operation_id.clone(),
                    detail: format!(
                        "no request event '{}' observed",
                        exp.request_event_kind
                    ),
                    entry_id: None,
                });
                return ExpectationOutcome::Inconclusive;
            }
            Some(entry) => entry,
        };

        let request_seq = request.global_sequence;

        // Step 2: Find the first governed event AFTER the request
        let governed_entry = filtered
            .iter()
            .find(|e| {
                e.event_kind == exp.governed_event_kind
                    && e.global_sequence > request_seq
            });

        match governed_entry {
            Some(governed) => {
                findings.push(ReplayFinding {
                    severity: FindingSeverity::Info,
                    operation_id: exp.operation_id.clone(),
                    detail: format!(
                        "request at seq {} followed by governed event '{}' at seq {}",
                        request_seq, exp.governed_event_kind, governed.global_sequence
                    ),
                    entry_id: Some(governed.id.0.to_string()),
                });
                ExpectationOutcome::Pass
            }
            None => {
                // Check if there's a governed event BEFORE the request (wrong order)
                let governed_before = filtered.iter().any(|e| {
                    e.event_kind == exp.governed_event_kind
                        && e.global_sequence <= request_seq
                });

                if governed_before {
                    findings.push(ReplayFinding {
                        severity: FindingSeverity::Error,
                        operation_id: exp.operation_id.clone(),
                        detail: format!(
                            "governed event '{}' exists but only before request at seq {}",
                            exp.governed_event_kind, request_seq
                        ),
                        entry_id: Some(request.id.0.to_string()),
                    });
                } else {
                    findings.push(ReplayFinding {
                        severity: FindingSeverity::Error,
                        operation_id: exp.operation_id.clone(),
                        detail: format!(
                            "request at seq {} has no governed event '{}' after it",
                            request_seq, exp.governed_event_kind
                        ),
                        entry_id: Some(request.id.0.to_string()),
                    });
                }
                ExpectationOutcome::Fail
            }
        }
    }
}

// ── Internal ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectationOutcome {
    Pass,
    Fail,
    Inconclusive,
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Actor;
    use crate::entry::TraceEntry;
    use crate::ids::TraceId;
    use crate::stream::{EntryHash, TraceStreamId, TraceStreamScope};

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
            stream_id: TraceStreamId {
                scope,
                id: stream_id.into(),
            },
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

    fn workflow_expectation() -> OperationExpectation {
        OperationExpectation {
            operation_id: "workflow_run_initiation".into(),
            request_event_kind: "workflow.gate_passed".into(),
            governed_event_kind: "workflow.action_executed".into(),
            stream_scope: Some(TraceStreamScope::Workflow),
            stream_id: None,
        }
    }

    fn approval_expectation() -> OperationExpectation {
        OperationExpectation {
            operation_id: "approval_resolution".into(),
            request_event_kind: "tool.suspended".into(),
            governed_event_kind: "tool.resumed".into(),
            stream_scope: Some(TraceStreamScope::Session),
            stream_id: None,
        }
    }

    fn export_expectation() -> OperationExpectation {
        OperationExpectation {
            operation_id: "evidence_export".into(),
            request_event_kind: "evidence.export_requested".into(),
            governed_event_kind: "evidence.export_completed".into(),
            stream_scope: None,
            stream_id: None,
        }
    }

    // ── Test 2: Empty trace → Inconclusive ──

    #[test]
    fn empty_trace_is_inconclusive() {
        let report = OperationReplayVerifier::replay::<TestEvent>(&[], &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Inconclusive);
        assert_eq!(report.operations_checked, 1);
        assert_eq!(report.events_scanned, 0);
    }

    // ── Test 3: Workflow request + governed in order → Pass ──

    #[test]
    fn workflow_request_then_governed_passes() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Pass);
        assert_eq!(report.operations_checked, 1);
    }

    // ── Test 4: Workflow request without governed → Fail ──

    #[test]
    fn workflow_request_without_governed_fails() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "session.step_started", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.has_errors());
    }

    // ── Test 5: Workflow governed without request → Inconclusive ──

    #[test]
    fn workflow_governed_without_request_is_inconclusive() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.action_executed", "h1"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Inconclusive);
    }

    // ── Test 6: Workflow governed before request → Fail ──

    #[test]
    fn workflow_governed_before_request_fails() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.action_executed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.gate_passed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.findings.iter().any(|f| f.severity == FindingSeverity::Error));
    }

    // ── Test 7: Approval request + resolved in order → Pass ──

    #[test]
    fn approval_request_then_resolved_passes() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, "tool.resumed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[approval_expectation()]);
        assert_eq!(report.result, ReplayResult::Pass);
    }

    // ── Test 8: Approval request without resolution → Fail ──

    #[test]
    fn approval_request_without_resolution_fails() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, "session.step_started", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[approval_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
    }

    // ── Test 9: Approval wrong order → Fail ──

    #[test]
    fn approval_wrong_order_fails() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "tool.resumed", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, "tool.suspended", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[approval_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.detail.contains("before request") && f.severity == FindingSeverity::Error
        ));
    }

    // ── Test 10: Evidence export request + completion in order → Pass ──

    #[test]
    fn evidence_export_request_then_completion_passes() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Global, "g1", 1, "evidence.export_requested", "h1"),
            make_entry(2, TraceStreamScope::Global, "g1", 2, "evidence.export_completed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[export_expectation()]);
        assert_eq!(report.result, ReplayResult::Pass);
    }

    // ── Test 11: Evidence export request without completion → Fail ──

    #[test]
    fn evidence_export_without_completion_fails() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Global, "g1", 1, "evidence.export_requested", "h1"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[export_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
    }

    // ── Test 12: Multiple operations all pass → Pass ──

    #[test]
    fn multiple_operations_all_pass() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h3"),
            make_entry(4, TraceStreamScope::Session, "s1", 2, "tool.resumed", "h4"),
        ];
        let exps = vec![workflow_expectation(), approval_expectation()];
        let report = OperationReplayVerifier::replay(&entries, &exps);
        assert_eq!(report.result, ReplayResult::Pass);
        assert_eq!(report.operations_checked, 2);
    }

    // ── Test 13: One operation fails, others pass → Fail ──

    #[test]
    fn one_operation_fails_report_is_fail() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
            make_entry(3, TraceStreamScope::Session, "s1", 1, "tool.suspended", "h3"),
            // Missing: tool.resumed
        ];
        let exps = vec![workflow_expectation(), approval_expectation()];
        let report = OperationReplayVerifier::replay(&entries, &exps);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.has_errors());
    }

    // ── Test 14: Report counts accurate ──

    #[test]
    fn report_counts_are_accurate() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 1, "irrelevant", "h2"),
            make_entry(3, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h3"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.operations_checked, 1);
        assert_eq!(report.events_scanned, 3);
    }

    // ── Test 15: Replay does not mutate entries ──

    #[test]
    fn replay_does_not_mutate_entries() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
        ];
        let original_hashes: Vec<String> = entries.iter().map(|e| e.entry_hash.0.clone()).collect();
        let original_kinds: Vec<String> = entries.iter().map(|e| e.event_kind.clone()).collect();

        let _report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);

        let after_hashes: Vec<String> = entries.iter().map(|e| e.entry_hash.0.clone()).collect();
        let after_kinds: Vec<String> = entries.iter().map(|e| e.event_kind.clone()).collect();

        assert_eq!(original_hashes, after_hashes, "hashes must not change");
        assert_eq!(original_kinds, after_kinds, "event kinds must not change");
    }

    // ── Test 16: Empty/malformed expectation → Fail with Error ──

    #[test]
    fn malformed_expectation_returns_structured_failure() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
        ];
        let bad_exp = OperationExpectation {
            operation_id: "bad_op".into(),
            request_event_kind: "".into(),
            governed_event_kind: "workflow.action_executed".into(),
            stream_scope: None,
            stream_id: None,
        };
        let report = OperationReplayVerifier::replay(&entries, &[bad_exp]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.severity == FindingSeverity::Error
            && f.operation_id == "bad_op"
            && f.detail.contains("empty")
        ));
    }

    // ── Test 17: Stream-scope filter ignores matching event in wrong scope ──

    #[test]
    fn stream_scope_filter_ignores_wrong_scope() {
        // Request is in Workflow scope, governed is in Session scope
        // Expectation filters to Workflow scope only → should Fail
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 1, "workflow.action_executed", "h2"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Fail);
        assert!(report.findings.iter().any(|f|
            f.severity == FindingSeverity::Error
            && f.detail.contains("no governed event")
        ));
    }

    // ── Test 18: Multiple candidate governed events chooses first after request ──

    #[test]
    fn multiple_governed_events_chooses_first_after_request() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf1", 2, "workflow.action_executed", "h2"),
            make_entry(3, TraceStreamScope::Workflow, "wf1", 3, "workflow.action_executed", "h3"),
        ];
        let report = OperationReplayVerifier::replay(&entries, &[workflow_expectation()]);
        assert_eq!(report.result, ReplayResult::Pass);
        // Should reference the FIRST governed event (seq 2, not seq 3)
        let pass_finding = report.findings.iter()
            .find(|f| f.severity == FindingSeverity::Info && f.detail.contains("seq 2"))
            .expect("should find first governed at seq 2");
        assert!(pass_finding.detail.contains("seq 2"));
    }

    // ── Extra: stream_id filtering ──

    #[test]
    fn stream_id_filter_narrows_to_exact_stream() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Workflow, "wf1", 1, "workflow.gate_passed", "h1"),
            make_entry(2, TraceStreamScope::Workflow, "wf2", 1, "workflow.action_executed", "h2"),
        ];
        let exp = OperationExpectation {
            operation_id: "workflow_run_initiation".into(),
            request_event_kind: "workflow.gate_passed".into(),
            governed_event_kind: "workflow.action_executed".into(),
            stream_scope: Some(TraceStreamScope::Workflow),
            stream_id: Some("wf1".into()),
        };
        let report = OperationReplayVerifier::replay(&entries, &[exp]);
        // Request in wf1, governed only in wf2 → Fail (filtered to wf1)
        assert_eq!(report.result, ReplayResult::Fail);
    }

    // ── Extra: Inconclusive when all expectations have no requests ──

    #[test]
    fn all_inconclusive_yields_inconclusive() {
        let entries = vec![
            make_entry(1, TraceStreamScope::Session, "s1", 1, "session.started", "h1"),
            make_entry(2, TraceStreamScope::Session, "s1", 2, "session.ended", "h2"),
        ];
        let exps = vec![workflow_expectation(), approval_expectation()];
        let report = OperationReplayVerifier::replay(&entries, &exps);
        assert_eq!(report.result, ReplayResult::Inconclusive);
        // No Pass, no Fail, all Inconclusive
    }
}
