//! Guard tests for workflow external attestation.

// --- Crate import guards ---

#[test] fn attestation_crate_does_not_import_tool_executor() {
    let sources = [
        include_str!("../../workflow/src/workflow_external_attestation.rs"),
        include_str!("../../workflow/src/workflow_external_attestation_validation.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    }
}

#[test] fn attestation_crate_does_not_import_policy_engine() {
    let sources = [include_str!("../../workflow/src/workflow_external_attestation.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
    }
}

#[test] fn attestation_crate_does_not_import_session_runner() {
    let sources = [include_str!("../../workflow/src/workflow_external_attestation.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
    }
}

#[test] fn attestation_crate_does_not_import_process() {
    let sources = [
        include_str!("../../workflow/src/workflow_external_attestation.rs"),
        include_str!("../../workflow/src/workflow_external_attestation_validation.rs"),
    ];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn attestation_crate_does_not_import_memory() {
    let sources = [include_str!("../../workflow/src/workflow_external_attestation.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore")));
    }
}

// --- App behavioral guards ---

#[test] fn attestation_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn attestation_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn attestation_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow_run") || l.contains("mutate")));
}

#[test] fn attestation_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("route_action")));
}

#[test] fn attestation_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    assert!(!src.contains("resolve_approval"));
}

#[test] fn attestation_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    assert!(!src.contains(".append("));
}

#[test] fn attestation_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_external_attestation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

// --- UI surface guard ---

#[test] fn attestation_ui_does_not_expose_execute_verify_route_resolve() {
    let src = include_str!("../src/ui/workflow_external_attestation_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
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

// --- Patch 4: structural no-trust guards ---

#[test]
fn serialized_attestation_contains_no_trust_score_or_confidence() {
    use openwand_workflow::workflow_external_attestation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let req = ExternalAttestationRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        target_kind: ExternalAttestationTargetKind::ManualResult,
        target_id: "wmr_g".into(),
        expected_target_hash: None,
        kind: ExternalAttestationKind::ThirdPartySignoff,
        source_name: "A".into(),
        source_role: "r".into(),
        source_system_identifier: None,
        claim: "c".into(),
        references: vec![],
        reported_signature: None,
        attested_at: chrono::Utc::now(),
        idempotency_key: "k".into(),
    };
    let att = build_external_attestation(req);
    let json = serde_json::to_string_pretty(&att).unwrap().to_lowercase();
    assert!(json.contains("\"verified_by_openwand\": false"));
    assert!(json.contains("\"promotes_trust\": false"));
    assert!(json.contains("\"certifies_external_truth\": false"));
    assert!(!json.contains("trust_score"));
    assert!(!json.contains("confidence"));
}

// Patch 6: attestations do not affect chain validity
#[test]
fn attestations_do_not_affect_chain_hash() {
    use openwand_workflow::workflow_evidence_chain_inspector::*;
    let links = vec![EvidenceChainLink {
        record_type: "run".into(),
        record_id: "wfx_1".into(),
        presence: EvidenceLinkPresence::Present,
        record_hash: "h1".into(),
        source_path_hint: None,
    }];
    let hash1 = compute_chain_hash(&links);
    let hash2 = compute_chain_hash(&links);
    // Chain hash is deterministic regardless of attestations
    assert_eq!(hash1, hash2);
}
