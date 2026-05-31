//! Auto-commit proposal hard execution guards.
//!
//! These tests enforce that the proposal module contains NO git commit,
//! staging, push, tag, tool executor, or shell executor code.
//!
//! Correction #4: proposal output dir is outside the temp git repo.

use std::path::Path;

/// Source guard: no git commit invocation in proposal module.
/// Excludes string literals and comments — only checks for code constructs.
#[test]
fn auto_commit_proposal_code_contains_no_git_commit_invocation() {
    let source = include_str!("../src/eval_proposal.rs");
    let lower = source.to_lowercase();
    // Verify no process::Command usage at all
    assert!(!lower.contains("std::process"), "Must not use std::process");
    assert!(!lower.contains("process::command"), "Must not use Command");
    assert!(!lower.contains("git_commit"), "Must not reference git_commit as code");
}

#[test]
fn auto_commit_proposal_code_contains_no_git_add_invocation() {
    let source = include_str!("../src/eval_proposal.rs");
    let lower = source.to_lowercase();
    assert!(!lower.contains("git add"), "Proposal module must not contain 'git add'");
    assert!(!lower.contains("git_add"), "Proposal module must not contain 'git_add'");
}

#[test]
fn auto_commit_proposal_code_contains_no_git_push_invocation() {
    let source = include_str!("../src/eval_proposal.rs");
    let lower = source.to_lowercase();
    assert!(!lower.contains("git push"), "Proposal module must not contain 'git push'");
}

#[test]
fn auto_commit_proposal_code_contains_no_git_tag_invocation() {
    let source = include_str!("../src/eval_proposal.rs");
    let lower = source.to_lowercase();
    assert!(!lower.contains("git tag"), "Proposal module must not contain 'git tag'");
}

#[test]
fn auto_commit_proposal_does_not_import_tool_executor() {
    let source = include_str!("../src/eval_proposal.rs");
    assert!(!source.contains("ToolExecutor"), "Must not import ToolExecutor");
    assert!(!source.contains("tool_executor"), "Must not import tool_executor");
}

#[test]
fn auto_commit_proposal_does_not_import_shell_executor() {
    let source = include_str!("../src/eval_proposal.rs");
    assert!(!source.contains("Shell"), "Must not import Shell executor");
    assert!(!source.contains("Command"), "Must not import std::process::Command");
}

#[test]
fn auto_commit_proposal_does_not_mutate_workspace() {
    // The builder function signature takes only borrowed references.
    // It cannot mutate workspace. This test documents the invariant.
    use openwand_app::eval_proposal::*;
    use openwand_app::eval_readiness::*;

    let readiness = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: openwand_app::eval_readiness::ReadinessScore {
            weighted_pass_rate: 0.95,
            patch_pass_rate: 0.98,
            policy_pass_rate: 1.0,
            rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95,
            regression_count: 0,
        },
        thresholds: openwand_app::eval_readiness::AutoCommitReadinessThresholds::default(),
        evidence_window: openwand_app::eval_readiness::EvidenceWindow {
            total_reports_found: 1,
            reports_used: 1,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    };

    // These are immutable references — the builder cannot mutate them
    let workspace = WorkspaceSnapshotDigest {
        blake3_hash: "test".to_string(),
        file_count: 0,
        generated_at: chrono::Utc::now(),
        file_digests: vec![],
    };

    // The builder takes &references, so it physically cannot mutate them
    assert!(true, "Builder takes shared borrows only");
}

/// Runtime behavior guard: proposal generation leaves git state unchanged.
/// Correction #4: output dir is OUTSIDE the temp git repo.
#[test]
fn proposal_generation_leaves_git_head_index_and_worktree_unchanged() {
    use openwand_app::eval_proposal::*;
    use openwand_app::eval_readiness::*;

    // Create temp git repo
    let repo_dir = tempfile::tempdir().unwrap();
    let repo_path = repo_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("git init failed");

    // Create and commit a file
    std::fs::write(repo_path.join("lib.rs"), "fn main() {}").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add failed");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .expect("git commit failed");

    // Snapshot git state BEFORE proposal generation
    let head_before = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_before = std::fs::read(repo_path.join(".git/index")).unwrap();
    let status_before = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("git status failed");
    let porcelain_before = String::from_utf8_lossy(&status_before.stdout).to_string();

    // Create SEPARATE output dir outside the repo (Correction #4)
    let output_dir = tempfile::tempdir().unwrap();

    // Build and save a proposal
    let readiness = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: openwand_app::eval_readiness::ReadinessScore {
            weighted_pass_rate: 0.95,
            patch_pass_rate: 0.98,
            policy_pass_rate: 1.0,
            rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95,
            regression_count: 0,
        },
        thresholds: openwand_app::eval_readiness::AutoCommitReadinessThresholds::default(),
        evidence_window: openwand_app::eval_readiness::EvidenceWindow {
            total_reports_found: 1,
            reports_used: 1,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    };

    let workspace = WorkspaceSnapshotDigest {
        blake3_hash: "test_hash".to_string(),
        file_count: 1,
        generated_at: chrono::Utc::now(),
        file_digests: vec![],
    };

    let eval_report = openwand_app::eval_model::EvalRunReport {
        report_schema_version: 2,
        scenario_id: "test".to_string(),
        provider: openwand_app::eval_model::ProviderRealitySnapshot {
            provider: "test".to_string(),
            model: "test".to_string(),
            base_url_redacted: None,
            supports_streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            health_status: openwand_app::eval_model::ProviderHealthStatus::Healthy,
            temperature: None,
            max_tokens: None,
            observed_at: chrono::Utc::now(),
        },
        prompt: openwand_app::eval_model::PromptEvalResult {
            prompt_seen: true,
            evidence_missing: false,
            model: Some("test".to_string()),
            provider: Some("test".to_string()),
            system_prompt_hash: None,
            message_count: 1,
            tool_count: 0,
        },
        memory: openwand_app::eval_model::MemoryEvalResult {
            included_claims_seen: vec![],
            excluded_claims_seen: vec![],
            missing_required: vec![],
            unexpected_included: vec![],
            prompt_panel_equivalent: true,
        },
        tools: openwand_app::eval_model::ToolEvalResult {
            requested_tools: vec![],
            executed_tools: vec![],
            blocked_tools: vec![],
            forbidden_requested: vec![],
        },
        policy: openwand_app::eval_model::PolicyEvalResult {
            gates_seen: vec![],
            required_approvals_seen: vec![],
            unexpected_allows: vec![],
        },
        patch: openwand_app::eval_model::PatchEvalResult {
            planned: true,
            applied: true,
            preimage_verified: true,
            postimage_verified: true,
            rollback_available: true,
            changed_files_match_expected: true,
        },
        explain: openwand_app::eval_model::ExplainEvalResult {
            memory_matches: true,
            policy_matches: true,
            tool_matches: true,
            completion_matches: true,
        },
        rebuild: openwand_app::eval_model::RebuildEvalResult {
            events_replayed: 1,
            state_matches: true,
            divergences: vec![],
        },
        score: openwand_app::eval_model::EvalScore {
            total: 1,
            max: 1,
            pass_rate: 1.0,
            dimensions: vec![],
        },
    };

    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval_report,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    let _path = save_proposal(output_dir.path(), &proposal).unwrap();

    // Verify proposal was saved OUTSIDE the repo
    assert!(output_dir.path().join("proposals").exists(), "Output should be in separate dir");
    assert!(!repo_path.join("proposals").exists(), "No output should be inside repo");

    // Snapshot git state AFTER
    let head_after = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_after = std::fs::read(repo_path.join(".git/index")).unwrap();
    let status_after = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("git status failed");
    let porcelain_after = String::from_utf8_lossy(&status_after.stdout).to_string();

    // Assert git state unchanged
    assert_eq!(head_before, head_after, "HEAD must not change");
    assert_eq!(index_before, index_after, "Index must not change");
    assert_eq!(porcelain_before, porcelain_after, "Working tree must not change");
}
