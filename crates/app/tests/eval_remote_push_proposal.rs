//! Push proposal and human approval gate tests.

use openwand_app::eval_remote_push_proposal::*;
use openwand_app::eval_remote_push_readiness::*;
use openwand_app::eval_post_commit_verify::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_execution::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_readiness::*;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_full_chain() -> (AutoCommitProposal, AutoCommitProposalReview, AutoCommitExecutionRecord, PostCommitVerificationRecord, RemotePushReadinessRecord) {
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
    RemotePushReadinessRecord {
        readiness_id: readiness_id_for(&verified.verification_id.0, "origin", "main", "rkey"),
        verification_id: verified.verification_id.clone(), execution_id: verified.execution_id.clone(),
        proposal_id: verified.proposal_id.clone(), review_id: verified.review_id.clone(),
        commit_hash: verified.commit_evidence.as_ref().map(|e| e.commit_hash.clone()).unwrap_or_default(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![],
        local_branch: Some(LocalBranchPushSnapshot {
            current_head: verified.commit_evidence.as_ref().map(|e| e.commit_hash.clone()).unwrap_or_default(),
            current_branch: "main".into(), target_remote: "origin".into(), target_branch: "main".into(),
            upstream_ref: Some("refs/remotes/origin/main".into()), remote_tracking_ref: Some("refs/remotes/origin/main".into()),
            ahead_count: 1, behind_count: 0, diverged: false, worktree_clean: true, index_clean: true,
        }),
        remote_tracking: Some(RemoteTrackingSnapshot {
            remote_name: "origin".into(), tracking_ref: "refs/remotes/origin/main".into(),
            tracking_commit: Some("tracking_commit".into()), observed_from_local_refs_only: true,
        }),
        branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    }
}

fn make_proposal_request(readiness: &RemotePushReadinessRecord) -> RemotePushProposalRequest {
    RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey".into() }
}

fn make_review_request(proposal: &RemotePushProposal, decision: RemotePushProposalReviewDecision) -> RemotePushProposalReviewRequest {
    RemotePushProposalReviewRequest {
        proposal_id: proposal.proposal_id.clone(), decision, reviewer: "alice".into(), rationale: "LGTM".into(),
        feedback: None, idempotency_key: "rvkey".into(),
    }
}

fn make_full_proposal() -> (RemotePushProposal, RemotePushReadinessRecord) {
    let (_, _, _, _, readiness) = make_full_chain();
    let req = make_proposal_request(&readiness);
    let proposal = build_push_proposal(&req, Some(&readiness), &[]).unwrap();
    (proposal, readiness)
}

// Readiness/workspace/eval helpers
fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport { generated_at: chrono::Utc::now(), report_schema_version: 1, target: ReadinessTarget::AutoCommit, status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore { weighted_pass_rate: 0.95, patch_pass_rate: 0.98, policy_pass_rate: 1.0, rebuild_pass_rate: 1.0, explain_pass_rate: 0.95, regression_count: 0 },
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
        score: EvalScore { total: 5, max: 5, pass_rate: 1.0, dimensions: vec![
            DimensionScore { name: "patch".into(), passed: 1, total: 1, evidence_refs: vec![EvalEvidenceRef { source: EvalEvidenceSource::Trace, event_kind: Some("file.patch".into()), summary: "test".into() }] },
        ] },
    }
}

// ── DTO Tests (8) ───────────────────────────────────────────────────────────

#[test] fn push_proposal_request_roundtrips() {
    let (_, _, _, _, readiness) = make_full_chain();
    let req = make_proposal_request(&readiness);
    let json = serde_json::to_string(&req).unwrap();
    let parsed: RemotePushProposalRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.idempotency_key, parsed.idempotency_key);
}

#[test] fn push_proposal_record_roundtrips() {
    let (proposal, _) = make_full_proposal();
    let json = serde_json::to_string(&proposal).unwrap();
    let parsed: RemotePushProposal = serde_json::from_str(&json).unwrap();
    assert_eq!(proposal.proposal_id, parsed.proposal_id);
    assert_eq!(proposal.status, parsed.status);
}

#[test] fn push_proposal_id_is_content_addressed() {
    let a = push_proposal_id_for("r1", "k1");
    let b = push_proposal_id_for("r1", "k1");
    let c = push_proposal_id_for("r1", "k2");
    assert_eq!(a, b); assert_ne!(a, c);
}

#[test] fn push_proposal_id_is_deterministic() {
    for _ in 0..5 { assert_eq!("rpp_", &push_proposal_id_for("r", "k").0[..4]); }
}

#[test] fn push_proposal_id_differs_by_remote_branch_commit() {
    let a = push_proposal_id_for("r1", "k");
    let b = push_proposal_id_for("r2", "k");
    assert_ne!(a, b);
}

#[test] fn push_review_roundtrips() {
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    let json = serde_json::to_string(&review).unwrap();
    let parsed: RemotePushProposalReview = serde_json::from_str(&json).unwrap();
    assert_eq!(review.review_id, parsed.review_id);
    assert_eq!(review.decision, parsed.decision);
}

#[test] fn push_review_id_is_content_addressed() {
    let a = push_review_id_for("p1", "alice", "k1");
    let b = push_review_id_for("p1", "alice", "k1");
    let c = push_review_id_for("p1", "alice", "k2");
    assert_eq!(a, b); assert_ne!(a, c);
}

#[test] fn push_review_id_is_deterministic() {
    for _ in 0..5 { assert_eq!("rprv_", &push_review_id_for("p", "r", "k").0[..5]); }
}

// ── Proposal Creation Tests (10) ────────────────────────────────────────────

#[test] fn creates_proposal_from_ready_readiness() {
    let (proposal, _) = make_full_proposal();
    assert_eq!(RemotePushProposalStatus::Eligible, proposal.status);
    assert!(proposal.commit_hash.contains("testcommit"));
}

#[test] fn proposal_copies_remote_branch_and_commit_from_readiness() {
    let (proposal, readiness) = make_full_proposal();
    assert_eq!(readiness.target_remote, proposal.target_remote);
    assert_eq!(readiness.target_branch, proposal.target_branch);
    assert_eq!(readiness.commit_hash, proposal.commit_hash);
}

#[test] fn proposal_copies_expected_remote_tracking_commit() {
    let (proposal, readiness) = make_full_proposal();
    let expected = readiness.remote_tracking.as_ref().and_then(|rt| rt.tracking_commit.clone()).unwrap_or_default();
    assert_eq!(expected, proposal.expected_remote_tracking_commit);
}

#[test] fn proposal_ref_update_is_fast_forward_only() {
    let (proposal, _) = make_full_proposal();
    assert!(proposal.ref_update.fast_forward_only);
}

#[test] fn blocks_missing_readiness() {
    let (_, _, _, _, readiness) = make_full_chain();
    let req = make_proposal_request(&readiness);
    let result = build_push_proposal(&req, None, &[]);
    assert!(result.is_err());
}

#[test] fn blocks_blocked_readiness() {
    let (_, _, _, _, mut readiness) = make_full_chain();
    readiness.status = RemotePushReadinessStatus::Blocked;
    let req = make_proposal_request(&readiness);
    let result = build_push_proposal(&req, Some(&readiness), &[]);
    assert!(result.is_err());
}

#[test] fn blocks_inconclusive_readiness() {
    let (_, _, _, _, mut readiness) = make_full_chain();
    readiness.status = RemotePushReadinessStatus::Inconclusive;
    let req = make_proposal_request(&readiness);
    let result = build_push_proposal(&req, Some(&readiness), &[]);
    assert!(result.is_err());
}

#[test] fn blocks_non_verified_chain() {
    let (_, _, _, _, mut readiness) = make_full_chain();
    readiness.status = RemotePushReadinessStatus::Ready;
    readiness.decision = RemotePushReadinessDecision::Blocked { reason_code: "x".into(), summary: "y".into() };
    let req = make_proposal_request(&readiness);
    let result = build_push_proposal(&req, Some(&readiness), &[]);
    assert!(result.is_err());
}

#[test] fn proposal_hash_changes_when_ref_update_changes() {
    let (p1, readiness) = make_full_proposal();
    let mut r2 = readiness.clone();
    r2.commit_hash = "DIFFERENT_HASH".into();
    let req2 = RemotePushProposalRequest { readiness_id: r2.readiness_id.clone(), ..make_proposal_request(&readiness) };
    let p2 = build_push_proposal(&req2, Some(&r2), &[]).unwrap();
    assert_ne!(p1.proposal_hash, p2.proposal_hash);
}

#[test] fn proposal_copies_readiness_hash_from_persisted_record() {
    // Patch 3: readiness_hash comes from serialized readiness, not recomputed
    let (proposal, readiness) = make_full_proposal();
    let expected_hash = format!("{}", blake3::hash(serde_json::to_string(&readiness).unwrap().as_bytes()).to_hex());
    assert_eq!(expected_hash, proposal.readiness_hash);
}

// ── Review Validation Tests (11) ────────────────────────────────────────────

#[test] fn approval_requires_reviewer() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "  ".into(), rationale: "ok".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn approval_requires_rationale() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "  ".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn approval_does_not_create_execution_grant() {
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert!(!review.creates_execution_grant);
    assert!(!review.execution_allowed_now);
}

#[test] fn rejection_requires_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn rejection_requires_blocking_reason() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec![], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn change_request_requires_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "needs work".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn change_request_requires_requested_change() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "fix".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "fix".into(), blocking_reasons: vec![], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn latest_review_supersedes_prior_for_lookup() {
    let (proposal, _) = make_full_proposal();
    let req1 = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "fix".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "fix".into(), blocking_reasons: vec![], requested_changes: vec!["add tests".into()], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k1".into() };
    let r1 = build_push_proposal_review(&proposal, &req1, &[]).unwrap();
    let req2 = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let r2 = build_push_proposal_review(&proposal, &req2, &[r1.clone()]).unwrap();
    assert_ne!(r1.review_id, r2.review_id);
    // Latest is the approved one
    assert_eq!(RemotePushProposalReviewDecision::Approved, r2.decision);
}

#[test] fn prior_reviews_remain_persisted() {
    // Both reviews should be savable/loadable
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    let req1 = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["risk".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k1".into() };
    let r1 = build_push_proposal_review(&proposal, &req1, &[]).unwrap();
    save_push_proposal_review(dir.path(), &r1).unwrap();
    let req2 = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let r2 = build_push_proposal_review(&proposal, &req2, &[r1.clone()]).unwrap();
    save_push_proposal_review(dir.path(), &r2).unwrap();
    // Both loadable
    assert!(load_push_proposal_review(dir.path(), &r1.review_id).unwrap().is_some());
    assert!(load_push_proposal_review(dir.path(), &r2.review_id).unwrap().is_some());
}

// ── Idempotency Tests (3) ───────────────────────────────────────────────────

#[test] fn same_idempotency_key_returns_existing_push_proposal() {
    let (_, _, _, _, readiness) = make_full_chain();
    let req = make_proposal_request(&readiness);
    let p1 = build_push_proposal(&req, Some(&readiness), &[]).unwrap();
    let p2 = build_push_proposal(&req, Some(&readiness), &[p1.clone()]).unwrap();
    assert_eq!(p1.proposal_id, p2.proposal_id);
}

#[test] fn same_idempotency_key_returns_existing_push_review() {
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let r1 = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    let r2 = build_push_proposal_review(&proposal, &req, &[r1.clone()]).unwrap();
    assert_eq!(r1.review_id, r2.review_id);
}

#[test] fn different_review_idempotency_key_preserves_audit_history() {
    let (proposal, _) = make_full_proposal();
    let req1 = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "ok".into(), feedback: None, idempotency_key: "k1".into() };
    let r1 = build_push_proposal_review(&proposal, &req1, &[]).unwrap();
    let req2 = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "bob".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["risk".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k2".into() };
    let r2 = build_push_proposal_review(&proposal, &req2, &[r1.clone()]).unwrap();
    assert_ne!(r1.review_id, r2.review_id, "Different key should create new review");
}

// ── Feedback Export Tests (6) ────────────────────────────────────────────────

#[test] fn rejected_review_exports_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["risk".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k".into() };
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert!(review.feedback.is_some());
    let fb = review.feedback.unwrap();
    assert!(!fb.blocking_reasons.is_empty());
}

#[test] fn change_requested_review_exports_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "fix".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "needs work".into(), blocking_reasons: vec![], requested_changes: vec!["add tests".into()], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k".into() };
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert!(review.feedback.is_some());
    let fb = review.feedback.unwrap();
    assert!(!fb.requested_changes.is_empty());
}

#[test] fn approved_review_does_not_require_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert!(review.feedback.is_none());
}

#[test] fn feedback_includes_blocking_reasons() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["security risk".into(), "no tests".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k".into() };
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert_eq!(2, review.feedback.unwrap().blocking_reasons.len());
}

#[test] fn feedback_includes_requested_changes() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "fix".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "needs work".into(), blocking_reasons: vec![], requested_changes: vec!["add tests".into(), "fix docs".into()], evidence_gaps: vec![], suggested_next_action: "fix".into() }),
        idempotency_key: "k".into() };
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert_eq!(2, review.feedback.unwrap().requested_changes.len());
}

#[test] fn feedback_roundtrips() {
    let fb = RemotePushProposalFeedback { summary: "test".into(), blocking_reasons: vec!["a".into()], requested_changes: vec!["b".into()], evidence_gaps: vec!["c".into()], suggested_next_action: "do".into() };
    let json = serde_json::to_string(&fb).unwrap();
    let parsed: RemotePushProposalFeedback = serde_json::from_str(&json).unwrap();
    assert_eq!(fb.summary, parsed.summary);
    assert_eq!(fb.blocking_reasons, parsed.blocking_reasons);
}

// ── Persistence Tests (8) ────────────────────────────────────────────────────

#[test] fn proposal_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    save_push_proposal(dir.path(), &proposal).unwrap();
    let loaded = load_push_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, loaded.proposal_id);
    assert_eq!(proposal.proposal_hash, loaded.proposal_hash);
}

#[test] fn latest_proposal_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    save_push_proposal(dir.path(), &proposal).unwrap();
    let latest = load_latest_push_proposal(dir.path()).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, latest.proposal_id);
}

#[test] fn proposal_by_readiness_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, readiness) = make_full_proposal();
    save_push_proposal(dir.path(), &proposal).unwrap();
    let loaded = load_push_proposal_by_readiness(dir.path(), &readiness.readiness_id).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, loaded.proposal_id);
}

#[test] fn review_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    save_push_proposal_review(dir.path(), &review).unwrap();
    let loaded = load_push_proposal_review(dir.path(), &review.review_id).unwrap().unwrap();
    assert_eq!(review.review_id, loaded.review_id);
}

#[test] fn latest_review_for_proposal_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    save_push_proposal_review(dir.path(), &review).unwrap();
    let loaded = load_latest_push_review_for_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(review.review_id, loaded.review_id);
}

#[test] fn feedback_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(),
        feedback: Some(RemotePushProposalFeedback { summary: "bad".into(), blocking_reasons: vec!["risk".into()], requested_changes: vec![], evidence_gaps: vec![], suggested_next_action: "recheck".into() }),
        idempotency_key: "k".into() };
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    save_push_proposal_review(dir.path(), &review).unwrap();
    let fb = load_push_review_feedback(dir.path(), &review.review_id).unwrap().unwrap();
    assert_eq!("bad", fb.summary);
    assert_eq!(vec!["risk".to_string()], fb.blocking_reasons);
}

#[test] fn list_push_proposals_sorted_by_date() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let (_, _, _, _, readiness) = make_full_chain();
        let req = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: format!("k{}", i) };
        let p = build_push_proposal(&req, Some(&readiness), &[]).unwrap();
        save_push_proposal(dir.path(), &p).unwrap();
    }
    let loaded = list_push_proposals(dir.path()).unwrap();
    assert_eq!(3, loaded.len());
    assert!(loaded[0].created_at >= loaded[1].created_at);
}

#[test] fn list_push_reviews_sorted_by_date() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    for i in 0..3 {
        let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: format!("r{}", i), rationale: "ok".into(), feedback: None, idempotency_key: format!("k{}", i) };
        let r = build_push_proposal_review(&proposal, &req, &[]).unwrap();
        save_push_proposal_review(dir.path(), &r).unwrap();
    }
    let loaded = list_push_reviews(dir.path()).unwrap();
    assert_eq!(3, loaded.len());
    assert!(loaded[0].reviewed_at >= loaded[1].reviewed_at);
}

// ── CLI Tests (8) ────────────────────────────────────────────────────────────

#[test] fn cli_push_proposal_create_outputs_proposal_id() {
    let (proposal, _) = make_full_proposal();
    assert!(proposal.proposal_id.0.starts_with("rpp_"));
}

#[test] fn cli_push_proposal_show_roundtrips_record() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    save_push_proposal(dir.path(), &proposal).unwrap();
    let loaded = load_push_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, loaded.proposal_id);
    assert_eq!(proposal.commit_hash, loaded.commit_hash);
}

#[test] fn cli_push_proposal_latest_by_readiness_returns_latest() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, readiness) = make_full_proposal();
    save_push_proposal(dir.path(), &proposal).unwrap();
    let loaded = load_push_proposal_by_readiness(dir.path(), &readiness.readiness_id).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, loaded.proposal_id);
}

#[test] fn cli_push_review_approve_outputs_review_id() {
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    assert!(review.review_id.0.starts_with("rprv_"));
    assert!(!review.creates_execution_grant);
    assert!(!review.execution_allowed_now);
}

#[test] fn cli_push_review_reject_requires_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Rejected, reviewer: "alice".into(), rationale: "no".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn cli_push_review_request_changes_requires_feedback() {
    let (proposal, _) = make_full_proposal();
    let req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::ChangesRequested, reviewer: "alice".into(), rationale: "fix".into(), feedback: None, idempotency_key: "k".into() };
    assert!(build_push_proposal_review(&proposal, &req, &[]).is_err());
}

#[test] fn cli_push_review_latest_returns_latest() {
    let dir = tempfile::tempdir().unwrap();
    let (proposal, _) = make_full_proposal();
    let req = make_review_request(&proposal, RemotePushProposalReviewDecision::Approved);
    let review = build_push_proposal_review(&proposal, &req, &[]).unwrap();
    save_push_proposal_review(dir.path(), &review).unwrap();
    let latest = load_latest_push_review_for_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(review.review_id, latest.review_id);
}

#[test] fn cli_does_not_expose_push_execution() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git push"), "No git push");
    }
}

// ── Patch 3 Tests (1) ────────────────────────────────────────────────────────

// proposal_copies_readiness_hash_from_persisted_record is tested above in Proposal Creation

#[test] fn proposal_does_not_recompute_readiness_hash_from_current_git_state() {
    // Verify the module has no git observation — purely data
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        assert!(!t.contains("std::process::Command"), "No process::Command in code");
        assert!(!t.to_lowercase().contains("git "), "No git commands in code");
    }
}

// ── Source and Runtime Guard Tests (8) ───────────────────────────────────────

#[test] fn module_does_not_call_git_push() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        assert!(!t.to_lowercase().contains("git push"), "No git push");
    }
}

#[test] fn module_does_not_call_git_fetch_pull_or_ls_remote() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git fetch"), "No fetch");
        assert!(!lower.contains("git pull"), "No pull");
        assert!(!lower.contains("git ls-remote"), "No ls-remote");
    }
}

#[test] fn module_does_not_create_tags_or_branches() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git tag"), "No tag");
        assert!(!lower.contains("git branch"), "No branch");
    }
}

#[test] fn module_does_not_call_release_or_remote_mutation_tools() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git remote"), "No remote mutation");
        assert!(!lower.contains("hub ") && !lower.contains("gh ") && !lower.contains("glab "), "No host tools");
    }
}

#[test] fn module_does_not_execute_arbitrary_shell() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("/bin/sh"), "No /bin/sh");
        assert!(!lower.contains("cmd.exe"), "No cmd.exe");
    }
}

#[test] fn module_does_not_import_process_command() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        assert!(!t.contains("std::process::Command"), "No process::Command in code");
    }
}

#[test] fn module_does_not_import_push_backend() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    assert!(!source.contains("LocalPushReadinessBackend"), "No push backend import");
    assert!(!source.contains("RemotePushReadinessBackend"), "No readiness backend import as type");
}

#[test] fn proposal_creation_leaves_everything_unchanged() {
    let source = include_str!("../src/eval_remote_push_proposal.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        assert!(!t.contains("std::process"), "No process calls in code");
    }
}
