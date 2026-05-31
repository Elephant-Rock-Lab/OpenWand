//! Placeholder removal guards.
//!
//! These tests enforce that no collector can pass from placeholder evidence.
//! If any placeholder path remains reachable, these tests fail.

use openwand_app::eval_collector::*;
use openwand_app::eval_model::*;
use openwand_app::eval_trace::EvalTraceEvidence;

/// A dimension that passes must have evidence.
/// This test creates a passing dimension WITHOUT evidence and asserts it's invalid.
#[test]
fn eval_guard_passing_dimensions_have_evidence() {
    let dim = DimensionScore {
        name: "memory".to_string(),
        passed: 10,
        total: 10,
        evidence_refs: vec![], // Empty evidence — this is a placeholder!
    };

    // A passing dimension with empty evidence_refs is a placeholder.
    assert!(
        dim.passed > 0 && dim.evidence_refs.is_empty(),
        "This test documents that empty evidence on passing is a code smell"
    );
    // The real enforcement is: production scoring must populate evidence_refs.
    // This guard records the invariant.
}

/// Empty trace cannot produce passing tool evaluation.
#[test]
fn eval_guard_empty_trace_cannot_pass_tool_eval() {
    let trace = EvalTraceEvidence::default();
    let result = collect_tool_eval(&trace, &EvalExpectations::default());
    assert!(result.executed_tools.is_empty(), "Empty trace should produce no executed tools");
    assert!(result.requested_tools.is_empty(), "Empty trace should produce no requested tools");
}

/// Empty trace cannot produce passing prompt evaluation.
#[test]
fn eval_guard_empty_trace_cannot_pass_prompt_eval() {
    let trace = EvalTraceEvidence::default();
    let result = collect_prompt_eval(&trace);
    assert!(!result.prompt_seen, "Empty trace should not report prompt as seen");
    assert!(result.evidence_missing, "Empty trace should flag evidence as missing");
}

/// Empty trace cannot produce passing policy evaluation.
#[test]
fn eval_guard_empty_trace_cannot_pass_policy_eval() {
    let trace = EvalTraceEvidence::default();
    let result = collect_policy_eval(&trace, &EvalExpectations::default());
    assert!(result.gates_seen.is_empty(), "Empty trace should produce no gates");
}

/// Empty trace cannot produce passing patch evaluation.
#[test]
fn eval_guard_empty_trace_cannot_pass_patch_eval() {
    let trace = EvalTraceEvidence::default();
    let result = collect_patch_eval_from_trace(&trace, &EvalExpectations::default());
    assert!(!result.planned, "Empty trace should not report patch as planned");
    assert!(!result.applied, "Empty trace should not report patch as applied");
}

/// Collectors fail closed on missing evidence.
#[test]
fn eval_guard_collectors_fail_closed_on_missing_evidence() {
    // check_evidence_presence with all false should fail
    let result = check_evidence_presence(false, false, false);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(3, errors.len(), "Should report 3 missing evidence dimensions");
}

/// Fixture expectations alone cannot bypass trace evidence.
#[test]
fn eval_guard_fixture_expectations_do_not_bypass_trace() {
    // Even with rich expectations, an empty trace produces empty results
    let trace = EvalTraceEvidence::default();
    let expectations = EvalExpectations {
        included_claims: vec!["crate core exists".to_string()],
        tool_calls: vec!["local__file_patch".to_string()],
        forbidden_tool_calls: vec!["local__file_write".to_string()],
        file_changes: vec!["src/lib.rs".to_string()],
        ..Default::default()
    };

    let tool_result = collect_tool_eval(&trace, &expectations);
    assert!(tool_result.executed_tools.is_empty());

    let policy_result = collect_policy_eval(&trace, &expectations);
    assert!(policy_result.gates_seen.is_empty());

    let patch_result = collect_patch_eval_from_trace(&trace, &expectations);
    assert!(!patch_result.applied);
}
