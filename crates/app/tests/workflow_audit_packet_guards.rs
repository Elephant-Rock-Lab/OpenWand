//! Guard tests for audit packet review and distribution.
//!
//! Patch 8: Both record types covered with no-authority guards.

// --- Crate import guards ---

#[test] fn review_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_audit_packet_review.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    }
}

#[test] fn distribution_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_audit_packet_distribution.rs")];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    }
}

#[test] fn review_crate_does_not_import_policy_engine() {
    let src = include_str!("../../workflow/src/workflow_audit_packet_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn distribution_crate_does_not_import_policy_engine() {
    let src = include_str!("../../workflow/src/workflow_audit_packet_distribution.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn review_crate_does_not_import_session_runner() {
    let src = include_str!("../../workflow/src/workflow_audit_packet_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
}

#[test] fn review_crate_does_not_import_process() {
    let sources = [
        include_str!("../../workflow/src/workflow_audit_packet_review.rs"),
        include_str!("../../workflow/src/workflow_audit_packet_distribution.rs"),
    ];
    for src in &sources { assert!(!src.lines().any(|l| l.contains("std::process"))); }
}

// --- App behavioral guards ---

#[test] fn review_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_audit_packet_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn distribution_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_audit_packet_distribution.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn review_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_audit_packet_review.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow") || l.contains("mutate")));
}

#[test] fn distribution_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_audit_packet_distribution.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow") || l.contains("mutate")));
}

#[test] fn review_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_audit_packet_review.rs");
    assert!(!src.contains(".append("));
}

#[test] fn distribution_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_audit_packet_distribution.rs");
    assert!(!src.contains(".append("));
}

// --- Patch 8: No-authority serialized guards ---

#[test]
fn audit_packet_review_does_not_certify_or_verify_packet() {
    use openwand_workflow::workflow_audit_packet_review::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let req = AuditPacketReviewRequest {
        inspection_id: "weci_g".into(),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        expected_audit_packet_hash: "h".into(),
        expected_chain_hash: "h".into(),
        reviewer: "bob".into(),
        decision: AuditPacketReviewDecision::ReviewedWithCaveats,
        scope: "guard".into(),
        caveats: vec![],
        idempotency_key: "k".into(),
    };
    let rec = build_audit_packet_review(req);
    assert!(!rec.certifies_truth);
    assert!(!rec.approves_packet_truth);
    assert!(!rec.verifies_packet_contents);
    assert!(!rec.certifies_external_truth);
    assert!(!rec.modifies_audit_packet);
}

#[test]
fn audit_packet_distribution_does_not_prove_delivery() {
    use openwand_workflow::workflow_audit_packet_distribution::*;
    use openwand_workflow::workflow_audit_packet_review::AuditPacketReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let req = AuditPacketDistributionRequest {
        review_id: AuditPacketReviewId("wapr_g".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        expected_review_hash: "h".into(),
        audit_packet_hash: "h".into(),
        chain_hash: "h".into(),
        inspection_id: "weci_g".into(),
        destination: AuditPacketDistributionDestination {
            destination_kind: AuditPacketDestinationKind::Other,
            label: "test".into(),
            reference: "ref".into(),
            operator_supplied_hash: None,
            notes: vec![],
        },
        distribution_notes: vec![],
        idempotency_key: "k".into(),
    };
    let rec = build_audit_packet_distribution(req);
    assert!(!rec.proof_of_delivery);
    assert!(!rec.recipient_acceptance_proven);
    assert!(!rec.destination_verified);
    assert!(!rec.external_system_integrated);
}

#[test]
fn audit_packet_distribution_does_not_send_or_upload() {
    use openwand_workflow::workflow_audit_packet_distribution::*;
    let src = include_str!("../../workflow/src/workflow_audit_packet_distribution.rs");
    assert!(src.contains("sends_external_message: false"));
    assert!(src.contains("uploads_files: false"));
}

#[test]
fn review_distribution_do_not_mutate_packet_or_workflow() {
    let review_src = include_str!("../../workflow/src/workflow_audit_packet_review.rs");
    let dist_src = include_str!("../../workflow/src/workflow_audit_packet_distribution.rs");
    assert!(review_src.contains("modifies_audit_packet: false"));
    assert!(review_src.contains("mutates_workflow_state: false"));
    assert!(dist_src.contains("modifies_audit_packet: false"));
    assert!(dist_src.contains("mutates_workflow_state: false"));
}

#[test]
fn review_distribution_do_not_append_trace_or_write_memory() {
    let review_src = include_str!("../../workflow/src/workflow_audit_packet_review.rs");
    let dist_src = include_str!("../../workflow/src/workflow_audit_packet_distribution.rs");
    assert!(review_src.contains("appends_trace: false"));
    assert!(review_src.contains("writes_memory: false"));
    assert!(dist_src.contains("appends_trace: false"));
    assert!(dist_src.contains("writes_memory: false"));
}

#[test]
fn serialized_review_contains_no_certified_verified_truth_fields() {
    use openwand_workflow::workflow_audit_packet_review::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let req = AuditPacketReviewRequest {
        inspection_id: "weci_s".into(),
        workflow_execution_id: WorkflowExecutionId("wfx_s".into()),
        expected_audit_packet_hash: "h".into(),
        expected_chain_hash: "h".into(),
        reviewer: "bob".into(),
        decision: AuditPacketReviewDecision::NotedWithoutCertification,
        scope: "guard".into(),
        caveats: vec![],
        idempotency_key: "k".into(),
    };
    let rec = build_audit_packet_review(req);
    let json = serde_json::to_string(&rec).unwrap().to_lowercase();
    assert!(json.contains("\"certifies_truth\":false"));
    assert!(json.contains("\"verifies_packet_contents\":false"));
    assert!(json.contains("\"certifies_external_truth\":false"));
}

#[test]
fn serialized_distribution_contains_no_delivery_proof_fields() {
    use openwand_workflow::workflow_audit_packet_distribution::*;
    use openwand_workflow::workflow_audit_packet_review::AuditPacketReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let req = AuditPacketDistributionRequest {
        review_id: AuditPacketReviewId("wapr_s".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_s".into()),
        expected_review_hash: "h".into(),
        audit_packet_hash: "h".into(),
        chain_hash: "h".into(),
        inspection_id: "weci_s".into(),
        destination: AuditPacketDistributionDestination {
            destination_kind: AuditPacketDestinationKind::FileShare,
            label: "test".into(),
            reference: "ref".into(),
            operator_supplied_hash: None,
            notes: vec![],
        },
        distribution_notes: vec![],
        idempotency_key: "k".into(),
    };
    let rec = build_audit_packet_distribution(req);
    let json = serde_json::to_string(&rec).unwrap().to_lowercase();
    assert!(json.contains("\"proof_of_delivery\":false"));
    assert!(json.contains("\"recipient_acceptance_proven\":false"));
    assert!(json.contains("\"destination_verified\":false"));
}

// --- UI surface guard ---

#[test] fn review_ui_does_not_expose_verify_execute_route() {
    let src = include_str!("../src/ui/workflow_audit_packet_review_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
}

#[test] fn distribution_ui_does_not_expose_send_upload_verify() {
    let src = include_str!("../src/ui/workflow_audit_packet_distribution_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("send")));
    assert!(!fn_lines.iter().any(|l| l.contains("upload")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
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
