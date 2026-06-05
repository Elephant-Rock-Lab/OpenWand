//! Guard tests for workflow verification readiness.

// --- Crate import guards ---

#[test] fn readiness_crate_does_not_import_tool_executor() {
    let sources = [
        include_str!("../../workflow/src/workflow_verification_readiness.rs"),
        include_str!("../../workflow/src/workflow_verification_readiness_evaluator.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    }
}

#[test] fn readiness_crate_does_not_import_policy_engine() {
    let sources = [include_str!("../../workflow/src/workflow_verification_readiness.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
    }
}

#[test] fn readiness_crate_does_not_import_session_runner() {
    let sources = [include_str!("../../workflow/src/workflow_verification_readiness.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
    }
}

#[test] fn readiness_crate_does_not_import_process() {
    let sources = [
        include_str!("../../workflow/src/workflow_verification_readiness.rs"),
        include_str!("../../workflow/src/workflow_verification_readiness_evaluator.rs"),
    ];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn readiness_crate_does_not_import_memory() {
    let sources = [include_str!("../../workflow/src/workflow_verification_readiness.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore")));
    }
}

// --- App behavioral guards ---

#[test] fn readiness_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn readiness_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn readiness_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow_run") || l.contains("mutate")));
}

#[test] fn readiness_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    assert!(!src.contains("route_action"));
}

#[test] fn readiness_app_does_not_fetch_urls() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("reqwest")));
}

#[test] fn readiness_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    assert!(!src.contains(".append("));
}

#[test] fn readiness_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_verification_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

// --- UI surface guard ---

#[test] fn readiness_ui_does_not_expose_verify_execute_route() {
    let src = include_str!("../src/ui/workflow_verification_readiness_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("route")));
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

// --- Patch 7: structural authority guard ---

#[test]
fn serialized_readiness_record_has_no_authority() {
    use openwand_workflow::workflow_verification_readiness::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let request = VerificationReadinessRequest {
        target_kind: VerificationReadinessTargetKind::ManualResult,
        target_id: "wmr_g".into(),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        expected_target_hash: "h".into(),
        idempotency_key: "k".into(),
    };
    let rec = evaluate_readiness_metadata_only(&request, "reported_succeeded", "h", "wfx_g");
    let json = serde_json::to_string_pretty(&rec).unwrap().to_lowercase();
    assert!(json.contains("\"performs_verification\": false"));
    assert!(json.contains("\"verifies_external_truth\": false"));
    assert!(json.contains("\"promotes_trust\": false"));
    assert!(json.contains("\"schedules_verification\": false"));
    assert!(json.contains("\"execution_allowed_now\": false"));
}
