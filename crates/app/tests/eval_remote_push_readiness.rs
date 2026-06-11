//! Governed remote push readiness gate tests.

use openwand_app::eval_remote_push_readiness::*;
use openwand_app::eval_post_commit_verify::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_execution::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_readiness::*;

// ── Shared helpers ──────────────────────────────────────────────────────────

fn make_eligible_proposal() -> AutoCommitProposal {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report();
    let inputs = AutoCommitProposalInputs { readiness: &readiness, workspace_digest: &workspace, eval_report: &eval, comparison: None };
    build_auto_commit_proposal(inputs)
}

fn make_approved_review(proposal: &AutoCommitProposal) -> AutoCommitProposalReview {
    build_proposal_review(proposal, AutoCommitProposalReviewDecision::Approved, AutoCommitProposalReviewer::User, "OK".into(), vec![], None).unwrap()
}

fn make_executed_record(proposal: &AutoCommitProposal, review: &AutoCommitProposalReview) -> AutoCommitExecutionRecord {
    let backend = TestGitBackend::new("tracking_commit", "main");
    let req = AutoCommitExecutionRequest {
        proposal_id: proposal.proposal_id.clone(), review_id: review.review_id.clone(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "k".into(),
    };
    execute_proposal(&backend, std::path::Path::new("/tmp"), &req, Some(proposal), Some(review), Some(review), &[], true, Some(RollbackPlanSnapshot {
        pre_commit_head: "tracking_commit".into(), branch: "main".into(),
        index_status_hash: "idx".into(), worktree_status_hash: "wt".into(),
        recovery_command: "git reset --hard tracking_commit".into(), notes: vec![],
    }))
}

fn make_verified_record(exec: &AutoCommitExecutionRecord, proposal: &AutoCommitProposal) -> PostCommitVerificationRecord {
    let commit = exec.resulting_commit.as_ref().unwrap();
    let msg = format!("{}\n\n{}", proposal.commit_title, proposal.commit_body);
    let msg_hash = format!("{}", blake3::hash(msg.as_bytes()).to_hex());
    let evidence = CommitEvidenceSnapshot {
        commit_hash: commit.commit_hash.clone(), parent_hash: commit.parent_hash.clone(),
        tree_hash: "tree".into(), branch: "main".into(), message_hash: msg_hash,
        changed_paths: proposal.included_files.iter().map(|f| f.path.clone()).collect(),
        diff_hash: "diff".into(),
    };
    PostCommitVerificationRecord {
        verification_id: verification_id_for(&exec.execution_id.0, "vkey"),
        execution_id: exec.execution_id.clone(),
        proposal_id: exec.proposal_id.clone(),
        review_id: exec.review_id.clone(),
        status: PostCommitVerificationStatus::Verified,
        decision: PostCommitVerificationDecision::Verified,
        predicates: vec![],
        commit_evidence: Some(evidence),
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

fn make_readiness_request(verification_id: &PostCommitVerificationId) -> RemotePushReadinessRequest {
    RemotePushReadinessRequest {
        verification_id: verification_id.clone(),
        target_remote: "origin".into(), target_branch: "main".into(),
        requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "rkey".into(),
    }
}

fn make_matching_backend(verified: &PostCommitVerificationRecord) -> TestPushReadinessBackend {
    TestPushReadinessBackend::new_ready().with_branch_state(LocalBranchPushSnapshot {
        current_head: verified.commit_evidence.as_ref().map(|e| e.commit_hash.clone()).unwrap_or_default(),
        current_branch: "main".into(), target_remote: "origin".into(), target_branch: "main".into(),
        upstream_ref: Some("refs/remotes/origin/main".into()),
        remote_tracking_ref: Some("refs/remotes/origin/main".into()),
        ahead_count: 1, behind_count: 0, diverged: false, worktree_clean: true, index_clean: true,
    })
}

fn make_full_chain() -> (AutoCommitProposal, AutoCommitProposalReview, AutoCommitExecutionRecord, PostCommitVerificationRecord) {
    let proposal = make_eligible_proposal();
    let review = make_approved_review(&proposal);
    let exec = make_executed_record(&proposal, &review);
    let verified = make_verified_record(&exec, &proposal);
    (proposal, review, exec, verified)
}

// Readiness/workspace/eval helpers
fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(), report_schema_version: 1, target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore { weighted_pass_rate: 0.95, patch_pass_rate: 0.98, policy_pass_rate: 1.0, rebuild_pass_rate: 1.0, explain_pass_rate: 0.95, capability_context_pass_rate: 1.0, regression_count: 0 },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow { total_reports_found: 15, reports_used: 15, reports_skipped_incompatible: 0, scenario_ids_covered: vec!["test".into()], earliest_report: None, latest_report: None },
        scenario_results: vec![], blockers: vec![], warnings: vec![],
    }
}

fn make_workspace_digest(hash: &str) -> WorkspaceSnapshotDigest {
    WorkspaceSnapshotDigest { blake3_hash: hash.into(), file_count: 5, generated_at: chrono::Utc::now(), file_digests: vec![] }
}

fn make_eval_report() -> openwand_app::eval_model::EvalRunReport {
    use openwand_app::eval_model::*;
    EvalRunReport {
        report_schema_version: 2, scenario_id: "test".into(),
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

#[test] fn readiness_request_roundtrips() {
    let req = make_readiness_request(&verification_id_for("v", "k"));
    let json = serde_json::to_string(&req).unwrap();
    let parsed: RemotePushReadinessRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.target_remote, parsed.target_remote);
}

#[test] fn readiness_record_roundtrips() {
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v", "origin", "main", "k"),
        verification_id: verification_id_for("v", "k"),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "abc".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&r).unwrap();
    let parsed: RemotePushReadinessRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(r.readiness_id, parsed.readiness_id);
    assert_eq!(r.status, parsed.status);
}

#[test] fn ready_record_requires_verified_evidence() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocked_record_requires_reason() {
    let d = RemotePushReadinessDecision::Blocked { reason_code: "x".into(), summary: "y".into() };
    if let RemotePushReadinessDecision::Blocked { reason_code, summary } = d {
        assert!(!reason_code.is_empty()); assert!(!summary.is_empty());
    }
}

#[test] fn inconclusive_record_requires_reason() {
    let d = RemotePushReadinessDecision::Inconclusive { reason_code: "x".into(), summary: "y".into() };
    if let RemotePushReadinessDecision::Inconclusive { reason_code, summary } = d {
        assert!(!reason_code.is_empty()); assert!(!summary.is_empty());
    }
}

#[test] fn readiness_id_is_content_addressed() {
    let a = readiness_id_for("v", "origin", "main", "k1");
    let b = readiness_id_for("v", "origin", "main", "k1");
    let c = readiness_id_for("v", "origin", "main", "k2");
    assert_eq!(a, b); assert_ne!(a, c);
}

#[test] fn readiness_id_is_deterministic() {
    for _ in 0..5 { assert_eq!("rpr_", &readiness_id_for("v", "r", "b", "k").0[..4]); }
}

#[test] fn readiness_id_differs_by_remote_and_branch() {
    let a = readiness_id_for("v", "origin", "main", "k");
    let b = readiness_id_for("v", "upstream", "main", "k");
    let c = readiness_id_for("v", "origin", "dev", "k");
    assert_ne!(a, b); assert_ne!(a, c); assert_ne!(b, c);
}

// ── Predicate Tests (19) ────────────────────────────────────────────────────

#[test] fn blocks_missing_verification_record() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, None, &[]);
    assert_eq!(RemotePushReadinessStatus::Blocked, r.status);
}

#[test] fn blocks_non_verified_verification_record() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    verified.status = PostCommitVerificationStatus::Failed;
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_current_head_not_verified_commit() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified).with_branch_state(LocalBranchPushSnapshot {
        current_head: "WRONG_HEAD".into(), ..make_matching_backend(&verified).branch_state.clone()
    });
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_dirty_worktree() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.worktree_clean = false;
    let backend = make_matching_backend(&verified).with_branch_state(bs);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_dirty_index() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.index_clean = false;
    let backend = make_matching_backend(&verified).with_branch_state(bs);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_missing_target_remote() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified).with_remote_url(false);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_branch_behind_remote() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.behind_count = 3;
    let backend = make_matching_backend(&verified).with_branch_state(bs);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_branch_diverged_from_remote() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.diverged = true; bs.ahead_count = 2; bs.behind_count = 1;
    let backend = make_matching_backend(&verified).with_branch_state(bs);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_protected_branch_direct_push() {
    let (_, _, _, verified) = make_full_chain();
    let policy = BranchProtectionPolicySnapshot {
        protected_branch: true, direct_push_allowed: false, ..TestPushReadinessBackend::new_ready().policy.clone()
    };
    let backend = make_matching_backend(&verified).with_policy(policy);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_failed_post_commit_checks() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    verified.post_commit_checks = vec![PostCommitCheckResult {
        spec: PostCommitCheckSpec { name: "test".into(), kind: PostCommitCheckKind::CargoCheckWorkspace },
        status: PostCommitCheckStatus::Failed, output_summary: "error".into(),
    }];
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_skipped_required_checks() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    verified.post_commit_checks = vec![PostCommitCheckResult {
        spec: PostCommitCheckSpec { name: "test".into(), kind: PostCommitCheckKind::CargoCheckWorkspace },
        status: PostCommitCheckStatus::Skipped, output_summary: "skipped".into(),
    }];
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_missing_rollback_drill() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    verified.rollback_drill = None;
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_unclean_rollback_drill() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    if let Some(ref mut drill) = verified.rollback_drill { drill.clean = false; drill.conflicts = vec!["conflict".into()]; }
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_live_repo_changed_during_drill() {
    let (_, _, exec, _) = make_full_chain();
    let mut verified = make_verified_record(&exec, &make_eligible_proposal());
    if let Some(ref mut drill) = verified.rollback_drill { drill.live_head_after = "MUTATED".into(); }
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn ready_when_verified_clean_ahead_and_policy_allows() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Ready, r.status);
    for p in &r.predicates { assert!(p.passed, "Predicate {:?} failed: {}", p.predicate, p.reason); }
}

#[test] fn blocks_commit_hash_mismatch() {
    let (_, _, _, verified) = make_full_chain();
    let mut v2 = verified.clone();
    v2.commit_evidence = Some(CommitEvidenceSnapshot { commit_hash: "WRONG_HASH".into(), ..verified.commit_evidence.clone().unwrap() });
    let backend = make_matching_backend(&verified); // HEAD matches original, not WRONG_HASH
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&v2), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn blocks_branch_not_ahead() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.ahead_count = 0;
    let backend = make_matching_backend(&verified).with_branch_state(bs);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
}

#[test] fn inconclusive_when_tracking_ref_missing() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.remote_tracking_ref = None; bs.ahead_count = 0;
    let mut backend = make_matching_backend(&verified).with_branch_state(bs);
    backend.tracking_state.tracking_commit = None;
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Inconclusive, r.status);
}

#[test] fn target_remote_configured_from_local_git_config() {
    // Test backend reports remote_url_exists=true → predicate passes
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified).with_remote_url(true);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    let pred = r.predicates.iter().find(|p| p.predicate == RemotePushPredicate::TargetRemoteConfigured).unwrap();
    assert!(pred.passed);
}

// ── Branch Policy Tests (6) ─────────────────────────────────────────────────

#[test] fn default_policy_protects_main() {
    let policy = default_policy("main");
    assert!(policy.protected_branch);
    assert!(!policy.direct_push_allowed);
}

#[test] fn default_policy_allows_non_protected_branch() {
    let policy = default_policy("feature/test");
    assert!(!policy.protected_branch);
    assert!(policy.direct_push_allowed);
}

#[test] fn protected_branch_requires_explicit_policy_allow() {
    let rule = PushPolicyRule { pattern: "main".into(), protected_branch: true, direct_push_allowed: false, ..default_policy_rule() };
    let snapshot = rule.to_snapshot("main", "test");
    assert!(!snapshot.direct_push_allowed);
}

#[test] fn branch_pattern_matching_exact_wins() {
    let rules = vec![
        PushPolicyRule { pattern: "main".into(), protected_branch: true, direct_push_allowed: false, ..default_policy_rule() },
        PushPolicyRule { pattern: "wave/*".into(), protected_branch: false, direct_push_allowed: true, ..default_policy_rule() },
    ];
    let result = LocalPushReadinessBackend::select_policy(&rules, "main");
    assert!(!result.direct_push_allowed);
    assert_eq!("push_policy:exact_match", result.policy_source);
}

#[test] fn branch_pattern_matching_wildcard() {
    let rules = vec![
        PushPolicyRule { pattern: "wave/*".into(), protected_branch: false, direct_push_allowed: true, ..default_policy_rule() },
    ];
    let result = LocalPushReadinessBackend::select_policy(&rules, "wave/15");
    assert!(result.direct_push_allowed);
    assert!(result.policy_source.contains("wildcard_match"));
}

#[test] fn ambiguous_pattern_blocks() {
    let rules = vec![
        PushPolicyRule { pattern: "wave/*".into(), protected_branch: false, direct_push_allowed: true, ..default_policy_rule() },
        PushPolicyRule { pattern: "wave/*".into(), protected_branch: true, direct_push_allowed: false, ..default_policy_rule() },
    ];
    let result = LocalPushReadinessBackend::select_policy(&rules, "wave/15");
    assert!(result.policy_source.contains("ambiguous"));
}

fn default_policy_rule() -> PushPolicyRule {
    PushPolicyRule { pattern: String::new(), protected_branch: false, direct_push_allowed: true, requires_verified_commit: true, requires_clean_rollback_drill: true, requires_post_commit_checks: true, requires_no_behind_remote: true, requires_no_divergence: true, requires_protected_branch_approval: false }
}

// ── Persistence and Idempotency (6) ─────────────────────────────────────────

#[test] fn readiness_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v", "origin", "main", "k"), verification_id: verification_id_for("v", "k"),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "abc".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    };
    save_readiness_record(dir.path(), &r).unwrap();
    let loaded = load_readiness_record(dir.path(), &r.readiness_id).unwrap().unwrap();
    assert_eq!(r.readiness_id, loaded.readiness_id);
}

#[test] fn latest_readiness_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v", "origin", "main", "k"), verification_id: verification_id_for("v", "k"),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "abc".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Blocked, decision: RemotePushReadinessDecision::Blocked { reason_code: "x".into(), summary: "y".into() },
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Failed, post_commit_checks_passed: false, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: false, rollback_drill_clean: false, live_repo_unchanged_during_drill: false },
        created_at: chrono::Utc::now(),
    };
    save_readiness_record(dir.path(), &r).unwrap();
    let latest = load_latest_readiness(dir.path()).unwrap().unwrap();
    assert_eq!(r.readiness_id, latest.readiness_id);
}

#[test] fn latest_readiness_for_verification_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let vid = verification_id_for("v42", "k");
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v42", "origin", "main", "k"), verification_id: vid.clone(),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "abc".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    };
    save_readiness_record(dir.path(), &r).unwrap();
    let loaded = load_latest_readiness_for_verification(dir.path(), &vid).unwrap().unwrap();
    assert_eq!(r.readiness_id, loaded.readiness_id);
}

#[test] fn latest_readiness_for_commit_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v", "origin", "main", "k"), verification_id: verification_id_for("v", "k"),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "deadbeef".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    };
    save_readiness_record(dir.path(), &r).unwrap();
    let loaded = load_latest_readiness_for_commit(dir.path(), "deadbeef").unwrap().unwrap();
    assert_eq!(r.readiness_id, loaded.readiness_id);
}

#[test] fn same_idempotency_key_returns_existing_readiness() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let first = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Ready, first.status);
    let second = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[first.clone()]);
    assert_eq!(first.readiness_id, second.readiness_id);
}

#[test] fn list_readiness_records_sorted_by_date() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let r = RemotePushReadinessRecord {
            readiness_id: readiness_id_for(&format!("v{}", i), "origin", "main", "k"),
            verification_id: verification_id_for(&format!("v{}", i), "k"),
            execution_id: AutoCommitExecutionId(format!("e{}", i)), proposal_id: AutoCommitProposalId(format!("p{}", i)),
            review_id: AutoCommitProposalReviewId(format!("r{}", i)), commit_hash: format!("c{}", i),
            target_remote: "origin".into(), target_branch: "main".into(),
            status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
            predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
            check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
            rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
            created_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
        };
        save_readiness_record(dir.path(), &r).unwrap();
    }
    let loaded = list_readiness_records(dir.path()).unwrap();
    assert_eq!(3, loaded.len());
    assert!(loaded[0].created_at >= loaded[1].created_at);
}

// ── CLI Tests (6) ───────────────────────────────────────────────────────────

#[test] fn cli_push_readiness_ready_outputs_ready() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Ready, r.status);
    assert!(matches!(r.decision, RemotePushReadinessDecision::Ready));
}

#[test] fn cli_push_readiness_blocked_outputs_predicates() {
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified).with_remote_url(false);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_ne!(RemotePushReadinessStatus::Ready, r.status);
    assert!(!r.predicates.is_empty());
    let failed: Vec<_> = r.predicates.iter().filter(|p| !p.passed).collect();
    assert!(!failed.is_empty());
    for p in &r.predicates { assert!(!p.reason.is_empty()); }
}

#[test] fn cli_push_readiness_inconclusive_outputs_reason() {
    let (_, _, _, verified) = make_full_chain();
    let mut bs = make_matching_backend(&verified).branch_state.clone();
    bs.remote_tracking_ref = None; bs.ahead_count = 0;
    let mut backend = make_matching_backend(&verified).with_branch_state(bs);
    backend.tracking_state.tracking_commit = None;
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    assert_eq!(RemotePushReadinessStatus::Inconclusive, r.status);
    if let RemotePushReadinessDecision::Inconclusive { reason_code, summary } = &r.decision {
        assert!(!reason_code.is_empty()); assert!(!summary.is_empty());
    } else { panic!("Expected Inconclusive"); }
}

#[test] fn cli_push_readiness_show_roundtrips_record() {
    let dir = tempfile::tempdir().unwrap();
    let (_, _, _, verified) = make_full_chain();
    let backend = make_matching_backend(&verified);
    let req = make_readiness_request(&verified.verification_id);
    let r = evaluate_push_readiness(&backend, std::path::Path::new("/tmp"), &req, Some(&verified), &[]);
    save_readiness_record(dir.path(), &r).unwrap();
    let loaded = load_readiness_record(dir.path(), &r.readiness_id).unwrap().unwrap();
    assert_eq!(r.readiness_id, loaded.readiness_id); assert_eq!(r.status, loaded.status);
}

#[test] fn cli_does_not_expose_push_command() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git push"), "No git push");
    }
}

#[test] fn cli_push_readiness_latest_by_verification_and_commit() {
    let dir = tempfile::tempdir().unwrap();
    let vid = verification_id_for("v1", "k");
    let r = RemotePushReadinessRecord {
        readiness_id: readiness_id_for("v1", "origin", "main", "k"), verification_id: vid.clone(),
        execution_id: AutoCommitExecutionId("e".into()), proposal_id: AutoCommitProposalId("p".into()),
        review_id: AutoCommitProposalReviewId("r".into()), commit_hash: "deadbeef".into(),
        target_remote: "origin".into(), target_branch: "main".into(),
        status: RemotePushReadinessStatus::Ready, decision: RemotePushReadinessDecision::Ready,
        predicates: vec![], local_branch: None, remote_tracking: None, branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    };
    save_readiness_record(dir.path(), &r).unwrap();
    assert!(load_latest_readiness_for_verification(dir.path(), &vid).unwrap().is_some());
    assert!(load_latest_readiness_for_commit(dir.path(), "deadbeef").unwrap().is_some());
}

// ── Source and Runtime Guard Tests (8) ──────────────────────────────────────

#[test] fn module_does_not_call_git_push() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        assert!(!t.to_lowercase().contains("git push"), "No git push");
    }
}

#[test] fn module_does_not_call_git_fetch_pull_or_ls_remote() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
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
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        // Patch 1: No git branch (uses symbolic-ref instead)
        assert!(!lower.contains("git branch"), "No git branch");
        assert!(!lower.contains("git tag"), "No git tag");
    }
}

#[test] fn module_does_not_call_release_or_remote_mutation_tools() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("git remote add"), "No remote add");
        assert!(!lower.contains("git remote set-url"), "No remote set-url");
        assert!(!lower.contains("git remote remove"), "No remote remove");
        assert!(!lower.contains("hub ") && !lower.contains("glab ") && !lower.contains("gh "), "No host tools");
    }
}

#[test] fn module_does_not_execute_arbitrary_shell() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        let lower = t.to_lowercase();
        assert!(!lower.contains("/bin/sh"), "No /bin/sh");
        assert!(!lower.contains("cmd.exe"), "No cmd.exe");
        assert!(!lower.contains(".shell("), "No .shell()");
    }
}

#[test] fn command_only_used_inside_readiness_backend() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    let command_lines: Vec<&str> = source.lines().filter(|l| l.contains("std::process::Command")).collect();
    assert!(command_lines.len() <= 2, "Command only in LocalPushReadinessBackend, found {} lines", command_lines.len());
}

#[test] fn readiness_backend_uses_fixed_allowed_commands() {
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    assert!(source.contains("Command::new(\"git\")"), "Only git binary");
    // Patch 1: Uses symbolic-ref, not branch
    assert!(source.contains("symbolic-ref"), "Uses symbolic-ref for branch name");
}

#[test] fn readiness_backend_uses_symbolic_ref_not_branch() {
    // Patch 1 explicit test
    let source = include_str!("../src/eval_remote_push_readiness.rs");
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        // No git branch command in code lines
        if t.contains("\"branch\"") && t.contains("Command") {
            panic!("Must not use git branch command");
        }
    }
}
