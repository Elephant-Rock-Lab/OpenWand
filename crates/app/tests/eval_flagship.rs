//! Flagship scenario set validation.
//!
//! Validates all 8 evaluation scenario fixtures load, have non-empty expectations,
//! and every scenario requires both rebuild and explain verification.

use openwand_app::eval_model::*;

fn load_all() -> Vec<EvalScenario> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("eval");
    load_eval_fixtures(&dir).unwrap()
}

#[test]
fn eval_flagship_scenarios_load() {
    let scenarios = load_all();
    assert!(scenarios.len() >= 8, "Expected >= 8, got {}", scenarios.len());

    // Verify all expected IDs present
    let ids: Vec<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"memory_verified_used"));
    assert!(ids.contains(&"low_confidence_excluded"));
    assert!(ids.contains(&"conflict_requires_review"));
    assert!(ids.contains(&"policy_blocks_forbidden_write"));
    assert!(ids.contains(&"patch_plan_then_apply"));
    assert!(ids.contains(&"preimage_mismatch_recovery"));
    assert!(ids.contains(&"multi_turn_user_correction"));
    assert!(ids.contains(&"trace_rebuild_after_eval"));
}

#[test]
fn eval_flagship_expectations_are_non_empty() {
    let scenarios = load_all();
    for s in &scenarios {
        // Each scenario must have at least one expected outcome dimension
        let has_any_expectation = !s.expected.included_claims.is_empty()
            || !s.expected.excluded_claims.is_empty()
            || !s.expected.tool_calls.is_empty()
            || !s.expected.forbidden_tool_calls.is_empty()
            || !s.expected.file_changes.is_empty()
            || !s.expected.policy_events.is_empty();

        assert!(
            has_any_expectation,
            "Scenario '{}' has completely empty expectations",
            s.id
        );
    }
}

#[test]
fn eval_flagship_all_have_rebuild_expectation() {
    let scenarios = load_all();
    for s in &scenarios {
        assert!(
            s.expected.rebuild_matches,
            "Scenario '{}' does not require rebuild_matches",
            s.id
        );
    }
}

#[test]
fn eval_flagship_all_have_explain_expectation() {
    let scenarios = load_all();
    for s in &scenarios {
        assert!(
            s.expected.explain_matches,
            "Scenario '{}' does not require explain_matches",
            s.id
        );
    }
}
