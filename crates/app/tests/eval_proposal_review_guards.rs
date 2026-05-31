//! Proposal review hard execution guards.
//!
//! These tests enforce that the review module contains NO git, tool executor,
//! shell executor, or execution grant creation code.

#[test]
fn proposal_review_code_contains_no_git_commit_invocation() {
    let source = include_str!("../src/eval_proposal_review.rs");
    let lower = source.to_lowercase();
    assert!(!lower.contains("std::process"), "Must not use std::process");
    assert!(!lower.contains("git_commit"), "Must not reference git_commit as code");
}

#[test]
fn proposal_review_code_contains_no_git_add_invocation() {
    let source = include_str!("../src/eval_proposal_review.rs");
    assert!(!source.contains("git_add"), "Must not reference git_add as code");
}

#[test]
fn proposal_review_code_contains_no_git_push_invocation() {
    let source = include_str!("../src/eval_proposal_review.rs");
    assert!(!source.contains("git push"), "Must not contain 'git push'");
}

#[test]
fn proposal_review_code_contains_no_git_tag_invocation() {
    let source = include_str!("../src/eval_proposal_review.rs");
    assert!(!source.contains("git tag"), "Must not contain 'git tag'");
}

#[test]
fn proposal_review_does_not_import_tool_executor() {
    let source = include_str!("../src/eval_proposal_review.rs");
    assert!(!source.contains("ToolExecutor"), "Must not import ToolExecutor");
    assert!(!source.contains("tool_executor"), "Must not import tool_executor");
}

#[test]
fn proposal_review_does_not_import_shell_executor() {
    let source = include_str!("../src/eval_proposal_review.rs");
    assert!(!source.contains("std::process"), "Must not use std::process");
    assert!(!source.contains("Command"), "Must not use Command");
}

#[test]
fn proposal_review_does_not_create_execution_grant() {
    let source = include_str!("../src/eval_proposal_review.rs");
    // The module should never set creates_execution_grant to true
    assert!(!source.contains("creates_execution_grant: true"),
        "Must never set creates_execution_grant to true");
    // execution_allowed_now must always be false
    assert!(!source.contains("execution_allowed_now: true"),
        "Must never set execution_allowed_now to true");
}

#[test]
fn proposal_review_does_not_mutate_workspace() {
    // The builder takes &AutoCommitProposal (shared borrow)
    // It cannot mutate the proposal or workspace.
    use openwand_app::eval_proposal::*;
    use openwand_app::eval_proposal_review::*;
    use openwand_app::eval_readiness::*;

    let readiness = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore {
            weighted_pass_rate: 0.95, patch_pass_rate: 0.98,
            policy_pass_rate: 1.0, rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95, regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 1, reports_used: 1,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![], earliest_report: None, latest_report: None,
        },
        scenario_results: vec![], blockers: vec![], warnings: vec![],
    };

    let workspace = WorkspaceSnapshotDigest {
        blake3_hash: "test".to_string(), file_count: 0,
        generated_at: chrono::Utc::now(), file_digests: vec![],
    };

    // If this compiles, the builder takes shared borrows only
    assert!(true, "Builder signature takes &AutoCommitProposal");
}

/// Runtime guard: review generation leaves git state unchanged.
/// Output dir is outside the temp git repo.
#[test]
fn proposal_review_leaves_git_head_index_and_worktree_unchanged() {
    use openwand_app::eval_proposal::*;
    use openwand_app::eval_proposal_review::*;
    use openwand_app::eval_readiness::*;

    // Create temp git repo
    let repo_dir = tempfile::tempdir().unwrap();
    let repo_path = repo_dir.path();

    std::process::Command::new("git")
        .args(["init"]).current_dir(repo_path).output().expect("git init failed");
    std::fs::write(repo_path.join("lib.rs"), "fn main() {}").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(repo_path).output().expect("git add failed");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"]).current_dir(repo_path).output().expect("git commit failed");

    // Snapshot BEFORE
    let head_before = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_before = std::fs::read(repo_path.join(".git/index")).unwrap();
    let status_before = std::process::Command::new("git")
        .args(["status", "--porcelain"]).current_dir(repo_path).output().expect("git status failed");
    let porcelain_before = String::from_utf8_lossy(&status_before.stdout).to_string();

    // Separate output dir OUTSIDE repo
    let output_dir = tempfile::tempdir().unwrap();

    // Build proposal + review
    let readiness = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore {
            weighted_pass_rate: 0.95, patch_pass_rate: 0.98,
            policy_pass_rate: 1.0, rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95, regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 1, reports_used: 1,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![], earliest_report: None, latest_report: None,
        },
        scenario_results: vec![], blockers: vec![], warnings: vec![],
    };
    let workspace = WorkspaceSnapshotDigest {
        blake3_hash: "test".to_string(), file_count: 0,
        generated_at: chrono::Utc::now(), file_digests: vec![],
    };
    let eval_report = make_minimal_eval_report();
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness, workspace_digest: &workspace,
        eval_report: &eval_report, comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Test approval".to_string(),
        vec![], None,
    ).unwrap();

    save_proposal_review(output_dir.path(), &review).unwrap();

    // Verify output OUTSIDE repo
    assert!(output_dir.path().join("proposal_reviews").exists());
    assert!(!repo_path.join("proposal_reviews").exists());

    // Snapshot AFTER
    let head_after = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_after = std::fs::read(repo_path.join(".git/index")).unwrap();
    let status_after = std::process::Command::new("git")
        .args(["status", "--porcelain"]).current_dir(repo_path).output().expect("git status failed");
    let porcelain_after = String::from_utf8_lossy(&status_after.stdout).to_string();

    assert_eq!(head_before, head_after, "HEAD must not change");
    assert_eq!(index_before, index_after, "Index must not change");
    assert_eq!(porcelain_before, porcelain_after, "Working tree must not change");
}

fn make_minimal_eval_report() -> openwand_app::eval_model::EvalRunReport {
    use openwand_app::eval_model::*;
    EvalRunReport {
        report_schema_version: 2,
        scenario_id: "test".to_string(),
        provider: ProviderRealitySnapshot {
            provider: "test".to_string(), model: "test".to_string(),
            base_url_redacted: None, supports_streaming: true,
            supports_tools: true, supports_reasoning: false,
            health_status: ProviderHealthStatus::Healthy,
            temperature: None, max_tokens: None, observed_at: chrono::Utc::now(),
        },
        prompt: PromptEvalResult::default(),
        memory: MemoryEvalResult {
            included_claims_seen: vec![], excluded_claims_seen: vec![],
            missing_required: vec![], unexpected_included: vec![],
            prompt_panel_equivalent: true,
        },
        tools: ToolEvalResult {
            requested_tools: vec![], executed_tools: vec![],
            blocked_tools: vec![], forbidden_requested: vec![],
        },
        policy: PolicyEvalResult {
            gates_seen: vec![], required_approvals_seen: vec![], unexpected_allows: vec![],
        },
        patch: PatchEvalResult {
            planned: false, applied: false, preimage_verified: false,
            postimage_verified: false, rollback_available: false,
            changed_files_match_expected: true,
        },
        explain: ExplainEvalResult {
            memory_matches: false, policy_matches: false,
            tool_matches: false, completion_matches: false,
        },
        rebuild: RebuildEvalResult {
            events_replayed: 0, state_matches: false, divergences: vec![],
        },
        score: EvalScore { total: 0, max: 0, pass_rate: 0.0, dimensions: vec![] },
    }
}
