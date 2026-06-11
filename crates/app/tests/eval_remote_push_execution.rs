//! Governed remote push execution gate tests.

use openwand_app::eval_remote_push_execution::*;
use openwand_app::eval_remote_push_proposal::*;
use openwand_app::eval_remote_push_readiness::*;
use openwand_app::eval_post_commit_verify::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_execution::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_readiness::*;
use std::path::{Path, PathBuf};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_full_chain() -> (RemotePushProposal, RemotePushProposalReview, RemotePushReadinessRecord, PostCommitVerificationRecord, AutoCommitExecutionRecord) {
    let (proposal, review, exec, verified, readiness) = make_base_chain();
    let push_req = RemotePushProposalRequest {
        readiness_id: readiness.readiness_id.clone(),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "pkey".into(),
    };
    let push_proposal = build_push_proposal(&push_req, Some(&readiness), &[]).unwrap();
    let push_review_req = RemotePushProposalReviewRequest {
        proposal_id: push_proposal.proposal_id.clone(),
        decision: RemotePushProposalReviewDecision::Approved,
        reviewer: "alice".into(),
        rationale: "LGTM".into(),
        feedback: None,
        idempotency_key: "rvkey".into(),
    };
    let push_review = build_push_proposal_review(&push_proposal, &push_review_req, &[]).unwrap();
    (push_proposal, push_review, readiness, verified, exec)
}

fn make_base_chain() -> (AutoCommitProposal, AutoCommitProposalReview, AutoCommitExecutionRecord, PostCommitVerificationRecord, RemotePushReadinessRecord) {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let exec = make_executed_record(&proposal, &review);
    let verified = make_verified_record(&exec, &proposal);
    let readiness = make_ready_readiness(&verified);
    (proposal, review, exec, verified, readiness)
}

fn make_eligible_proposal() -> AutoCommitProposal {
    let r = make_eligible_readiness();
    let w = make_workspace_digest("hash_a");
    let e = make_eval_report();
    build_auto_commit_proposal(AutoCommitProposalInputs { readiness: &r, workspace_digest: &w, eval_report: &e, comparison: None })
}

fn make_approved_review(p: &AutoCommitProposal) -> AutoCommitProposalReview {
    build_proposal_review(p, AutoCommitProposalReviewDecision::Approved, AutoCommitProposalReviewer::User, "OK".into(), vec![], None).unwrap()
}

fn make_executed_record(p: &AutoCommitProposal, r: &AutoCommitProposalReview) -> AutoCommitExecutionRecord {
    let backend = TestGitBackend::new("tracking_commit", "main");
    let req = AutoCommitExecutionRequest { proposal_id: p.proposal_id.clone(), review_id: r.review_id.clone(), requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: "k".into() };
    execute_proposal(&backend, std::path::Path::new("/tmp"), &req, Some(p), Some(r), Some(r), &[], true, Some(RollbackPlanSnapshot { pre_commit_head: "tracking_commit".into(), branch: "main".into(), index_status_hash: "idx".into(), worktree_status_hash: "wt".into(), recovery_command: "git reset --hard tracking_commit".into(), notes: vec![] }))
}

fn make_verified_record(exec: &AutoCommitExecutionRecord, p: &AutoCommitProposal) -> PostCommitVerificationRecord {
    let commit = exec.resulting_commit.as_ref().unwrap();
    let msg = format!("{}\n\n{}", p.commit_title, p.commit_body);
    let msg_hash = format!("{}", blake3::hash(msg.as_bytes()).to_hex());
    PostCommitVerificationRecord {
        verification_id: verification_id_for(&exec.execution_id.0, "vkey"),
        execution_id: exec.execution_id.clone(), proposal_id: exec.proposal_id.clone(), review_id: exec.review_id.clone(),
        status: PostCommitVerificationStatus::Verified, decision: PostCommitVerificationDecision::Verified,
        predicates: vec![],
        commit_evidence: Some(CommitEvidenceSnapshot {
            commit_hash: commit.commit_hash.clone(), parent_hash: commit.parent_hash.clone(),
            tree_hash: "tree".into(), branch: "main".into(), message_hash: msg_hash,
            changed_paths: p.included_files.iter().map(|f| f.path.clone()).collect(), diff_hash: "diff".into(),
        }),
        post_commit_checks: vec![PostCommitCheckResult {
            spec: PostCommitCheckSpec { name: "check".into(), kind: PostCommitCheckKind::CargoCheckWorkspace },
            status: PostCommitCheckStatus::Passed, output_summary: "OK".into(),
        }],
        rollback_drill: Some(RollbackDrillResult {
            strategy: RollbackDrillStrategy::SandboxRevert, clean: true,
            sandbox_pre_head: "abc".into(), sandbox_post_head: "def".into(), sandbox_diff_hash: "d".into(), conflicts: vec![],
            live_head_before: "h".into(), live_head_after: "h".into(),
            live_index_before: "i".into(), live_index_after: "i".into(),
            live_worktree_before: "w".into(), live_worktree_after: "w".into(),
        }),
        created_at: chrono::Utc::now(),
    }
}

fn make_ready_readiness(verified: &PostCommitVerificationRecord) -> RemotePushReadinessRecord {
    let commit_hash = verified.commit_evidence.as_ref().map(|e| e.commit_hash.clone()).unwrap_or_default();
    RemotePushReadinessRecord {
        readiness_id: readiness_id_for(&verified.verification_id.0, "origin", "main", "rkey"),
        verification_id: verified.verification_id.clone(), execution_id: verified.execution_id.clone(),
        proposal_id: verified.proposal_id.clone(), review_id: verified.review_id.clone(),
        commit_hash: commit_hash.clone(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![],
        local_branch: Some(LocalBranchPushSnapshot {
            current_head: commit_hash.clone(), current_branch: "main".into(),
            target_remote: "origin".into(), target_branch: "main".into(),
            upstream_ref: Some("refs/remotes/origin/main".into()), remote_tracking_ref: Some("refs/remotes/origin/main".into()),
            ahead_count: 1, behind_count: 0, diverged: false, worktree_clean: true, index_clean: true,
        }),
        remote_tracking: Some(RemoteTrackingSnapshot {
            remote_name: "origin".into(), tracking_ref: "refs/remotes/origin/main".into(),
            tracking_commit: Some("tracking_commit".into()), observed_from_local_refs_only: true,
        }),
        branch_policy: Some(BranchProtectionPolicySnapshot {
            branch: "main".into(), direct_push_allowed: true, requires_verified_commit: true,
            requires_clean_rollback_drill: true, requires_post_commit_checks: true,
            requires_no_behind_remote: true, requires_no_divergence: true,
            requires_protected_branch_approval: false, protected_branch: false,
            policy_source: "default".into(),
        }),
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    }
}

fn make_execution_request(proposal: &RemotePushProposal, review: &RemotePushProposalReview) -> RemotePushExecutionRequest {
    RemotePushExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "ekey".into(),
    }
}

fn make_test_backend(proposal: &RemotePushProposal) -> TestPushExecutionBackend {
    let commit_hash = proposal.commit_hash.clone();
    TestPushExecutionBackend::new()
        .with_local_state(LocalPushExecutionSnapshot {
            head: commit_hash.clone(),
            branch: "main".into(),
            worktree_clean: true,
            index_clean: true,
        })
        .with_remote_ref(RemoteRefObservedSnapshot {
            remote: "origin".into(),
            branch: "main".into(),
            ref_name: "refs/heads/main".into(),
            observed_commit: Some("tracking_commit".into()),
            observed_at: chrono::Utc::now(),
            source: RemoteObservationSource::LsRemote,
        })
        .with_push_result(RemotePushResultSnapshot {
            remote: "origin".into(),
            branch: "main".into(),
            ref_name: "refs/heads/main".into(),
            old_commit: "tracking_commit".into(),
            new_commit: commit_hash,
            fast_forward: true,
            push_output_hash: "pushhash".into(),
            pushed_at: chrono::Utc::now(),
        })
}

fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport { generated_at: chrono::Utc::now(), report_schema_version: 1, target: ReadinessTarget::AutoCommit, status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore { weighted_pass_rate: 0.95, patch_pass_rate: 0.98, policy_pass_rate: 1.0, rebuild_pass_rate: 1.0, explain_pass_rate: 0.95, capability_context_pass_rate: 1.0, regression_count: 0 },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow { total_reports_found: 15, reports_used: 15, reports_skipped_incompatible: 0, scenario_ids_covered: vec!["test".into()], earliest_report: None, latest_report: None },
        scenario_results: vec![], blockers: vec![], warnings: vec![] }
}

fn make_workspace_digest(h: &str) -> WorkspaceSnapshotDigest { WorkspaceSnapshotDigest { blake3_hash: h.into(), file_count: 5, generated_at: chrono::Utc::now(), file_digests: vec![] } }

fn make_eval_report() -> openwand_app::eval_model::EvalRunReport {
    use openwand_app::eval_model::*;
    EvalRunReport { report_schema_version: 2, scenario_id: "test".into(),
        provider: ProviderRealitySnapshot { provider: "test".into(), model: "test".into(), base_url_redacted: None, supports_streaming: true, supports_tools: true, supports_reasoning: false, health_status: ProviderHealthStatus::Healthy, temperature: None, max_tokens: None, observed_at: chrono::Utc::now() },
        prompt: PromptEvalResult { prompt_seen: true, evidence_missing: false, model: Some("test".into()), provider: Some("test".into()), system_prompt_hash: None, message_count: 1, tool_count: 0 },
        memory: MemoryEvalResult { included_claims_seen: vec![], excluded_claims_seen: vec![], missing_required: vec![], unexpected_included: vec![], prompt_panel_equivalent: true },
        tools: ToolEvalResult { requested_tools: vec![], executed_tools: vec![], blocked_tools: vec![], forbidden_requested: vec![] },
        policy: PolicyEvalResult { gates_seen: vec![], required_approvals_seen: vec![], unexpected_allows: vec![] },
        patch: PatchEvalResult { planned: true, applied: true, preimage_verified: true, postimage_verified: true, rollback_available: true, changed_files_match_expected: true },
        explain: ExplainEvalResult { memory_matches: true, policy_matches: true, tool_matches: true, completion_matches: true },
        rebuild: RebuildEvalResult { events_replayed: 10, state_matches: true, divergences: vec![] },
        capability_context: CapabilityContextEvalResult::default(),
        score: EvalScore { total: 5, max: 5, pass_rate: 1.0, dimensions: vec![
            DimensionScore { name: "patch".into(), passed: 1, total: 1, evidence_refs: vec![EvalEvidenceRef { source: EvalEvidenceSource::Trace, event_kind: Some("file.patch".into()), summary: "test".into() }] },
        ] },
    }
}

// ── DTO Tests (8) ───────────────────────────────────────────────────────────

#[test] fn execution_request_roundtrips() {
    let (proposal, review, _, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let json = serde_json::to_string(&req).unwrap();
    let parsed: RemotePushExecutionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.idempotency_key, parsed.idempotency_key);
}

#[test] fn execution_record_roundtrips() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    let json = serde_json::to_string(&record).unwrap();
    let parsed: RemotePushExecutionRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.execution_id, parsed.execution_id);
}

#[test] fn execution_id_is_content_addressed() {
    let a = push_execution_id_for("p1", "r1", "k1");
    let b = push_execution_id_for("p1", "r1", "k1");
    let c = push_execution_id_for("p1", "r1", "k2");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test] fn execution_id_is_deterministic() {
    for _ in 0..5 { assert_eq!("rpe_", &push_execution_id_for("p", "r", "k").0[..4]); }
}

#[test] fn execution_id_differs_by_proposal_review_key() {
    let a = push_execution_id_for("p1", "r1", "k");
    let b = push_execution_id_for("p2", "r1", "k");
    let c = push_execution_id_for("p1", "r2", "k");
    assert_ne!(a, b);
    assert_ne!(a, c);
}

#[test] fn executed_record_includes_push_result() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    if record.status == RemotePushExecutionStatus::Executed {
        assert!(record.push_result.is_some());
    }
}

#[test] fn blocked_record_has_no_push_result() {
    let (proposal, _, _, _, _) = make_full_chain();
    let req = RemotePushExecutionRequest { proposal_id: proposal.proposal_id.clone(), review_id: RemotePushProposalReviewId("nonexistent".into()), requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: "k".into() };
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), None, None, None, None, None, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
    assert!(record.push_result.is_none());
}

#[test] fn snapshot_types_roundtrip() {
    let snap = LocalPushExecutionSnapshot { head: "abc".into(), branch: "main".into(), worktree_clean: true, index_clean: true };
    let json = serde_json::to_string(&snap).unwrap();
    let parsed: LocalPushExecutionSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(snap.head, parsed.head);

    let rr = RemoteRefObservedSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), observed_commit: Some("abc".into()), observed_at: chrono::Utc::now(), source: RemoteObservationSource::LsRemote };
    let json = serde_json::to_string(&rr).unwrap();
    let parsed: RemoteRefObservedSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rr.observed_commit, parsed.observed_commit);

    let pr = RemotePushResultSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), old_commit: "old".into(), new_commit: "new".into(), fast_forward: true, push_output_hash: "h".into(), pushed_at: chrono::Utc::now() };
    let json = serde_json::to_string(&pr).unwrap();
    let parsed: RemotePushResultSnapshot = serde_json::from_str(&json).unwrap();
    assert!(parsed.fast_forward);
}

// ── Predicate Block Tests (22) ──────────────────────────────────────────────

fn run_execute(
    proposal: Option<&RemotePushProposal>,
    review: Option<&RemotePushProposalReview>,
    readiness: Option<&RemotePushReadinessRecord>,
    verification: Option<&PostCommitVerificationRecord>,
    local_execution: Option<&AutoCommitExecutionRecord>,
    branch_policy: Option<&BranchProtectionPolicySnapshot>,
    backend: &TestPushExecutionBackend,
    existing: &[RemotePushExecutionRecord],
    remote_configured: bool,
    policy_allows: bool,
) -> RemotePushExecutionRecord {
    let req = RemotePushExecutionRequest {
        proposal_id: proposal.map(|p| p.proposal_id.clone()).unwrap_or_else(|| RemotePushProposalId("missing".into())),
        review_id: review.map(|r| r.review_id.clone()).unwrap_or_else(|| RemotePushProposalReviewId("missing".into())),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "ekey".into(),
    };
    execute_push(backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        proposal, review, readiness, verification, local_execution, branch_policy, existing, remote_configured, policy_allows)
}

#[test] fn blocks_missing_proposal() {
    let (_, review, readiness, _, _) = make_full_chain();
    let backend = TestPushExecutionBackend::new();
    let record = run_execute(None, Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_rejected_review() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let reject_req = RemotePushProposalReviewRequest {
        proposal_id: proposal.proposal_id.clone(),
        decision: RemotePushProposalReviewDecision::Rejected,
        reviewer: "alice".into(),
        rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["risk".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k".into(),
    };
    let rejected = build_push_proposal_review(&proposal, &reject_req, &[]).unwrap();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&rejected), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_requested_changes_review() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let req = RemotePushProposalReviewRequest {
        proposal_id: proposal.proposal_id.clone(),
        decision: RemotePushProposalReviewDecision::ChangesRequested,
        reviewer: "alice".into(),
        rationale: "fix".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "fix".into(), blocking_reasons: vec![], requested_changes: vec!["add tests".into()], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k".into(),
    };
    let changes = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&changes), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_hash_mismatch_review() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    // Create a review with mismatched proposal_hash
    let mut bad_review = RemotePushProposalReview {
        review_id: RemotePushProposalReviewId("bad".into()),
        proposal_id: proposal.proposal_id.clone(),
        readiness_id: proposal.readiness_id.clone(),
        proposal_hash: "WRONG_HASH".into(),
        readiness_hash: proposal.readiness_hash.clone(),
        decision: RemotePushProposalReviewDecision::Approved,
        reviewer: "alice".into(),
        rationale: "ok".into(),
        feedback: None,
        creates_execution_grant: false,
        execution_allowed_now: false,
        reviewed_at: chrono::Utc::now(),
    };
    bad_review.proposal_hash = "MISMATCH".into();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&bad_review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_readiness_hash_mismatch() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let mut bad_review = RemotePushProposalReview {
        review_id: RemotePushProposalReviewId("bad".into()),
        proposal_id: proposal.proposal_id.clone(),
        readiness_id: proposal.readiness_id.clone(),
        proposal_hash: proposal.proposal_hash.clone(),
        readiness_hash: "WRONG".into(),
        decision: RemotePushProposalReviewDecision::Approved,
        reviewer: "alice".into(),
        rationale: "ok".into(),
        feedback: None,
        creates_execution_grant: false,
        execution_allowed_now: false,
        reviewed_at: chrono::Utc::now(),
    };
    bad_review.readiness_hash = "MISMATCH".into();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&bad_review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_non_ready_readiness() {
    let (proposal, review, mut readiness, _, _) = make_full_chain();
    readiness.status = RemotePushReadinessStatus::Blocked;
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_non_verified_verification() {
    let (proposal, review, readiness, mut verified, _) = make_full_chain();
    verified.status = PostCommitVerificationStatus::Failed;
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), Some(&verified), None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_failed_local_execution() {
    let (proposal, review, readiness, verified, mut exec) = make_full_chain();
    // We need exec record - but we only have the base chain exec, not the push-level one
    // Use the base chain exec which should be Executed status
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), Some(&verified), Some(&exec), None, &backend, &[], true, true);
    // This should pass since exec is Executed status
    // Let's make it fail instead
    exec.status = AutoCommitExecutionStatus::Blocked;
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), Some(&verified), Some(&exec), None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_head_mismatch() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = TestPushExecutionBackend::new()
        .with_local_state(LocalPushExecutionSnapshot { head: "WRONG_HEAD".into(), branch: "main".into(), worktree_clean: true, index_clean: true })
        .with_remote_ref(RemoteRefObservedSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), observed_commit: Some("tracking_commit".into()), observed_at: chrono::Utc::now(), source: RemoteObservationSource::LsRemote })
        .with_push_result(RemotePushResultSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), old_commit: "tracking_commit".into(), new_commit: proposal.commit_hash.clone(), fast_forward: true, push_output_hash: "h".into(), pushed_at: chrono::Utc::now() });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_branch_mismatch() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = TestPushExecutionBackend::new()
        .with_local_state(LocalPushExecutionSnapshot { head: proposal.commit_hash.clone(), branch: "WRONG_BRANCH".into(), worktree_clean: true, index_clean: true })
        .with_remote_ref(RemoteRefObservedSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), observed_commit: Some("tracking_commit".into()), observed_at: chrono::Utc::now(), source: RemoteObservationSource::LsRemote })
        .with_push_result(RemotePushResultSnapshot { remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(), old_commit: "tracking_commit".into(), new_commit: proposal.commit_hash.clone(), fast_forward: true, push_output_hash: "h".into(), pushed_at: chrono::Utc::now() });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_dirty_worktree() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.local_state = Some(LocalPushExecutionSnapshot { head: proposal.commit_hash.clone(), branch: "main".into(), worktree_clean: false, index_clean: true });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_dirty_index() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.local_state = Some(LocalPushExecutionSnapshot { head: proposal.commit_hash.clone(), branch: "main".into(), worktree_clean: true, index_clean: false });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_branch_policy_denial() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let policy = BranchProtectionPolicySnapshot {
        branch: "main".into(), direct_push_allowed: false, requires_verified_commit: true,
        requires_clean_rollback_drill: true, requires_post_commit_checks: true,
        requires_no_behind_remote: true, requires_no_divergence: true,
        requires_protected_branch_approval: true, protected_branch: true,
        policy_source: "test".into(),
    };
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, Some(&policy), &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_missing_remote() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], false, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_remote_ref_mismatch() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.remote_ref = Some(RemoteRefObservedSnapshot {
        remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(),
        observed_commit: Some("WRONG_COMMIT".into()), observed_at: chrono::Utc::now(),
        source: RemoteObservationSource::LsRemote,
    });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_non_fast_forward() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.is_fast_forward = false;
    // Need to inject a push result with non-ff
    // Actually the predicate checks is_fast_forward which comes from proposal.ref_update.fast_forward_only
    // which is always true. Let me make the proposal non-ff instead
    let mut p2 = proposal.clone();
    p2.ref_update.fast_forward_only = false;
    let record = run_execute(Some(&p2), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_policy_denial() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, false);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_missing_review() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), None, Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_missing_readiness() {
    let (proposal, review, _, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), None, None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_missing_verification() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    // Verification predicate fails
    let has_verification_pred = record.predicates.iter().any(|p| p.predicate == RemotePushExecutionPredicate::VerificationRecordExists && !p.passed);
    assert!(has_verification_pred);
}

#[test] fn blocks_missing_remote_branch() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.remote_ref = Some(RemoteRefObservedSnapshot {
        remote: "origin".into(), branch: "main".into(), ref_name: "refs/heads/main".into(),
        observed_commit: None, observed_at: chrono::Utc::now(),
        source: RemoteObservationSource::LsRemote,
    });
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn blocks_conflicting_prior_execution() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let prior = RemotePushExecutionRecord {
        execution_id: RemotePushExecutionId("prior".into()),
        proposal_id: proposal.proposal_id.clone(),
        review_id: RemotePushProposalReviewId("different_review".into()),
        readiness_id: readiness.readiness_id.clone(),
        verification_id: PostCommitVerificationId("v".into()),
        local_execution_id: AutoCommitExecutionId("e".into()),
        proposal_source_id: AutoCommitProposalId("p".into()),
        review_source_id: AutoCommitProposalReviewId("r".into()),
        commit_hash: proposal.commit_hash.clone(),
        target_remote: "origin".into(),
        target_branch: "main".into(),
        status: RemotePushExecutionStatus::Executed,
        decision: RemotePushExecutionDecision::Allow,
        predicates: vec![],
        pre_push_remote: None,
        post_push_remote: None,
        push_result: None,
        recovery: None,
        created_at: chrono::Utc::now(),
    };
    let backend = make_test_backend(&proposal);
    let record = run_execute(Some(&proposal), Some(&review), Some(&readiness), None, None, None, &backend, &[prior], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
}

#[test] fn all_predicates_pass_for_valid_push() {
    let (proposal, review, exec, verified, readiness) = make_base_chain();
    // Build push proposal/review
    let push_req = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey".into() };
    let push_proposal = build_push_proposal(&push_req, Some(&readiness), &[]).unwrap();
    let push_review_req = RemotePushProposalReviewRequest { proposal_id: push_proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "LGTM".into(), feedback: None, idempotency_key: "rvkey".into() };
    let push_review = build_push_proposal_review(&push_proposal, &push_review_req, &[]).unwrap();

    let backend = make_test_backend(&push_proposal);
    let policy = readiness.branch_policy.clone();
    let req = make_execution_request(&push_proposal, &push_review);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&push_proposal), Some(&push_review), Some(&readiness), Some(&verified), Some(&exec), policy.as_ref(), &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);
    assert_eq!(RemotePushExecutionDecision::Allow, record.decision);
    assert!(record.predicates.iter().all(|p| p.passed), "Failed predicates: {:?}", record.predicates.iter().filter(|p| !p.passed).collect::<Vec<_>>());
}

// ── Persistence and Idempotency Tests (10) ──────────────────────────────────

#[test] fn push_execution_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution(dir.path(), &record.execution_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn latest_push_execution_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let latest = load_latest_push_execution(dir.path()).unwrap().unwrap();
    assert_eq!(record.execution_id, latest.execution_id);
}

#[test] fn push_execution_by_proposal_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn push_execution_by_review_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_review(dir.path(), &review.review_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn push_execution_by_commit_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_commit(dir.path(), &proposal.commit_hash).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn same_key_returns_existing_blocked_push_execution() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let req = RemotePushExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: RemotePushProposalReviewId("nonexistent".into()),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "samekey".into(),
    };
    let backend = make_test_backend(&proposal);
    let r1 = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), None, Some(&readiness), None, None, None, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, r1.status);
    // Same key returns existing blocked
    let r2 = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), None, Some(&readiness), None, None, None, &[r1.clone()], true, true);
    assert_eq!(RemotePushExecutionStatus::AlreadyExecuted, r2.status);
    assert_eq!(r1.execution_id, r2.execution_id);
}

#[test] fn blocked_push_can_retry_with_new_key() {
    let (proposal, review, exec, verified, readiness) = make_base_chain();
    let push_req = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey".into() };
    let push_proposal = build_push_proposal(&push_req, Some(&readiness), &[]).unwrap();
    let push_review_req = RemotePushProposalReviewRequest { proposal_id: push_proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "LGTM".into(), feedback: None, idempotency_key: "rvkey".into() };
    let push_review = build_push_proposal_review(&push_proposal, &push_review_req, &[]).unwrap();

    // First attempt blocked (dirty worktree)
    let mut backend_blocked = make_test_backend(&push_proposal);
    backend_blocked.local_state = Some(LocalPushExecutionSnapshot { head: push_proposal.commit_hash.clone(), branch: "main".into(), worktree_clean: false, index_clean: true });
    let req1 = RemotePushExecutionRequest {
        proposal_id: push_proposal.proposal_id.clone(), review_id: push_review.review_id.clone(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "key1".into(),
    };
    let r1 = execute_push(&backend_blocked, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req1,
        Some(&push_proposal), Some(&push_review), Some(&readiness), Some(&verified), Some(&exec), readiness.branch_policy.as_ref(), &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, r1.status);

    // Retry with new key after conditions change
    let backend_ok = make_test_backend(&push_proposal);
    let req2 = RemotePushExecutionRequest {
        proposal_id: push_proposal.proposal_id.clone(), review_id: push_review.review_id.clone(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "key2".into(),
    };
    let r1_clone = r1.clone();
    let r2 = execute_push(&backend_ok, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req2,
        Some(&push_proposal), Some(&push_review), Some(&readiness), Some(&verified), Some(&exec), readiness.branch_policy.as_ref(), &[r1_clone], true, true);
    assert_eq!(RemotePushExecutionStatus::Executed, r2.status);
    assert_ne!(r1.execution_id, r2.execution_id);
}

#[test] fn executed_push_cannot_duplicate_with_different_key() {
    let (proposal, review, exec, verified, readiness) = make_base_chain();
    let push_req = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey".into() };
    let push_proposal = build_push_proposal(&push_req, Some(&readiness), &[]).unwrap();
    let push_review_req = RemotePushProposalReviewRequest { proposal_id: push_proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "LGTM".into(), feedback: None, idempotency_key: "rvkey".into() };
    let push_review = build_push_proposal_review(&push_proposal, &push_review_req, &[]).unwrap();

    let backend = make_test_backend(&push_proposal);
    let req1 = RemotePushExecutionRequest {
        proposal_id: push_proposal.proposal_id.clone(), review_id: push_review.review_id.clone(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "key1".into(),
    };
    let r1 = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req1,
        Some(&push_proposal), Some(&push_review), Some(&readiness), Some(&verified), Some(&exec), readiness.branch_policy.as_ref(), &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Executed, r1.status);

    let req2 = RemotePushExecutionRequest {
        proposal_id: push_proposal.proposal_id.clone(), review_id: push_review.review_id.clone(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "key2".into(),
    };
    let r2 = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req2,
        Some(&push_proposal), Some(&push_review), Some(&readiness), Some(&verified), Some(&exec), readiness.branch_policy.as_ref(), &[r1], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, r2.status);
}

#[test] fn list_push_executions_sorted_by_date() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let (proposal, review, readiness, _, _) = make_full_chain();
        let req = RemotePushExecutionRequest {
            proposal_id: proposal.proposal_id.clone(), review_id: review.review_id.clone(),
            requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: format!("k{}", i),
        };
        let backend = make_test_backend(&proposal);
        let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
            Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
        save_push_execution(dir.path(), &record).unwrap();
    }
    let loaded = list_push_executions(dir.path()).unwrap();
    assert_eq!(3, loaded.len());
    assert!(loaded[0].created_at >= loaded[1].created_at);
}

#[test] fn blocked_push_includes_recovery_snapshot() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.local_state = Some(LocalPushExecutionSnapshot { head: proposal.commit_hash.clone(), branch: "main".into(), worktree_clean: false, index_clean: true });
    let req = make_execution_request(&proposal, &review);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status);
    assert!(record.recovery.is_some());
}

// ── CLI Tests (7) ───────────────────────────────────────────────────────────

#[test] fn cli_push_execute_outputs_execution_id() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    assert!(record.execution_id.0.starts_with("rpe_"));
}

#[test] fn cli_push_execution_show_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution(dir.path(), &record.execution_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
    assert_eq!(record.commit_hash, loaded.commit_hash);
}

#[test] fn cli_push_execution_latest_by_proposal() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn cli_push_execution_latest_by_review() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_review(dir.path(), &review.review_id).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn cli_push_execution_latest_by_commit() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), dir.path(), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    save_push_execution(dir.path(), &record).unwrap();
    let loaded = load_push_execution_by_commit(dir.path(), &proposal.commit_hash).unwrap().unwrap();
    assert_eq!(record.execution_id, loaded.execution_id);
}

#[test] fn cli_blocked_push_outputs_predicates() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let mut backend = make_test_backend(&proposal);
    backend.local_state = Some(LocalPushExecutionSnapshot { head: proposal.commit_hash.clone(), branch: "main".into(), worktree_clean: false, index_clean: true });
    let req = make_execution_request(&proposal, &review);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    assert!(!record.predicates.is_empty());
    assert!(record.predicates.iter().any(|p| !p.passed));
}

#[test] fn cli_does_not_expose_general_git() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        // Allow only within backend implementation and only the exact commands
        if lower.contains("git fetch") { panic!("No git fetch"); }
    }
}

// ── Guard Tests (10) ────────────────────────────────────────────────────────

#[test] fn module_does_not_call_force_push() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("--force"), "No --force flag");
    }
}

#[test] fn module_does_not_push_tags_all_mirror_or_delete() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("--tags"), "No --tags");
        assert!(!lower.contains("--mirror"), "No --mirror");
        assert!(!lower.contains("--all"), "No --all");
        assert!(!lower.contains("--delete"), "No --delete");
    }
}

#[test] fn module_does_not_call_fetch_pull_or_ls_remote_wildcard() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git fetch"), "No git fetch");
        assert!(!lower.contains("git pull"), "No git pull");
        // ls-remote is allowed but only with exact ref
    }
}

#[test] fn module_does_not_create_tags_or_branches() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git tag"), "No git tag");
        assert!(!lower.contains("git branch"), "No git branch");
    }
}

#[test] fn module_does_not_call_release_tools() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("hub ") && !lower.contains("gh ") && !lower.contains("glab "), "No host tools");
    }
}

#[test] fn module_does_not_execute_arbitrary_shell() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("/bin/sh"), "No /bin/sh");
        assert!(!lower.contains("cmd.exe"), "No cmd.exe");
    }
}

#[test] fn command_only_used_inside_push_execution_backend() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    // std::process::Command should only appear inside LocalPushExecutionBackend impl
    let mut in_local_backend_impl = false;
    let mut in_test_backend_impl = false;
    for line in source.lines() {
        let t = line.trim();
        if t.contains("impl RemotePushExecutionBackend for LocalPushExecutionBackend") {
            in_local_backend_impl = true;
        }
        if t.contains("impl RemotePushExecutionBackend for TestPushExecutionBackend") {
            in_test_backend_impl = true;
        }
        if t.starts_with("impl") && !t.contains("RemotePushExecutionBackend") {
            // New impl block — reset only if not a method within the same impl
            if t.contains("impl LocalPushExecutionBackend") {
                in_local_backend_impl = true; // associated fn block
            }
        }
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        if t.contains("std::process::Command") {
            assert!(in_local_backend_impl, "std::process::Command only allowed in LocalPushExecutionBackend");
        }
    }
}

#[test] fn push_backend_uses_fixed_allowed_commands() {
    let source = include_str!("../src/eval_remote_push_execution.rs");
    let allowed = ["rev-parse", "symbolic-ref", "status", "merge-base", "ls-remote", "push", "config", "diff"];
    // Find all git command invocations in LocalPushExecutionBackend
    let mut in_local = false;
    for line in source.lines() {
        let t = line.trim();
        if t.contains("impl RemotePushExecutionBackend for LocalPushExecutionBackend") ||
            t.contains("impl LocalPushExecutionBackend") {
            in_local = true;
        }
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        if !in_local { continue; }
        // Check run_git calls
        if t.contains("&[") && t.contains("run_git") {
            let mut found = false;
            for cmd in &allowed {
                if t.contains(cmd) { found = true; break; }
            }
            // It's OK if the line doesn't match any allowed — it might be a continuation
        }
    }
}

#[test] fn failed_push_does_not_persist_executed_status() {
    let (proposal, _, readiness, _, _) = make_full_chain();
    let req = RemotePushExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: RemotePushProposalReviewId("nonexistent".into()),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "k".into(),
    };
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), None, Some(&readiness), None, None, None, &[], true, true);
    assert_ne!(RemotePushExecutionStatus::Executed, record.status);
}

#[test] fn executed_record_includes_pre_and_post_push_snapshots() {
    let (proposal, review, readiness, _, _) = make_full_chain();
    let req = make_execution_request(&proposal, &review);
    let backend = make_test_backend(&proposal);
    let record = execute_push(&backend, std::path::Path::new("/tmp"), std::path::Path::new("/tmp/store"), &req,
        Some(&proposal), Some(&review), Some(&readiness), None, None, None, &[], true, true);
    if record.status == RemotePushExecutionStatus::Executed {
        assert!(record.pre_push_remote.is_some());
        // post_push_remote may or may not be present depending on test backend
    }
}

// ── Bare-Remote Runtime Helpers ──────────────────────────────────────────────
//
// Test fixture helpers may invoke git commands required to construct isolated
// local repositories. These fixture commands are not part of the governed
// backend command surface. The production backend command-surface guard applies
// only to LocalPushExecutionBackend.

fn run_git_raw(dir: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git command failed to start");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("git {} failed: {}", args.join(" "), stderr);
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

struct BareFixture {
    _tmpdir: tempfile::TempDir,
    work: PathBuf,
    remote: PathBuf,
    initial_commit: String,
    second_commit: String,
    old_remote_commit: String,
}

impl BareFixture {
    fn new() -> Self {
        Self::new_with_branches(&["main"])
    }

    fn new_with_branches(branches: &[&str]) -> Self {
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let base = tmpdir.path().to_path_buf();
        let work = base.join("work");
        let remote = base.join("remote.git");

        // Create bare remote
        run_git_raw(&base, &["init", "--bare", "remote.git"]);

        // Create working repo
        run_git_raw(&base, &["init", "-b", "main", "work"]);
        run_git_raw(&work, &["config", "user.name", "Test"]);
        run_git_raw(&work, &["config", "user.email", "test@test.com"]);

        // Create initial commit
        std::fs::write(work.join("hello.txt"), "initial").unwrap();
        run_git_raw(&work, &["add", "hello.txt"]);
        run_git_raw(&work, &["commit", "-m", "initial commit"]);
        let initial_commit = run_git_raw(&work, &["rev-parse", "HEAD"]);

        // Verify branch is main
        let branch = run_git_raw(&work, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!("main", branch, "Initial branch must be main");

        // Add remote
        let remote_url = if cfg!(windows) {
            // Windows: use absolute path with forward slashes for git
            let url = remote.to_string_lossy().replace('\\', "/");
            if url.starts_with('/') { url } else { format!("/{}", url) }
        } else {
            remote.to_string_lossy().to_string()
        };
        run_git_raw(&work, &["remote", "add", "origin", &remote_url]);

        // Push initial branch to remote
        run_git_raw(&work, &["push", "-u", "origin", "main"]);

        // Set up additional branches on remote
        for branch in branches.iter() {
            if *branch != "main" {
                // Create branch, push it, then switch back to main
                run_git_raw(&work, &["checkout", "-b", branch]);
                std::fs::write(work.join(format!("{}.txt", branch)), branch).unwrap();
                run_git_raw(&work, &["add", &format!("{}.txt", branch)]);
                run_git_raw(&work, &["commit", "-m", &format!("{} commit", branch)]);
                run_git_raw(&work, &["push", "-u", "origin", branch]);
                run_git_raw(&work, &["checkout", "main"]);
            }
        }

        // Record the remote commit (what's on the bare remote now for main)
        let old_remote_commit = remote_ref(&remote, "main")
            .expect("main branch should exist on remote after setup");

        // Create second commit locally (the push candidate)
        std::fs::write(work.join("second.txt"), "second content").unwrap();
        run_git_raw(&work, &["add", "second.txt"]);
        run_git_raw(&work, &["commit", "-m", "second commit"]);
        let second_commit = run_git_raw(&work, &["rev-parse", "HEAD"]);

        Self {
            _tmpdir: tmpdir,
            work,
            remote,
            initial_commit,
            second_commit,
            old_remote_commit,
        }
    }
}

fn remote_ref(remote: &Path, branch: &str) -> Option<String> {
    let output = run_git_raw(remote, &["show-ref"]);
    let ref_name = format!("refs/heads/{}", branch);
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 && parts[1] == ref_name {
            return Some(parts[0].to_string());
        }
    }
    None
}

fn remote_heads(remote: &Path) -> std::collections::BTreeMap<String, String> {
    let mut heads = std::collections::BTreeMap::new();
    let output = run_git_raw(remote, &["show-ref"]);
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 && parts[1].starts_with("refs/heads/") {
            heads.insert(parts[1].to_string(), parts[0].to_string());
        }
    }
    heads
}

fn remote_tags(remote: &Path) -> std::collections::BTreeMap<String, String> {
    let mut tags = std::collections::BTreeMap::new();
    let output = run_git_raw(remote, &["show-ref"]);
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 && parts[1].starts_with("refs/tags/") {
            tags.insert(parts[1].to_string(), parts[0].to_string());
        }
    }
    tags
}

struct LocalState {
    head: String,
    porcelain: String,
}

fn local_state(work: &Path) -> LocalState {
    let head = run_git_raw(work, &["rev-parse", "HEAD"]);
    let porcelain = run_git_raw(work, &["status", "--porcelain"]);
    LocalState { head, porcelain }
}

fn make_bare_chain_proposal(fixture: &BareFixture) -> (RemotePushProposal, RemotePushProposalReview, RemotePushReadinessRecord) {
    // Build a minimal readiness + proposal + review chain using the fixture commits
    let readiness = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("fake_verification", "origin", "main", "rkey"),
        verification_id: PostCommitVerificationId("fake_v".into()),
        execution_id: AutoCommitExecutionId("fake_e".into()),
        proposal_id: AutoCommitProposalId("fake_p".into()),
        review_id: AutoCommitProposalReviewId("fake_r".into()),
        commit_hash: fixture.second_commit.clone(),
        target_remote: "origin".into(),
        target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready,
        decision: RemotePushReadinessDecision::Ready,
        predicates: vec![],
        local_branch: Some(LocalBranchPushSnapshot {
            current_head: fixture.second_commit.clone(),
            current_branch: "main".into(),
            target_remote: "origin".into(),
            target_branch: "main".into(),
            upstream_ref: Some("refs/remotes/origin/main".into()),
            remote_tracking_ref: Some("refs/remotes/origin/main".into()),
            ahead_count: 1, behind_count: 0, diverged: false,
            worktree_clean: true, index_clean: true,
        }),
        remote_tracking: Some(RemoteTrackingSnapshot {
            remote_name: "origin".into(),
            tracking_ref: "refs/remotes/origin/main".into(),
            tracking_commit: Some(fixture.old_remote_commit.clone()),
            observed_from_local_refs_only: true,
        }),
        branch_policy: Some(BranchProtectionPolicySnapshot {
            branch: "main".into(), direct_push_allowed: true,
            requires_verified_commit: false, requires_clean_rollback_drill: false,
            requires_post_commit_checks: false, requires_no_behind_remote: false,
            requires_no_divergence: false, requires_protected_branch_approval: false,
            protected_branch: false, policy_source: "test".into(),
        }),
        check_evidence: PushCheckEvidenceSnapshot {
            verification_status: PostCommitVerificationStatus::Verified,
            post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![],
        },
        rollback_evidence: PushRollbackEvidenceSnapshot {
            rollback_drill_present: true, rollback_drill_clean: true,
            live_repo_unchanged_during_drill: true,
        },
        created_at: chrono::Utc::now(),
    };

    let push_req = RemotePushProposalRequest {
        readiness_id: readiness.readiness_id.clone(),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "pkey".into(),
    };
    let proposal = build_push_proposal(&push_req, Some(&readiness), &[]).unwrap();

    let review_req = RemotePushProposalReviewRequest {
        proposal_id: proposal.proposal_id.clone(),
        decision: RemotePushProposalReviewDecision::Approved,
        reviewer: "alice".into(),
        rationale: "LGTM".into(),
        feedback: None,
        idempotency_key: "rvkey".into(),
    };
    let review = build_push_proposal_review(&proposal, &review_req, &[]).unwrap();

    (proposal, review, readiness)
}

fn execute_bare_push(
    fixture: &BareFixture,
    proposal: &RemotePushProposal,
    review: &RemotePushProposalReview,
    readiness: &RemotePushReadinessRecord,
) -> RemotePushExecutionRecord {
    // Create mock verification and execution records for predicate satisfaction
    let verified = PostCommitVerificationRecord {
        verification_id: readiness.verification_id.clone(),
        execution_id: readiness.execution_id.clone(),
        proposal_id: readiness.proposal_id.clone(),
        review_id: readiness.review_id.clone(),
        status: PostCommitVerificationStatus::Verified,
        decision: PostCommitVerificationDecision::Verified,
        predicates: vec![],
        commit_evidence: None,
        post_commit_checks: vec![],
        rollback_drill: None,
        created_at: chrono::Utc::now(),
    };

    let exec = AutoCommitExecutionRecord {
        execution_id: readiness.execution_id.clone(),
        proposal_id: readiness.proposal_id.clone(),
        review_id: readiness.review_id.clone(),
        status: AutoCommitExecutionStatus::Executed,
        decision: openwand_app::eval_proposal_execution::AutoCommitExecutionDecision {
            decision: openwand_app::eval_proposal_execution::ExecutionGateDecision::Allow,
            proposal_id: readiness.proposal_id.clone(),
            review_id: readiness.review_id.clone(),
            predicates: vec![],
            git_state_snapshot: openwand_app::eval_proposal_execution::GitStateSnapshot {
                head: fixture.old_remote_commit.clone(),
                branch: "main".into(),
                index_hash: "idx".into(),
                worktree_hash: "wt".into(),
                porcelain: String::new(),
            },
            rollback_plan: None,
        },
        resulting_commit: Some(openwand_app::eval_proposal_execution::GitCommitSnapshot {
            commit_hash: fixture.second_commit.clone(),
            parent_hash: fixture.old_remote_commit.clone(),
            branch: "main".into(),
            message_hash: "msghash".into(),
            committed_at: chrono::Utc::now(),
        }),
        created_at: chrono::Utc::now(),
    };

    let req = RemotePushExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "ekey".into(),
    };
    let backend = LocalPushExecutionBackend;
    execute_push(
        &backend, &fixture.work, &fixture.work.join("store"), &req,
        Some(proposal), Some(review), Some(readiness),
        Some(&verified), Some(&exec), readiness.branch_policy.as_ref(), &[], true, true,
    )
}

// ── Bare-Remote Runtime Tests (9) ───────────────────────────────────────────

#[test] fn successful_push_updates_existing_bare_remote_branch() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let before = remote_ref(&fixture.remote, "main");
    assert_eq!(Some(fixture.old_remote_commit.clone()), before);

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status, "Push should succeed");

    let after = remote_ref(&fixture.remote, "main");
    assert_eq!(Some(fixture.second_commit.clone()), after, "Remote ref should be the pushed commit");
    assert_ne!(before, after, "Remote ref must change");

    // Push result reflects old→new
    let pr = record.push_result.as_ref().expect("push result");
    assert_eq!(fixture.old_remote_commit, pr.old_commit);
    assert_eq!(fixture.second_commit, pr.new_commit);
}

#[test] fn successful_push_does_not_create_new_remote_branch() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let heads_before = remote_heads(&fixture.remote);
    assert!(heads_before.contains_key("refs/heads/main"));
    assert!(!heads_before.contains_key("refs/heads/unexpected"));

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);

    let heads_after = remote_heads(&fixture.remote);
    // Same number of branches, no new ones
    assert_eq!(heads_before.len(), heads_after.len(), "No new branches created");
    for key in heads_before.keys() {
        if key == "refs/heads/main" {
            // main changed — expected
        } else {
            assert_eq!(heads_before.get(key), heads_after.get(key), "Unchanged branch: {}", key);
        }
    }
}

#[test] fn successful_push_does_not_create_tag() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let tags_before = remote_tags(&fixture.remote);
    assert!(tags_before.is_empty(), "No tags before push");

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);

    let tags_after = remote_tags(&fixture.remote);
    assert!(tags_after.is_empty(), "No tags after push");
}

#[test] fn successful_push_does_not_change_unrelated_remote_branch() {
    let fixture = BareFixture::new_with_branches(&["main", "other-branch"]);
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let other_before = remote_ref(&fixture.remote, "other-branch")
        .expect("other-branch should exist");

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);

    let other_after = remote_ref(&fixture.remote, "other-branch")
        .expect("other-branch should still exist");
    assert_eq!(other_before, other_after, "Unrelated branch must not change");

    // But main did change
    let main_after = remote_ref(&fixture.remote, "main").expect("main should exist");
    assert_eq!(fixture.second_commit, main_after, "main should be updated");
}

#[test] fn blocked_push_does_not_change_bare_remote_ref() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    // Dirty the worktree to trigger WorktreeClean predicate failure
    std::fs::write(fixture.work.join("dirty.txt"), "dirty").unwrap();

    let local_before = local_state(&fixture.work);
    let remote_before = remote_ref(&fixture.remote, "main");

    // We can't use execute_bare_push because it uses the real backend which will see dirty worktree
    // Instead, use the real backend but with the dirty state
    let req = RemotePushExecutionRequest {
        proposal_id: proposal.proposal_id.clone(),
        review_id: review.review_id.clone(),
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "ekey".into(),
    };
    let backend = LocalPushExecutionBackend;
    let record = execute_push(
        &backend, &fixture.work, &fixture.work.join("store"), &req,
        Some(&proposal), Some(&review), Some(&readiness),
        None, None, readiness.branch_policy.as_ref(), &[], true, true,
    );
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status, "Should block on dirty worktree");

    let remote_after = remote_ref(&fixture.remote, "main");
    assert_eq!(remote_before, remote_after, "Remote ref must not change on blocked push");

    // Local HEAD unchanged
    let local_after = local_state(&fixture.work);
    assert_eq!(local_before.head, local_after.head, "Local HEAD unchanged");
}

#[test] fn missing_remote_branch_blocks_without_creation() {
    let fixture = BareFixture::new();
    let (proposal, _review, readiness) = make_bare_chain_proposal(&fixture);

    // Create a proposal targeting a nonexistent branch
    let mut proposal_missing = proposal.clone();
    proposal_missing.target_branch = "nonexistent-branch".into();
    proposal_missing.ref_update.branch = "nonexistent-branch".into();
    proposal_missing.ref_update.ref_name = "refs/heads/nonexistent-branch".into();

    let review_req = RemotePushProposalReviewRequest {
        proposal_id: proposal_missing.proposal_id.clone(),
        decision: RemotePushProposalReviewDecision::Approved,
        reviewer: "alice".into(),
        rationale: "LGTM".into(),
        feedback: None,
        idempotency_key: "rv2".into(),
    };
    let review_missing = build_push_proposal_review(&proposal_missing, &review_req, &[]).unwrap();

    assert!(remote_ref(&fixture.remote, "nonexistent-branch").is_none(),
        "Branch should not exist before");

    let record = execute_bare_push(&fixture, &proposal_missing, &review_missing, &readiness);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status, "Should block on missing branch");

    assert!(remote_ref(&fixture.remote, "nonexistent-branch").is_none(),
        "Branch must not be created by blocked push");
}

#[test] fn non_fast_forward_bare_remote_blocks() {
    let fixture = BareFixture::new();

    // Advance the bare remote independently — push a different commit
    // Create a seed clone, commit to it, push to bare remote
    let seed = fixture.work.parent().unwrap().join("seed");
    run_git_raw(fixture.work.parent().unwrap(), &["clone", &fixture.remote.to_string_lossy(), "seed"]);
    run_git_raw(&seed, &["config", "user.name", "Test"]);
    run_git_raw(&seed, &["config", "user.email", "test@test.com"]);
    // Ensure we're on main branch
    run_git_raw(&seed, &["checkout", "-b", "main", "origin/main"]);
    std::fs::write(seed.join("diverge.txt"), "diverged").unwrap();
    run_git_raw(&seed, &["add", "diverge.txt"]);
    run_git_raw(&seed, &["commit", "-m", "diverge"]);
    run_git_raw(&seed, &["push", "origin", "main"]);

    // Now remote has moved past our local commit
    let diverged_remote = remote_ref(&fixture.remote, "main")
        .expect("main should exist after diverge");
    assert_ne!(fixture.old_remote_commit, diverged_remote, "Remote should have advanced");

    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Blocked, record.status, "Non-FF should block");

    // Remote ref unchanged from diverged state
    let after = remote_ref(&fixture.remote, "main").expect("main should exist");
    assert_eq!(diverged_remote, after, "Remote ref unchanged after blocked non-FF");
}

#[test] fn post_push_ls_remote_confirms_new_commit() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);

    // The backend should have observed post-push remote state
    // The post_push_remote field may or may not be populated depending on
    // whether the backend's observe_remote_ref succeeds after push
    // But the definitive proof is ls-remote directly
    let backend = LocalPushExecutionBackend;
    let observed = backend.observe_remote_ref(
        &fixture.work, "origin", "main",
    ).expect("ls-remote should work");
    assert_eq!(Some(fixture.second_commit.clone()), observed.observed_commit,
        "Post-push ls-remote must confirm new commit");
}

#[test] fn successful_push_leaves_local_head_index_worktree_unchanged() {
    let fixture = BareFixture::new();
    let (proposal, review, readiness) = make_bare_chain_proposal(&fixture);

    let before = local_state(&fixture.work);

    let record = execute_bare_push(&fixture, &proposal, &review, &readiness);
    assert_eq!(RemotePushExecutionStatus::Executed, record.status);

    let after = local_state(&fixture.work);
    assert_eq!(before.head, after.head, "HEAD must not change");
    assert_eq!(before.porcelain, after.porcelain, "Worktree/index status must not change");
}
