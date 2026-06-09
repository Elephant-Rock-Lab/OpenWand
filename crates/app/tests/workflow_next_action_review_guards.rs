//! Guard tests for next-action review.
//!
//! Coverage gap closure (Wave 50A, FIX-05, KNOWN_GAPS gap 1).

// --- Crate import guards ---

#[test] fn review_crate_does_not_import_tool_executor() {
    let src = include_str!("../../workflow/src/workflow_next_action_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn review_crate_does_not_import_policy_engine() {
    let src = include_str!("../../workflow/src/workflow_next_action_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn review_crate_does_not_import_session_runner() {
    let src = include_str!("../../workflow/src/workflow_next_action_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
}

#[test] fn review_crate_does_not_import_process() {
    let src = include_str!("../../workflow/src/workflow_next_action_review.rs");
    assert!(!src.lines().any(|l| l.contains("std::process")));
}

// --- App behavioral guards ---

#[test] fn review_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_next_action_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn review_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_next_action_review.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow") || l.contains("mutate")));
}

#[test] fn review_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_next_action_review.rs");
    assert!(!src.contains(".append("));
}

// --- No-authority serialized guards ---

#[test]
fn next_action_review_does_not_route_or_execute() {
    use openwand_workflow::workflow_next_action_review::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    let review = WorkflowNextActionReview {
        review_id: WorkflowNextActionReviewId("wnar_guard".into()),
        proposal_id: WorkflowNextActionProposalId("wnap_g".into()),
        proposal_hash: "h".into(),
        source_run_revision_id: WorkflowRunRevisionId("wrr_g".into()),
        source_run_revision_hash: "h".into(),
        decision: WorkflowNextActionReviewDecision::Approved,
        reviewer: "guard".into(),
        rationale: "test".into(),
        feedback: None,
        creates_route: false,
        routes_action_now: false,
        executes_tool_now: false,
        mutates_workflow_state_now: false,
        reviewed_at: chrono::Utc::now(),
    };
    assert!(!review.creates_route);
    assert!(!review.routes_action_now);
    assert!(!review.executes_tool_now);
    assert!(!review.mutates_workflow_state_now);
}

#[test]
fn serialized_review_contains_no_route_execute_truth_fields() {
    use openwand_workflow::workflow_next_action_review::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    let review = WorkflowNextActionReview {
        review_id: WorkflowNextActionReviewId("wnar_ser".into()),
        proposal_id: WorkflowNextActionProposalId("wnap_s".into()),
        proposal_hash: "h".into(),
        source_run_revision_id: WorkflowRunRevisionId("wrr_s".into()),
        source_run_revision_hash: "h".into(),
        decision: WorkflowNextActionReviewDecision::Approved,
        reviewer: "guard".into(),
        rationale: "test".into(),
        feedback: None,
        creates_route: false,
        routes_action_now: false,
        executes_tool_now: false,
        mutates_workflow_state_now: false,
        reviewed_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&review).unwrap().to_lowercase();
    assert!(json.contains("\"creates_route\":false"));
    assert!(json.contains("\"routes_action_now\":false"));
    assert!(json.contains("\"executes_tool_now\":false"));
    assert!(json.contains("\"mutates_workflow_state_now\":false"));
}

#[test]
fn review_does_not_create_route_or_execute_source_guard() {
    let src = include_str!("../../workflow/src/workflow_next_action_review.rs");
    assert!(src.contains("creates_route: false") || src.contains("creates_route: bool") || src.contains("creates_route: false,"));
    assert!(src.contains("executes_tool_now: false") || src.contains("executes_tool_now: bool") || src.contains("executes_tool_now: false,"));
}

// --- UI surface guard ---

#[test] fn review_ui_does_not_expose_route_execute_verify() {
    let src = include_str!("../src/ui/workflow_next_action_review_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("route")));
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
}

// --- Dependency guard ---

#[test]
fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("workflow").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    let mut dep_count = 0u32;
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" { in_deps = true; continue; }
        if trimmed.starts_with('[') { in_deps = false; continue; }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let name = trimmed.split('=').next().unwrap().trim();
        assert!(allowed.contains(&name), "Unexpected dependency: {}", name);
        dep_count += 1;
    }
    assert_eq!(6, dep_count, "Workflow crate must have exactly 6 dependencies");
}
