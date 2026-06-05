//! Guard tests for workflow evidence chain inspector.

// --- Crate import guards ---

#[test] fn inspector_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_evidence_chain_inspector.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    }
}

#[test] fn inspector_crate_does_not_import_policy_engine() {
    let sources = [include_str!("../../workflow/src/workflow_evidence_chain_inspector.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
    }
}

#[test] fn inspector_crate_does_not_import_session_runner() {
    let sources = [include_str!("../../workflow/src/workflow_evidence_chain_inspector.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
    }
}

#[test] fn inspector_crate_does_not_import_process() {
    let sources = [
        include_str!("../../workflow/src/workflow_evidence_chain_inspector.rs"),
        include_str!("../../workflow/src/workflow_evidence_chain_inspector_validation.rs"),
    ];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn inspector_crate_does_not_import_memory() {
    let sources = [include_str!("../../workflow/src/workflow_evidence_chain_inspector.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore")));
    }
}

// --- App behavioral guards ---

#[test] fn inspector_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn inspector_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn inspector_app_does_not_verify_external_state() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    assert!(!src.contains("verify_shell") && !src.contains("verify_git"));
}

#[test] fn inspector_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow_run") || l.contains("mutate")));
}

#[test] fn inspector_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("route_action")));
}

#[test] fn inspector_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("resolve_approval")));
}

#[test] fn inspector_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    assert!(!src.contains(".append("));
}

#[test] fn inspector_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_evidence_chain_inspector.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

// --- UI surface guard ---

#[test] fn inspector_ui_does_not_expose_execute_verify_route_resolve_reconcile() {
    let src = include_str!("../src/ui/workflow_evidence_chain_inspector_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    assert!(!fn_lines.iter().any(|l| l.contains("route")));
    assert!(!fn_lines.iter().any(|l| l.contains("resolve")));
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
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

// --- Wave-specific guards ---

// Patch 4: no-certification flags
#[test]
fn inspection_state_serialized_json_has_no_authority() {
    use openwand_workflow::workflow_evidence_chain_inspector::*;
    let state = build_inspection_state("wfx_g", vec![], vec![], false);
    let json = serde_json::to_string_pretty(&state).unwrap().to_lowercase();
    assert!(json.contains("\"certifies_external_truth\": false"));
    assert!(json.contains("\"verifies_artifacts\": false"));
    assert!(json.contains("\"executes_commands\": false"));
    assert!(json.contains("\"mutates_workflow_state\": false"));
}

// Patch 5: recorded_evidence naming
#[test]
fn audit_packet_records_use_recorded_evidence_not_verified() {
    use openwand_workflow::workflow_evidence_chain_inspector::*;
    let rec = AuditPacketRecord {
        record_type: "test".into(),
        record_id: "id".into(),
        record_hash: "h".into(),
        source_path_hint: None,
        recorded_evidence: serde_json::json!({}),
    };
    let json = serde_json::to_string(&rec).unwrap();
    assert!(json.contains("recorded_evidence"));
    assert!(!json.contains("verified_record"));
    assert!(!json.contains("truth_record"));
    assert!(!json.contains("certified_record"));
}
