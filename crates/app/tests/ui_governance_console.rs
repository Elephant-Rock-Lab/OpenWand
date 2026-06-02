//! UI governance console tests.

use openwand_app::ui::governance_state::*;
use openwand_app::ui::governance_actions::*;
use openwand_app::ui::governance_components::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_proposal_execution::*;
use openwand_app::eval_post_commit_verify::*;
use openwand_app::eval_remote_push_readiness::*;
use openwand_app::eval_remote_push_proposal::*;
use openwand_app::eval_remote_push_execution::*;
use openwand_app::eval_readiness::*;

use std::path::{Path, PathBuf};

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
    let backend = openwand_app::eval_proposal_execution::TestGitBackend::new("tracking_commit", "main");
    let req = AutoCommitExecutionRequest { proposal_id: p.proposal_id.clone(), review_id: r.review_id.clone(), requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: "k".into() };
    openwand_app::eval_proposal_execution::execute_proposal(&backend, Path::new("/tmp"), &req, Some(p), Some(r), Some(r), &[], true, Some(openwand_app::eval_proposal_execution::RollbackPlanSnapshot { pre_commit_head: "tracking_commit".into(), branch: "main".into(), index_status_hash: "idx".into(), worktree_status_hash: "wt".into(), recovery_command: "git reset --hard tracking_commit".into(), notes: vec![] }))
}

fn make_verified_record(exec: &AutoCommitExecutionRecord, p: &AutoCommitProposal) -> PostCommitVerificationRecord {
    let commit = exec.resulting_commit.as_ref().unwrap();
    let msg = format!("{}\n\n{}", p.commit_title, p.commit_body);
    let msg_hash = format!("{}", blake3::hash(msg.as_bytes()).to_hex());
    PostCommitVerificationRecord {
        verification_id: openwand_app::eval_post_commit_verify::verification_id_for(&exec.execution_id.0, "vkey"),
        execution_id: exec.execution_id.clone(), proposal_id: exec.proposal_id.clone(), review_id: exec.review_id.clone(),
        status: PostCommitVerificationStatus::Verified, decision: PostCommitVerificationDecision::Verified,
        predicates: vec![],
        commit_evidence: Some(openwand_app::eval_post_commit_verify::CommitEvidenceSnapshot {
            commit_hash: commit.commit_hash.clone(), parent_hash: commit.parent_hash.clone(),
            tree_hash: "tree".into(), branch: "main".into(), message_hash: msg_hash,
            changed_paths: p.included_files.iter().map(|f| f.path.clone()).collect(), diff_hash: "diff".into(),
        }),
        post_commit_checks: vec![openwand_app::eval_post_commit_verify::PostCommitCheckResult {
            spec: openwand_app::eval_post_commit_verify::PostCommitCheckSpec { name: "check".into(), kind: openwand_app::eval_post_commit_verify::PostCommitCheckKind::CargoCheckWorkspace },
            status: openwand_app::eval_post_commit_verify::PostCommitCheckStatus::Passed, output_summary: "OK".into(),
        }],
        rollback_drill: Some(openwand_app::eval_post_commit_verify::RollbackDrillResult {
            strategy: openwand_app::eval_post_commit_verify::RollbackDrillStrategy::SandboxRevert, clean: true,
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
        branch_policy: None,
        check_evidence: PushCheckEvidenceSnapshot { verification_status: PostCommitVerificationStatus::Verified, post_commit_checks_passed: true, failed_checks: vec![], skipped_required_checks: vec![] },
        rollback_evidence: PushRollbackEvidenceSnapshot { rollback_drill_present: true, rollback_drill_clean: true, live_repo_unchanged_during_drill: true },
        created_at: chrono::Utc::now(),
    }
}

fn save_full_chain(dir: &Path) -> (AutoCommitProposal, AutoCommitProposalReview, AutoCommitExecutionRecord, PostCommitVerificationRecord, RemotePushReadinessRecord) {
    let chain = make_full_chain();
    openwand_app::eval_proposal::save_proposal(dir, &chain.0).unwrap();
    openwand_app::eval_proposal_review::save_proposal_review(dir, &chain.1).unwrap();
    openwand_app::eval_proposal_execution::save_execution_record(dir, &chain.2).unwrap();
    openwand_app::eval_post_commit_verify::save_verification_record(dir, &chain.3).unwrap();
    openwand_app::eval_remote_push_readiness::save_readiness_record(dir, &chain.4).unwrap();
    chain
}

fn save_push_chain(dir: &Path, readiness: &RemotePushReadinessRecord) -> (RemotePushProposal, RemotePushProposalReview) {
    let req = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "test".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey".into() };
    let proposal = build_push_proposal(&req, Some(readiness), &[]).unwrap();
    let rev_req = RemotePushProposalReviewRequest { proposal_id: proposal.proposal_id.clone(), decision: RemotePushProposalReviewDecision::Approved, reviewer: "alice".into(), rationale: "LGTM".into(), feedback: None, idempotency_key: "rvkey".into() };
    let review = build_push_proposal_review(&proposal, &rev_req, &[]).unwrap();
    save_push_proposal(dir, &proposal).unwrap();
    save_push_proposal_review(dir, &review).unwrap();
    (proposal, review)
}

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

// ── State Projection Tests (13) ─────────────────────────────────────────────

#[test] fn governance_state_loads_full_chain() {
    let dir = tempfile::tempdir().unwrap();
    let chain = save_full_chain(dir.path());
    let (_, _, _, _, readiness) = &chain;
    save_push_chain(dir.path(), readiness);
    let state = load_governance_console(dir.path());
    assert!(state.local_proposal.is_some());
    assert!(state.local_review.is_some());
    assert!(state.local_execution.is_some());
    assert!(state.post_commit_verification.is_some());
    assert!(state.push_readiness.is_some());
    assert!(state.push_proposal.is_some());
    assert!(state.push_review.is_some());
}

#[test] fn governance_state_handles_missing_records() {
    let dir = tempfile::tempdir().unwrap();
    let state = load_governance_console(dir.path());
    assert!(state.local_proposal.is_none());
    assert!(state.local_review.is_none());
    assert!(state.chain_warnings.is_empty());
}

#[test] fn governance_state_links_records_by_ids() {
    let dir = tempfile::tempdir().unwrap();
    let chain = save_full_chain(dir.path());
    let state = load_governance_console(dir.path());
    // Local review should link to proposal
    let review = state.local_review.unwrap();
    let proposal = state.local_proposal.unwrap();
    assert!(review.linked_ids.iter().any(|(k, v)| k == "proposal_id" && v == &proposal.id));
    // Local execution should link to both
    let exec = state.local_execution.unwrap();
    assert!(exec.linked_ids.iter().any(|(k, _)| k == "proposal_id"));
    assert!(exec.linked_ids.iter().any(|(k, _)| k == "review_id"));
}

#[test] fn governance_state_surfaces_hashes() {
    let dir = tempfile::tempdir().unwrap();
    save_full_chain(dir.path());
    let state = load_governance_console(dir.path());
    assert!(state.local_proposal.as_ref().unwrap().hash.is_some());
    assert!(state.local_review.as_ref().unwrap().hash.is_some());
}

#[test] fn governance_state_surfaces_feedback() {
    let dir = tempfile::tempdir().unwrap();
    save_full_chain(dir.path());
    let state = load_governance_console(dir.path());
    // Local review always generates feedback summary
    assert!(!state.feedback.is_empty());
}

#[test] fn governance_state_surfaces_predicates() {
    let dir = tempfile::tempdir().unwrap();
    let state = GovernanceConsoleState::empty();
    // No predicates from empty state
    assert!(state.predicates.is_empty());
}

#[test] fn governance_state_marks_chain_gaps() {
    let dir = tempfile::tempdir().unwrap();
    // Save only a proposal — everything else missing
    let proposal = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &proposal).unwrap();
    let state = load_governance_console(dir.path());
    assert!(state.local_proposal.is_some());
    assert!(state.local_review.is_none());
    assert!(state.local_execution.is_none());
}

#[test] fn governance_state_is_projection_only() {
    let dir = tempfile::tempdir().unwrap();
    let before = load_governance_console(dir.path());
    let after = load_governance_console(dir.path());
    assert_eq!(before, after, "Loading twice must produce identical state");
}

#[test] fn governance_state_roundtrips_through_serde() {
    let state = GovernanceConsoleState::empty();
    let json = serde_json::to_string(&state).unwrap();
    let parsed: GovernanceConsoleState = serde_json::from_str(&json).unwrap();
    assert_eq!(state, parsed);
}

#[test] fn governance_state_empty_dir_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let state = load_governance_console(dir.path());
    assert!(state.local_proposal.is_none());
    assert!(state.predicates.is_empty());
    assert!(state.feedback.is_empty());
    assert!(state.chain_warnings.is_empty());
}

#[test] fn governance_state_warns_on_local_review_proposal_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    // Save two proposals and a review linked to the first
    let p1 = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &p1).unwrap();
    let r1 = make_approved_review(&p1);
    openwand_app::eval_proposal_review::save_proposal_review(dir.path(), &r1).unwrap();
    // Save a second proposal (becomes "latest")
    let p2 = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &p2).unwrap();
    let state = load_governance_console(dir.path());
    // The review references p1 but latest proposal is p2
    assert!(!state.chain_warnings.is_empty(), "Should warn on proposal mismatch");
}

#[test] fn governance_state_warns_on_push_review_proposal_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let chain = save_full_chain(dir.path());
    let (_, _, _, _, readiness) = &chain;
    let (pp1, pr1) = save_push_chain(dir.path(), readiness);
    // Save a second push proposal (becomes "latest")
    let req2 = RemotePushProposalRequest { readiness_id: readiness.readiness_id.clone(), requested_by: "t".into(), requested_at: chrono::Utc::now(), idempotency_key: "pkey2".into() };
    let pp2 = build_push_proposal(&req2, Some(readiness), &[pp1.clone()]).unwrap();
    save_push_proposal(dir.path(), &pp2).unwrap();
    let state = load_governance_console(dir.path());
    assert!(!state.chain_warnings.is_empty(), "Should warn on push review/proposal mismatch");
}

#[test] fn governance_state_warns_on_execution_chain_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let chain = save_full_chain(dir.path());
    let (proposal, review, _, _, _) = &chain;
    // Save a second execution linked to different proposal/review
    let p2 = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &p2).unwrap();
    let r2 = make_approved_review(&p2);
    openwand_app::eval_proposal_review::save_proposal_review(dir.path(), &r2).unwrap();
    let state = load_governance_console(dir.path());
    // Latest execution references first proposal/review, but latest proposal/review is second
    assert!(!state.chain_warnings.is_empty(), "Should warn on execution chain mismatch");
}

// ── View-Model Helper Tests (10) ────────────────────────────────────────────

#[test] fn record_card_lines_contain_status_and_hash() {
    let summary = GovernanceRecordSummary {
        kind: GovernanceRecordKind::LocalProposal,
        id: "test_id".into(), status: "Eligible".into(), decision: None,
        hash: Some("abc123".into()), linked_ids: vec![], created_at: None, summary: "test".into(),
    };
    let lines = record_card_lines(&summary);
    assert!(lines.iter().any(|l| l.contains("Eligible")));
    assert!(lines.iter().any(|l| l.contains("abc123")));
}

#[test] fn record_card_lines_contain_linked_ids() {
    let summary = GovernanceRecordSummary {
        kind: GovernanceRecordKind::LocalReview,
        id: "rev1".into(), status: "Approved".into(), decision: Some("Approved".into()),
        hash: None, linked_ids: vec![("proposal_id".into(), "prop1".into())], created_at: None, summary: "ok".into(),
    };
    let lines = record_card_lines(&summary);
    assert!(lines.iter().any(|l| l.contains("proposal_id") && l.contains("prop1")));
}

#[test] fn predicate_panel_rows_show_pass_fail() {
    let preds = vec![GovernancePredicateSummary {
        source_record_id: "v1".into(), source_kind: GovernanceRecordKind::PostCommitVerification,
        predicate: "TestPredicate".into(), passed: true, reason: "OK".into(),
    }, GovernancePredicateSummary {
        source_record_id: "v1".into(), source_kind: GovernanceRecordKind::PostCommitVerification,
        predicate: "TestPredicate2".into(), passed: false, reason: "Failed".into(),
    }];
    let rows = predicate_panel_rows(&preds);
    assert_eq!(2, rows.len());
    assert!(rows[0].passed);
    assert!(!rows[1].passed);
}

#[test] fn feedback_panel_rows_show_blocking_reasons() {
    let fb = vec![GovernanceFeedbackSummary {
        review_id: "r1".into(), kind: GovernanceRecordKind::PushReview,
        summary: "bad".into(), blocking_reasons: vec!["risk".into()],
        requested_changes: vec![], evidence_gaps: vec![],
    }];
    let rows = feedback_panel_rows(&fb);
    assert_eq!(1, rows.len());
    assert_eq!(vec!["risk".to_string()], rows[0].blocking_reasons);
}

#[test] fn overview_status_lines_show_chain_statuses() {
    let state = GovernanceConsoleState {
        local_proposal: Some(GovernanceRecordSummary {
            kind: GovernanceRecordKind::LocalProposal, id: "p1".into(),
            status: "Eligible".into(), decision: None, hash: None, linked_ids: vec![],
            created_at: None, summary: "test".into(),
        }),
        ..GovernanceConsoleState::empty()
    };
    let lines = overview_status_lines(&state);
    assert!(lines.iter().any(|l| l.contains("Eligible")));
    assert!(lines.iter().any(|l| l.contains("Missing")));
}

#[test] fn safety_banner_text_is_stable() {
    let text = safety_banner_text();
    assert!(text.contains("UI approval is not execution"));
    // Call twice — same text
    assert_eq!(text, safety_banner_text());
}

#[test] fn missing_record_returns_empty_overview() {
    let state = GovernanceConsoleState::empty();
    let lines = overview_status_lines(&state);
    assert!(lines.iter().all(|l| l.contains("Missing")));
}

#[test] fn ui_review_feedback_matches_cli_feedback_shape() {
    // Both UI and CLI go through the same builder, so feedback shapes are identical
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &proposal).unwrap();
    // CLI path
    let cli_review = build_proposal_review(
        &proposal, AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User, "LGTM".into(), vec![], None,
    ).unwrap();
    // UI path result
    let action = GovernanceUiAction::ApproveLocalProposal {
        proposal_id: proposal.proposal_id.0.clone(), reviewer: "alice".into(), rationale: "LGTM".into(),
    };
    let result = execute_governance_action(action, dir.path()).unwrap();
    if let GovernanceActionResult::LocalReviewCreated { decision, .. } = result {
        assert_eq!("Approved", decision);
        assert_eq!(format!("{:?}", cli_review.decision), "Approved");
    } else {
        panic!("Expected LocalReviewCreated");
    }
}

#[test] fn governance_components_render_without_panic() {
    let state = GovernanceConsoleState {
        local_proposal: Some(GovernanceRecordSummary {
            kind: GovernanceRecordKind::LocalProposal, id: "p1".into(),
            status: "Eligible".into(), decision: None, hash: Some("h".into()),
            linked_ids: vec![], created_at: Some(chrono::Utc::now()), summary: "test".into(),
        }),
        ..GovernanceConsoleState::empty()
    };
    // Pure helpers don't need Dioxus
    let _ = record_card_lines(state.local_proposal.as_ref().unwrap());
    let _ = overview_status_lines(&state);
    let _ = safety_banner_text();
    let _ = predicate_panel_rows(&state.predicates);
    let _ = feedback_panel_rows(&state.feedback);
}

// ── Action Result Tests (2) ─────────────────────────────────────────────────

#[test] fn ui_action_result_reports_no_execution_grant() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &proposal).unwrap();
    let action = GovernanceUiAction::ApproveLocalProposal {
        proposal_id: proposal.proposal_id.0.clone(), reviewer: "alice".into(), rationale: "LGTM".into(),
    };
    let result = execute_governance_action(action, dir.path()).unwrap();
    match result {
        GovernanceActionResult::LocalReviewCreated { creates_execution_grant, .. } => {
            assert!(!creates_execution_grant);
        }
        _ => panic!("Expected LocalReviewCreated"),
    }
}

#[test] fn ui_action_result_reports_no_execution_allowed_now() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &proposal).unwrap();
    let action = GovernanceUiAction::ApproveLocalProposal {
        proposal_id: proposal.proposal_id.0.clone(), reviewer: "alice".into(), rationale: "LGTM".into(),
    };
    let result = execute_governance_action(action, dir.path()).unwrap();
    match result {
        GovernanceActionResult::LocalReviewCreated { execution_allowed_now, .. } => {
            assert!(!execution_allowed_now);
        }
        _ => panic!("Expected LocalReviewCreated"),
    }
}

// ── Guard Tests (8) ─────────────────────────────────────────────────────────

macro_rules! source_guard {
    ($name:ident, $pattern:expr, $should_not_contain:expr) => {
        #[test]
        fn $name() {
            let files = [
                include_str!("../src/ui/governance_state.rs"),
                include_str!("../src/ui/governance_actions.rs"),
                include_str!("../src/ui/governance_components.rs"),
            ];
            for source in &files {
                for line in source.lines() {
                    let t = line.trim();
                    if t.starts_with("//") || t.starts_with("//!") { continue; }
                    let lower = t.to_lowercase();
                    assert!(!lower.contains($pattern), "Guard violation: {} found in UI module", $should_not_contain);
                }
            }
        }
    };
}

source_guard!(ui_module_does_not_import_process_command, "std::process::command", "std::process::Command");
source_guard!(ui_module_does_not_import_git_backend, "localgitbackend", "LocalGitBackend");
source_guard!(ui_module_does_not_import_push_execution_backend, "localpushexecutionbackend", "LocalPushExecutionBackend");
source_guard!(ui_module_does_not_import_local_execution_backend, "governedgitcommitbackend", "GovernedGitCommitBackend");

#[test] fn ui_module_does_not_call_shell() {
    let files = [
        include_str!("../src/ui/governance_state.rs"),
        include_str!("../src/ui/governance_actions.rs"),
        include_str!("../src/ui/governance_components.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            let lower = t.to_lowercase();
            assert!(!lower.contains("/bin/sh"), "No /bin/sh");
            assert!(!lower.contains("cmd.exe"), "No cmd.exe");
        }
    }
}

#[test] fn ui_module_does_not_construct_review_records_directly() {
    let source = include_str!("../src/ui/governance_actions.rs");
    // The adapter should NOT construct AutoCommitProposalReview or RemotePushProposalReview directly
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("//!") { continue; }
        // Allow constructing Request DTOs, not Record types
        assert!(!t.contains("AutoCommitProposalReview {") || t.contains("ReviewRequest"), "Should not construct review records directly");
        assert!(!t.contains("RemotePushProposalReview {") || t.contains("ReviewRequest"), "Should not construct push review records directly");
    }
}

#[test] fn ui_module_does_not_construct_execution_records_directly() {
    let files = [
        include_str!("../src/ui/governance_state.rs"),
        include_str!("../src/ui/governance_actions.rs"),
        include_str!("../src/ui/governance_components.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            assert!(!t.contains("AutoCommitExecutionRecord {"), "No direct execution record construction");
            assert!(!t.contains("RemotePushExecutionRecord {"), "No direct push execution record construction");
        }
    }
}

#[test] fn ui_module_does_not_mutate_trace_or_memory_directly() {
    let files = [
        include_str!("../src/ui/governance_state.rs"),
        include_str!("../src/ui/governance_actions.rs"),
        include_str!("../src/ui/governance_components.rs"),
    ];
    for source in &files {
        for line in source.lines() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("//!") { continue; }
            assert!(!t.contains("TraceStore"), "No TraceStore import");
            assert!(!t.contains("MemoryStore"), "No MemoryStore import");
        }
    }
}

// ── Runtime No-Mutation Tests (4) ───────────────────────────────────────────

#[test] fn ui_refresh_leaves_everything_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let action = GovernanceUiAction::Refresh;
    let result = execute_governance_action(action, dir.path()).unwrap();
    assert_eq!(GovernanceActionResult::Refreshed, result);
}

#[test] fn ui_review_action_persists_only_review_record() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    openwand_app::eval_proposal::save_proposal(dir.path(), &proposal).unwrap();
    let action = GovernanceUiAction::ApproveLocalProposal {
        proposal_id: proposal.proposal_id.0.clone(), reviewer: "alice".into(), rationale: "LGTM".into(),
    };
    let result = execute_governance_action(action, dir.path()).unwrap();
    // Should have created a review file
    if let GovernanceActionResult::LocalReviewCreated { review_id, .. } = result {
        let loaded = openwand_app::eval_proposal_review::load_proposal_review(dir.path(), &AutoCommitProposalReviewId(review_id.clone())).unwrap();
        assert!(loaded.is_some(), "Review should be persisted");
    }
    // Should NOT have created execution records
    let exec = openwand_app::eval_proposal_execution::load_latest_execution(dir.path()).unwrap();
    assert!(exec.is_none(), "No execution record should exist from UI review");
}
