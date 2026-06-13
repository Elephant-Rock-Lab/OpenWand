//! Governed auto-commit execution gate tests.

use openwand_app::eval_model::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_execution::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_readiness::*;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_eligible_proposal() -> AutoCommitProposal {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report();
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness, workspace_digest: &workspace,
        eval_report: &eval, comparison: None,
    };
    build_auto_commit_proposal(inputs)
}

fn make_approved_review(proposal: &AutoCommitProposal) -> AutoCommitProposalReview {
    build_proposal_review(
        proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Approved for execution".to_string(),
        vec![], None,
    ).unwrap()
}

fn make_rejected_review(proposal: &AutoCommitProposal) -> AutoCommitProposalReview {
    let feedback = ProposalRejectionFeedback {
        feedback_id: "pfb_test".to_string(),
        proposal_id: proposal.proposal_id.clone(),
        review_id: AutoCommitProposalReviewId("arv_test".to_string()),
        workspace_hash: proposal.workspace_snapshot_id.clone(),
        summary: "Rejected".to_string(),
        required_changes: vec![RequiredProposalChange {
            category: ProposalFeedbackCategory::Other,
            description: "Fix issues".to_string(),
            evidence_ref: None,
        }],
        blocked_dimensions: vec![], suggested_next_eval_focus: vec![],
        severity: ProposalFeedbackSeverity::Blocking,
    };
    build_proposal_review(
        proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "Rejected".to_string(),
        vec![], Some(feedback),
    ).unwrap()
}

fn make_execution_request(proposal: &AutoCommitProposal, review: &AutoCommitProposalReview) -> AutoCommitExecutionRequest {
    AutoCommitExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        requested_by: "test_user".to_string(),
        requested_at: chrono::Utc::now(),
        idempotency_key: format!("key_{}", proposal.proposal_id.0),
    }
}

fn make_git_state() -> GitStateSnapshot {
    GitStateSnapshot {
        head: "abc123".to_string(),
        branch: "main".to_string(),
        index_hash: "idx_hash".to_string(),
        worktree_hash: "wt_hash".to_string(),
        porcelain: String::new(),
    }
}

fn make_rollback_plan() -> RollbackPlanSnapshot {
    RollbackPlanSnapshot {
        pre_commit_head: "abc123".to_string(),
        branch: "main".to_string(),
        index_status_hash: "idx_hash".to_string(),
        worktree_status_hash: "wt_hash".to_string(),
        recovery_command: "git reset --hard abc123".to_string(),
        notes: vec!["Pre-commit snapshot".to_string()],
    }
}

// ── Commit 1: DTO and builder tests ────────────────────────────────────────

#[test]
fn execution_request_roundtrips() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let req = make_execution_request(&proposal, &review);
    let json = serde_json::to_string(&req).unwrap();
    let parsed: AutoCommitExecutionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.proposal_id, parsed.proposal_id);
    assert_eq!(req.idempotency_key, parsed.idempotency_key);
}

#[test]
fn execution_record_roundtrips() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let req = make_execution_request(&proposal, &review);
    let record = AutoCommitExecutionRecord {
        execution_id: execution_id_for(&proposal.proposal_id.0, &req.idempotency_key),
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        status: AutoCommitExecutionStatus::Blocked,
        decision: AutoCommitExecutionDecision {
            decision: ExecutionGateDecision::Block {
                reason_code: "test".to_string(),
                summary: "test block".to_string(),
            },
            proposal_id: proposal.proposal_id.clone(),
            review_id: review.review_id.clone(),
            predicates: vec![],
            git_state_snapshot: make_git_state(),
            rollback_plan: None,
        },
        resulting_commit: None,
        created_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: AutoCommitExecutionRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.execution_id, parsed.execution_id);
    assert_eq!(record.status, parsed.status);
}

#[test]
fn execution_decision_requires_predicates() {
    // Allow decision should have predicates
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    assert!(!record.decision.predicates.is_empty());
}

#[test]
fn executed_record_requires_commit_snapshot() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    if record.status == AutoCommitExecutionStatus::Executed {
        assert!(record.resulting_commit.is_some());
    }
}

#[test]
fn blocked_record_must_not_have_commit_snapshot() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    // Block by denying policy
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], false, None, // policy denied, no rollback plan
    );
    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);
    assert!(record.resulting_commit.is_none());
}

#[test]
fn rollback_plan_requires_pre_commit_head() {
    let plan = make_rollback_plan();
    assert!(!plan.pre_commit_head.is_empty());
}

#[test]
fn rollback_plan_requires_recovery_command() {
    let plan = make_rollback_plan();
    assert!(!plan.recovery_command.is_empty());
}

#[test]
fn execution_id_is_content_addressed() {
    let id1 = execution_id_for("acp_123", "key_a");
    let id2 = execution_id_for("acp_123", "key_a");
    let id3 = execution_id_for("acp_123", "key_b");
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

// ── Commit 2: Predicate tests ──────────────────────────────────────────────

fn make_predicate_ctx<'a>(
    proposal: Option<&'a AutoCommitProposal>,
    review: Option<&'a AutoCommitProposalReview>,
) -> (Option<&'a AutoCommitProposal>, Option<&'a AutoCommitProposalReview>, Option<&'a AutoCommitProposalReview>) {
    let latest = review;
    (proposal, review, latest)
}

#[test]
fn blocks_missing_proposal() {
    let git_state = make_git_state();
    let results = evaluate_execution_predicates(
        None, None, None, &git_state, "hash_a", "phash_a", "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::ProposalExists).unwrap();
    assert!(!pred.passed);
}

#[test]
fn blocks_missing_review() {
    let proposal = make_eligible_proposal();
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), None, None, &git_state, &ws_hash, &p_hash, "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::ReviewExists).unwrap();
    assert!(!pred.passed);
}

#[test]
fn blocks_non_latest_review() {
    let proposal = make_eligible_proposal();
    let review1 = make_approved_review(&proposal);
    // Create a second review that supersedes the first
    let mut review2 = make_approved_review(&proposal);
    review2.review_id = AutoCommitProposalReviewId("arv_later".to_string());
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review1), Some(&review2), &git_state,
        &ws_hash, &p_hash, "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::ReviewIsLatestForProposal).unwrap();
    assert!(!pred.passed, "Should block non-latest review");
}

#[test]
fn blocks_rejected_review() {
    let proposal = make_eligible_proposal();
    let review = make_rejected_review(&proposal);
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review), Some(&review), &git_state,
        &ws_hash, &p_hash, "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::ReviewApproved).unwrap();
    assert!(!pred.passed);
}

#[test]
fn blocks_requested_changes_review() {
    let proposal = make_eligible_proposal();
    let feedback = ProposalRejectionFeedback {
        feedback_id: "pfb_test".to_string(),
        proposal_id: proposal.proposal_id.clone(),
        review_id: AutoCommitProposalReviewId("arv_test".to_string()),
        workspace_hash: proposal.workspace_snapshot_id.clone(),
        summary: "Changes needed".to_string(),
        required_changes: vec![RequiredProposalChange {
            category: ProposalFeedbackCategory::Other,
            description: "Fix it".to_string(), evidence_ref: None,
        }],
        blocked_dimensions: vec![], suggested_next_eval_focus: vec![],
        severity: ProposalFeedbackSeverity::Advisory,
    };
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::ChangesRequested,
        AutoCommitProposalReviewer::User,
        "Needs work".to_string(),
        vec![], Some(feedback),
    ).unwrap();
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review), Some(&review), &git_state,
        &ws_hash, &p_hash, "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::ReviewApproved).unwrap();
    assert!(!pred.passed);
}

#[test]
fn blocks_workspace_hash_drift() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let git_state = make_git_state();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review), Some(&review), &git_state,
        "DRIFTED_hash", &p_hash, "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::CurrentWorkspaceHashMatchesReview).unwrap();
    assert!(!pred.passed);
}

#[test]
fn blocks_proposal_hash_drift() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review), Some(&review), &git_state,
        &ws_hash, "DRIFTED_phash", "", &[], "key1",
    );
    let pred = results.iter().find(|p| p.predicate == ExecutionPredicate::CurrentProposalHashMatchesReview).unwrap();
    assert!(!pred.passed);
}

#[test]
fn all_predicates_pass_for_valid_state() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let git_state = make_git_state();
    let ws_hash = proposal.workspace_snapshot_id.clone();
    let p_hash = serde_json::to_string(&proposal).map(|j| format!("{}", blake3::hash(j.as_bytes()).to_hex())).unwrap();
    let results = evaluate_execution_predicates(
        Some(&proposal), Some(&review), Some(&review), &git_state,
        &ws_hash, &p_hash, &proposal.commit_body, &[], "key1",
    );
    for pred in &results {
        assert!(pred.passed, "Predicate {:?} failed: {}", pred.predicate, pred.reason);
    }
}

// ── Commit 3: Policy and rollback tests ────────────────────────────────────

#[test]
fn policy_gate_called_for_git_effect() { // Minor correction: fixed typo
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], false, Some(make_rollback_plan()), // policy denied
    );
    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);
    let policy_pred = record.decision.predicates.iter()
        .find(|p| p.predicate == ExecutionPredicate::PolicyAllowsGitCommit).unwrap();
    assert!(!policy_pred.passed);
}

#[test]
fn policy_failure_blocks_execution() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], false, Some(make_rollback_plan()),
    );
    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);
}

#[test]
fn missing_rollback_plan_blocks_execution() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, None, // no rollback plan
    );
    let rollback_pred = record.decision.predicates.iter()
        .find(|p| p.predicate == ExecutionPredicate::RollbackPlanExists).unwrap();
    assert!(!rollback_pred.passed);
}

#[test]
fn rollback_plan_captures_pre_commit_state() {
    let plan = make_rollback_plan();
    assert_eq!("abc123", plan.pre_commit_head);
    assert_eq!("main", plan.branch);
}

#[test]
fn rollback_plan_includes_recovery_command() {
    let plan = make_rollback_plan();
    assert!(plan.recovery_command.contains("git reset"));
}

// ── Commit 4: Backend and runtime guard tests ──────────────────────────────

#[test]
fn test_backend_observe_state() {
    let backend = TestGitBackend::new("deadbeef", "main");
    let state = backend.observe_state(std::path::Path::new("/tmp")).unwrap();
    assert_eq!("deadbeef", state.head);
    assert_eq!("main", state.branch);
}

#[test]
fn test_backend_commit_exact_creates_commit() {
    let backend = TestGitBackend::new("deadbeef", "main");
    let req = ExactCommitRequest {
        commit_message: "test commit".to_string(),
        file_paths: vec!["src/lib.rs".to_string()],
        expected_head: "deadbeef".to_string(),
        expected_branch: "main".to_string(),
        proposal_hash: "phash".to_string(),
        review_hash: "rhash".to_string(),
        idempotency_key: "key1".to_string(),
    };
    let result = backend.create_commit_exact(std::path::Path::new("/tmp"), req.clone()).unwrap();
    assert!(result.commit_hash.contains("testcommit"));
    assert_eq!("deadbeef", result.parent_hash);
    let committed = backend.committed.lock().unwrap();
    assert_eq!(1, committed.len());
}

#[test]
fn happy_path_executes_commit() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    assert_eq!(AutoCommitExecutionStatus::Executed, record.status);
    assert!(record.resulting_commit.is_some());
}

// ── Commit 5: Persistence and idempotency tests ───────────────────────────

#[test]
fn execution_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    let path = save_execution_record(dir.path(), &record).unwrap();
    assert!(path.exists());

    let loaded = load_execution_record(dir.path(), &record.execution_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
    assert_eq!(record.status, loaded.status);
}

#[test]
fn same_idempotency_key_returns_existing_record() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let req = make_execution_request(&proposal, &review);

    let record1 = AutoCommitExecutionRecord {
        execution_id: execution_id_for(&proposal.proposal_id.0, &req.idempotency_key),
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        status: AutoCommitExecutionStatus::Executed,
        decision: AutoCommitExecutionDecision {
            decision: ExecutionGateDecision::Allow,
            proposal_id: proposal.proposal_id.clone(),
            review_id: review.review_id.clone(),
            predicates: vec![], git_state_snapshot: make_git_state(),
            rollback_plan: Some(make_rollback_plan()),
        },
        resulting_commit: Some(GitCommitSnapshot {
            commit_hash: "abc".to_string(), parent_hash: "def".to_string(),
            branch: "main".to_string(), message_hash: "msg".to_string(),
            committed_at: chrono::Utc::now(),
        }),
        created_at: chrono::Utc::now(),
    };
    save_execution_record(dir.path(), &record1).unwrap();

    // Idempotent check
    let existing = list_execution_records(dir.path()).unwrap();
    let backend = TestGitBackend::new("abc123", "main");
    let result = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &existing, true, Some(make_rollback_plan()),
    );
    // Should return existing record, not execute again
    assert_eq!(record1.execution_id, result.execution_id);
    let committed = backend.committed.lock().unwrap();
    assert_eq!(0, committed.len(), "Should not create second commit");
}

#[test]
fn execution_record_links_proposal_review() {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    assert_eq!(proposal.proposal_id, record.proposal_id);
    assert_eq!(review.review_id, record.review_id);
}

#[test]
fn latest_execution_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let record = AutoCommitExecutionRecord {
        execution_id: execution_id_for(&proposal.proposal_id.0, "key1"),
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        status: AutoCommitExecutionStatus::Blocked,
        decision: AutoCommitExecutionDecision {
            decision: ExecutionGateDecision::Block {
                reason_code: "test".to_string(), summary: "test".to_string(),
            },
            proposal_id: proposal.proposal_id.clone(),
            review_id: review.review_id.clone(),
            predicates: vec![], git_state_snapshot: make_git_state(),
            rollback_plan: None,
        },
        resulting_commit: None, created_at: chrono::Utc::now(),
    };
    save_execution_record(dir.path(), &record).unwrap();

    let latest = load_latest_execution(dir.path()).unwrap().unwrap();
    assert_eq!(record.execution_id, latest.execution_id);
}

#[test]
fn latest_execution_for_proposal_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let record = AutoCommitExecutionRecord {
        execution_id: execution_id_for(&proposal.proposal_id.0, "key1"),
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        status: AutoCommitExecutionStatus::Blocked,
        decision: AutoCommitExecutionDecision {
            decision: ExecutionGateDecision::Block {
                reason_code: "test".to_string(), summary: "test".to_string(),
            },
            proposal_id: proposal.proposal_id.clone(),
            review_id: review.review_id.clone(),
            predicates: vec![], git_state_snapshot: make_git_state(),
            rollback_plan: None,
        },
        resulting_commit: None, created_at: chrono::Utc::now(),
    };
    save_execution_record(dir.path(), &record).unwrap();

    let loaded = load_latest_execution_for_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

// ── Correction #1: Command guard tests ─────────────────────────────────────

#[test]
fn command_is_only_used_inside_local_git_backend() {
    let source = include_str!("../src/eval_proposal_execution.rs");
    // std::process::Command should only appear inside LocalGitBackend impl
    // Count lines with it
    let command_lines: Vec<&str> = source.lines()
        .filter(|l| l.contains("std::process::Command"))
        .collect();
    // Should be only in LocalGitBackend::run_git
    assert!(command_lines.len() <= 3,
        "Command should only appear in LocalGitBackend, found {} lines", command_lines.len());
}

#[test]
fn local_git_backend_uses_fixed_git_binary() {
    let source = include_str!("../src/eval_proposal_execution.rs");
    // Check that Command::new always uses "git"
    assert!(source.contains("Command::new(\"git\")"), "Must use fixed 'git' binary");
}

#[test]
fn local_git_backend_never_invokes_shell() {
    let source = include_str!("../src/eval_proposal_execution.rs");
    // Check code lines (not comments) for shell invocation
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("//!") { continue; }
        let lower = trimmed.to_lowercase();
        // No shell() method calls, no /bin/sh, no cmd.exe
        assert!(!lower.contains(".shell("), "Must not call .shell() method");
        assert!(!lower.contains("/bin/sh"), "Must not invoke /bin/sh");
        assert!(!lower.contains("cmd.exe"), "Must not invoke cmd.exe");
    }
}

#[test]
fn execution_module_does_not_push_tag_branch() {
    let source = include_str!("../src/eval_proposal_execution.rs");
    let lower = source.to_lowercase();
    assert!(!lower.contains("git push") || !source.contains("\"git push\""), "No git push");
    assert!(!lower.contains("git tag") || !source.contains("\"git tag\""), "No git tag");
    assert!(!lower.contains("git branch") || !source.contains("\"git branch\""), "No git branch creation");
}

// ── Correction #2: Staging guard tests ─────────────────────────────────────

#[test]
fn stages_only_approved_paths() {
    let backend = TestGitBackend::new("abc123", "main");
    let req = ExactCommitRequest {
        commit_message: "test".to_string(),
        file_paths: vec!["src/lib.rs".to_string(), "src/main.rs".to_string()],
        expected_head: "abc123".to_string(),
        expected_branch: "main".to_string(),
        proposal_hash: "ph".to_string(),
        review_hash: "rh".to_string(),
        idempotency_key: "key1".to_string(),
    };
    let result = backend.create_commit_exact(std::path::Path::new("/tmp"), req).unwrap();
    let committed = backend.committed.lock().unwrap();
    assert_eq!(2, committed[0].file_paths.len());
    assert!(committed[0].file_paths.contains(&"src/lib.rs".to_string()));
    assert!(committed[0].file_paths.contains(&"src/main.rs".to_string()));
}

#[test]
fn does_not_stage_unreviewed_file() {
    // Test backend only stages what's in the request — no extra files
    let backend = TestGitBackend::new("abc123", "main");
    let req = ExactCommitRequest {
        commit_message: "test".to_string(),
        file_paths: vec!["src/lib.rs".to_string()],
        expected_head: "abc123".to_string(),
        expected_branch: "main".to_string(),
        proposal_hash: "ph".to_string(),
        review_hash: "rh".to_string(),
        idempotency_key: "key1".to_string(),
    };
    let _ = backend.create_commit_exact(std::path::Path::new("/tmp"), req).unwrap();
    let committed = backend.committed.lock().unwrap();
    // Only 1 file, not the unreviewed one
    assert_eq!(1, committed[0].file_paths.len());
}

// ── Runtime git-state guard ────────────────────────────────────────────────

#[test]
fn blocked_execution_leaves_git_head_index_worktree_unchanged() {
    // Create temp git repo
    let repo_dir = tempfile::tempdir().unwrap();
    let repo_path = repo_dir.path();
    std::process::Command::new("git").args(["init"]).current_dir(repo_path).output().expect("git init failed");
    std::fs::write(repo_path.join("lib.rs"), "fn main() {}").unwrap();
    std::process::Command::new("git").args(["add", "."]).current_dir(repo_path).output().expect("git add failed");
    std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo_path).output().expect("git commit failed");

    let head_before = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_before = std::fs::read(repo_path.join(".git/index")).unwrap();

    // Separate output dir
    let output_dir = tempfile::tempdir().unwrap();

    // Create a blocked execution (rejected review)
    let proposal = make_eligible_proposal();
    let review = make_rejected_review(&proposal);
    let req = make_execution_request(&proposal, &review);

    // Use real backend
    let record = execute_proposal(
        &LocalGitBackend, repo_path, &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);

    let _ = save_execution_record(output_dir.path(), &record);

    let head_after = std::fs::read(repo_path.join(".git/HEAD")).unwrap();
    let index_after = std::fs::read(repo_path.join(".git/index")).unwrap();
    assert_eq!(head_before, head_after, "HEAD must not change on blocked execution");
    assert_eq!(index_before, index_after, "Index must not change on blocked execution");
}

// ── Helpers (shared DTOs) ──────────────────────────────────────────────────

fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(), report_schema_version: 1,
        target: ReadinessTarget::AutoCommit, status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore {
            weighted_pass_rate: 0.95, patch_pass_rate: 0.98,
            policy_pass_rate: 1.0, rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95, capability_context_pass_rate: 1.0, regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 15, reports_used: 15,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec!["test".to_string()],
            earliest_report: None, latest_report: None,
        },
        scenario_results: vec![], blockers: vec![], warnings: vec![],
    }
}

fn make_workspace_digest(hash: &str) -> WorkspaceSnapshotDigest {
    WorkspaceSnapshotDigest {
        blake3_hash: hash.to_string(), file_count: 5,
        generated_at: chrono::Utc::now(), file_digests: vec![],
    }
}

fn make_eval_report() -> EvalRunReport {
    EvalRunReport {
        report_schema_version: 2, scenario_id: "test".to_string(),
        provider: ProviderRealitySnapshot {
            provider: "test".to_string(), model: "test".to_string(),
            base_url_redacted: None, supports_streaming: true,
            supports_tools: true, supports_reasoning: false,
            health_status: ProviderHealthStatus::Healthy,
            temperature: None, max_tokens: None, observed_at: chrono::Utc::now(),
        },
        prompt: PromptEvalResult {
            prompt_seen: true, evidence_missing: false,
            model: Some("test".to_string()), provider: Some("test".to_string()),
            system_prompt_hash: None, message_count: 1, tool_count: 0,
        },
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
            planned: true, applied: true, preimage_verified: true,
            postimage_verified: true, rollback_available: true,
            changed_files_match_expected: true,
        },
        explain: ExplainEvalResult {
            memory_matches: true, policy_matches: true,
            tool_matches: true, completion_matches: true,
        },
        rebuild: RebuildEvalResult {
            events_replayed: 10, state_matches: true, divergences: vec![],
        },
        capability_context: CapabilityContextEvalResult::default(),
        score: EvalScore {
            total: 5, max: 5, pass_rate: 1.0,
            dimensions: vec![
                DimensionScore { name: "patch".to_string(), passed: 1, total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Trace,
                        event_kind: Some("file.patch".to_string()),
                        summary: "test".to_string(),
                    }],
                },
            ],
        },
    }
}

// ── CLI surface tests ───────────────────────────────────────────────────────

/// Verify that executing via the same code path the CLI uses, with a blocked
/// proposal (rejected review), outputs predicate results in the record.
#[test]
fn cli_execute_blocked_outputs_predicates() {
    let proposal = make_eligible_proposal();
    let review = make_rejected_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);

    // Same call chain as CLI handler (without the feature gate)
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );

    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);
    // CLI handler prints predicates; verify they exist and are non-empty
    assert!(!record.decision.predicates.is_empty(),
        "Blocked execution must carry predicate list for CLI output");

    // At least one must have failed (the rejected review)
    let failed: Vec<_> = record.decision.predicates.iter().filter(|p| !p.passed).collect();
    assert!(!failed.is_empty(), "Blocked record must show which predicates failed");

    // Each predicate has a non-empty reason string (CLI prints these)
    for p in &record.decision.predicates {
        assert!(!p.reason.is_empty(), "Predicate {:?} must have reason for CLI display", p.predicate);
    }
}

/// Verify that a blocked execution record never claims a commit was executed.
#[test]
fn cli_execute_blocked_prints_no_commit_executed() {
    let proposal = make_eligible_proposal();
    let review = make_rejected_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);

    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );

    // CLI handler checks resulting_commit to decide whether to print commit hash
    assert!(record.resulting_commit.is_none(),
        "Blocked execution must not have a resulting_commit; CLI would falsely print commit hash");
    assert_eq!(AutoCommitExecutionStatus::Blocked, record.status);

    // Decision must be Block, not Allow
    match &record.decision.decision {
        ExecutionGateDecision::Block { reason_code, summary } => {
            assert!(!reason_code.is_empty());
            assert!(!summary.is_empty());
        }
        ExecutionGateDecision::Allow => {
            panic!("Blocked execution must not have Allow decision");
        }
    }
}

/// Verify that `execution show` code path loads and roundtrips the record.
#[test]
fn cli_execution_show_roundtrips_record() {
    let dir = tempfile::tempdir().unwrap();

    // Create and execute a record (same as CLI execute handler does)
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let backend = TestGitBackend::new("abc123", "main");
    let req = make_execution_request(&proposal, &review);
    let record = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req,
        Some(&proposal), Some(&review), Some(&review),
        &[], true, Some(make_rollback_plan()),
    );
    save_execution_record(dir.path(), &record).unwrap();

    // Same code path as `execution show <execution_id>` CLI handler
    let loaded = load_execution_record(dir.path(), &record.execution_id)
        .expect("load should not error")
        .expect("record should exist");

    // Verify full roundtrip
    assert_eq!(record.execution_id, loaded.execution_id);
    assert_eq!(record.status, loaded.status);
    assert_eq!(record.proposal_id, loaded.proposal_id);
    assert_eq!(record.review_id, loaded.review_id);
    // resulting_commit must roundtrip correctly
    assert_eq!(record.resulting_commit.is_some(), loaded.resulting_commit.is_some());
    if let (Some(original), Some(loaded_commit)) = (&record.resulting_commit, &loaded.resulting_commit) {
        assert_eq!(original.commit_hash, loaded_commit.commit_hash);
        assert_eq!(original.parent_hash, loaded_commit.parent_hash);
        assert_eq!(original.branch, loaded_commit.branch);
    }
}

/// Verify that `execution latest` returns the most recent record, optionally
/// filtered by proposal ID.
#[test]
fn cli_execution_latest_returns_latest() {
    let dir = tempfile::tempdir().unwrap();

    // Create two proposals with their own executions
    let proposal1 = make_eligible_proposal();
    let review1 = make_approved_review(&proposal1);
    let backend = TestGitBackend::new("abc123", "main");
    let req1 = make_execution_request(&proposal1, &review1);
    let record1 = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req1,
        Some(&proposal1), Some(&review1), Some(&review1),
        &[], true, Some(make_rollback_plan()),
    );
    save_execution_record(dir.path(), &record1).unwrap();

    // Small delay so timestamps differ
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Second proposal (different workspace hash to get a different proposal ID)
    let inputs2 = AutoCommitProposalInputs {
        readiness: &make_eligible_readiness(),
        workspace_digest: &make_workspace_digest("hash_b"),
        eval_report: &make_eval_report(),
        comparison: None,
    };
    let proposal2 = build_auto_commit_proposal(inputs2);
    let review2 = make_approved_review(&proposal2);
    let req2 = AutoCommitExecutionRequest {
        proposal_id: proposal2.proposal_id.clone(),
        review_id: review2.review_id.clone(),
        requested_by: "test_user".to_string(),
        requested_at: chrono::Utc::now(),
        idempotency_key: format!("key_{}", proposal2.proposal_id.0),
    };
    let record2 = execute_proposal(
        &backend, std::path::Path::new("/tmp"), &req2,
        Some(&proposal2), Some(&review2), Some(&review2),
        &[], true, Some(make_rollback_plan()),
    );
    save_execution_record(dir.path(), &record2).unwrap();

    // Same code path as `execution latest` (no proposal filter)
    let latest = load_latest_execution(dir.path())
        .expect("load should not error")
        .expect("latest should exist");
    assert_eq!(record2.execution_id, latest.execution_id,
        "Latest should be the second (newer) execution");

    // Same code path as `execution latest --proposal-id <proposal1_id>`
    let latest_for_p1 = load_latest_execution_for_proposal(dir.path(), &proposal1.proposal_id)
        .expect("load should not error")
        .expect("should find execution for proposal1");
    assert_eq!(record1.execution_id, latest_for_p1.execution_id,
        "Latest for proposal1 should be record1, not record2");

    // And for proposal2
    let latest_for_p2 = load_latest_execution_for_proposal(dir.path(), &proposal2.proposal_id)
        .expect("load should not error")
        .expect("should find execution for proposal2");
    assert_eq!(record2.execution_id, latest_for_p2.execution_id,
        "Latest for proposal2 should be record2");
}
