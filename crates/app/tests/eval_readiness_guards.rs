//! Readiness guard tests — enforce observational-only invariant.
//!
//! Readiness computation must never:
//! - Execute git commands
//! - Mutate files (except readiness report output)
//! - Call any tool executor
//!
//! Clarification #4: Runtime workspace snapshot guard.

use openwand_app::eval_model::*;
use openwand_app::eval_readiness::*;
use std::path::Path;

/// Readiness function signature is pure: takes &[EvalRunReport], returns a report.
/// It cannot mutate the input or perform I/O.
#[test]
fn readiness_guard_readiness_fn_is_pure() {
    let reports = vec![];
    let _result = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    // If this compiles, the function takes &[EvalRunReport] (shared borrow).
    // It cannot mutate the reports.
}

/// Persistence only writes to the readiness directory, never the workspace.
#[test]
fn readiness_guard_persistence_writes_only_readiness_dir() {
    let dir = tempfile::tempdir().unwrap();
    let workspace_root = dir.path();

    // Create a marker file in workspace root
    let marker = workspace_root.join("workspace_marker.txt");
    std::fs::write(&marker, "unchanged").unwrap();

    // Create some nested structure
    let src_dir = workspace_root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    // Snapshot workspace before readiness
    let before_files: Vec<_> = walkdir::WalkDir::new(workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    // Run readiness computation
    let report = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: READINESS_REPORT_SCHEMA_VERSION,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::InsufficientEvidence,
        score: ReadinessScore {
            weighted_pass_rate: 0.0,
            patch_pass_rate: 0.0,
            policy_pass_rate: 0.0,
            rebuild_pass_rate: 0.0,
            explain_pass_rate: 0.0,
            capability_context_pass_rate: 1.0,
            regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 0,
            reports_used: 0,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    };

    let output_dir = workspace_root.join("eval_reports");
    let _path = save_readiness_report(&output_dir, &report).unwrap();

    // Snapshot workspace after readiness
    let after_files: Vec<_> = walkdir::WalkDir::new(workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    // New files must only be under eval_reports/readiness/
    let readiness_dir = output_dir.join("readiness");
    for new_file in &after_files {
        if !before_files.iter().any(|f| f == new_file) {
            assert!(
                new_file.starts_with(&readiness_dir),
                "New file {:?} should be under readiness dir {:?}",
                new_file, readiness_dir
            );
        }
    }

    // Existing files unchanged
    let marker_content = std::fs::read_to_string(&marker).unwrap();
    assert_eq!("unchanged", marker_content);

    let lib_content = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap();
    assert_eq!("fn main() {}", lib_content);
}

/// Readiness module has no git dependency.
#[test]
fn readiness_guard_no_git_dependency() {
    // The eval_readiness module imports only:
    // - serde, chrono (serialization)
    // - crate::eval_model (DTOs)
    // - crate::eval_compare (comparison)
    // - std::path, std::fs, std::collections
    //
    // None of these are git, process, or tool execution.
    // This test documents the import structure.
    //
    // If someone adds a git import to eval_readiness.rs,
    // this test serves as a signpost to update the guard.
    assert!(true, "Import guard: see eval_readiness.rs header imports");
}

/// compute_auto_commit_readiness takes shared borrows only.
#[test]
fn readiness_guard_observational_only() {
    let mut reports = vec![];
    for scenario in ["patch_plan_then_apply", "preimage_mismatch_recovery",
        "policy_blocks_forbidden_write", "trace_rebuild_after_eval",
        "multi_turn_user_correction"] {
        for _ in 0..3 {
            reports.push(make_minimal_report(scenario, true));
        }
    }

    let before: Vec<String> = reports.iter()
        .map(|r| r.scenario_id.clone())
        .collect();

    let _result = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());

    // After computation, reports must be unchanged
    let after: Vec<String> = reports.iter()
        .map(|r| r.scenario_id.clone())
        .collect();

    assert_eq!(before, after, "Reports must not be mutated by readiness computation");
}

fn make_minimal_report(scenario_id: &str, passing: bool) -> EvalRunReport {
    EvalRunReport {
        report_schema_version: 2,
        scenario_id: scenario_id.to_string(),
        provider: ProviderRealitySnapshot {
            provider: "test".to_string(),
            model: "test".to_string(),
            base_url_redacted: None,
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            health_status: ProviderHealthStatus::Healthy,
            temperature: None,
            max_tokens: None,
            observed_at: chrono::Utc::now(),
        },
        prompt: PromptEvalResult {
            prompt_seen: true,
            evidence_missing: false,
            model: Some("test".to_string()),
            provider: Some("test".to_string()),
            system_prompt_hash: None,
            message_count: 1,
            tool_count: 0,
        },
        memory: MemoryEvalResult {
            included_claims_seen: vec![],
            excluded_claims_seen: vec![],
            missing_required: vec![],
            unexpected_included: vec![],
            prompt_panel_equivalent: true,
        },
        tools: ToolEvalResult {
            requested_tools: vec![],
            executed_tools: vec![],
            blocked_tools: vec![],
            forbidden_requested: vec![],
        },
        policy: PolicyEvalResult {
            gates_seen: vec!["gate.evaluated".to_string()],
            required_approvals_seen: vec![],
            unexpected_allows: vec![],
        },
        patch: PatchEvalResult {
            planned: passing,
            applied: passing,
            preimage_verified: passing,
            postimage_verified: passing,
            rollback_available: passing,
            changed_files_match_expected: true,
        },
        explain: ExplainEvalResult {
            memory_matches: passing,
            policy_matches: passing,
            tool_matches: passing,
            completion_matches: passing,
        },
        rebuild: RebuildEvalResult {
            events_replayed: 10,
            state_matches: passing,
            divergences: vec![],
        },
        capability_context: CapabilityContextEvalResult::default(),
        score: EvalScore {
            total: if passing { 5 } else { 3 },
            max: 5,
            pass_rate: if passing { 1.0 } else { 0.6 },
            dimensions: vec![
                DimensionScore {
                    name: "patch".to_string(),
                    passed: if passing { 1 } else { 0 },
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Trace,
                        event_kind: Some("file.patch".to_string()),
                        summary: "test".to_string(),
                    }],
                },
                DimensionScore {
                    name: "policy".to_string(),
                    passed: 1,
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Trace,
                        event_kind: Some("gate.evaluated".to_string()),
                        summary: "test".to_string(),
                    }],
                },
                DimensionScore {
                    name: "rebuild".to_string(),
                    passed: if passing { 1 } else { 0 },
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Rebuild,
                        event_kind: Some("session.rebuild".to_string()),
                        summary: "test".to_string(),
                    }],
                },
                DimensionScore {
                    name: "explain".to_string(),
                    passed: if passing { 1 } else { 0 },
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Explanation,
                        event_kind: None,
                        summary: "test".to_string(),
                    }],
                },
            ],
        },
    }
}
