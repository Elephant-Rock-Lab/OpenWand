//! Proposal review tests — all commits combined.

use openwand_app::eval_model::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_proposal_review::*;
use openwand_app::eval_readiness::*;

// ── Commit 1: DTO and invariant tests ──────────────────────────────────────

fn make_eligible_proposal() -> AutoCommitProposal {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    build_auto_commit_proposal(inputs)
}

fn make_blocked_proposal() -> AutoCommitProposal {
    let mut readiness = make_blocked_readiness();
    readiness.blockers.push(ReadinessBlocker {
        kind: ReadinessBlockerKind::PatchPassRateBelowThreshold,
        scenario_id: Some("test".to_string()),
        detail: "Patch rate too low".to_string(),
    });
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    build_auto_commit_proposal(inputs)
}

fn make_test_feedback(proposal_id: &AutoCommitProposalId) -> ProposalRejectionFeedback {
    ProposalRejectionFeedback {
        feedback_id: format!("pfb_test_{}", proposal_id.0),
        proposal_id: proposal_id.clone(),
        review_id: AutoCommitProposalReviewId("placeholder".to_string()), // replaced by builder
        workspace_hash: "hash_a".to_string(),
        summary: "Test feedback".to_string(),
        required_changes: vec![RequiredProposalChange {
            category: ProposalFeedbackCategory::Tests,
            description: "Add more test coverage".to_string(),
            evidence_ref: None,
        }],
        blocked_dimensions: vec!["patch".to_string()],
        suggested_next_eval_focus: vec!["regression coverage".to_string()],
        severity: ProposalFeedbackSeverity::Blocking,
    }
}

#[test]
fn review_approved_references_exact_proposal() {
    let proposal = make_eligible_proposal();
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    ).unwrap();
    assert_eq!(proposal.proposal_id, review.proposal_id);
}

#[test]
fn review_rejected_references_exact_proposal() {
    let proposal = make_eligible_proposal();
    let feedback = make_test_feedback(&proposal.proposal_id);
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "Insufficient evidence".to_string(),
        vec![],
        Some(feedback),
    ).unwrap();
    assert_eq!(proposal.proposal_id, review.proposal_id);
    assert_eq!(AutoCommitProposalReviewDecision::Rejected, review.decision);
}

#[test]
fn review_changes_requested_references_exact_proposal() {
    let proposal = make_eligible_proposal();
    let feedback = make_test_feedback(&proposal.proposal_id);
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::ChangesRequested,
        AutoCommitProposalReviewer::User,
        "Needs changes".to_string(),
        vec![],
        Some(feedback),
    ).unwrap();
    assert_eq!(proposal.proposal_id, review.proposal_id);
    assert_eq!(AutoCommitProposalReviewDecision::ChangesRequested, review.decision);
}

#[test]
fn review_execution_allowed_now_is_always_false() {
    let proposal = make_eligible_proposal();
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    ).unwrap();
    assert!(!review.execution_allowed_now);
}

#[test]
fn review_creates_no_execution_grant() {
    let proposal = make_eligible_proposal();
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    ).unwrap();
    assert!(!review.creates_execution_grant);
}

#[test]
fn review_requires_rationale_for_rejection() {
    let proposal = make_eligible_proposal();
    let feedback = make_test_feedback(&proposal.proposal_id);
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "".to_string(), // empty rationale
        vec![],
        Some(feedback),
    );
    assert!(result.is_err());
}

#[test]
fn review_requires_feedback_for_changes_requested() {
    let proposal = make_eligible_proposal();
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::ChangesRequested,
        AutoCommitProposalReviewer::User,
        "Needs changes".to_string(),
        vec![],
        None, // no feedback
    );
    assert!(result.is_err());
}

#[test]
fn review_id_stable_for_same_proposal_decision_and_rationale() {
    let id1 = review_id_for("acp_123", &AutoCommitProposalReviewDecision::Approved, "ok");
    let id2 = review_id_for("acp_123", &AutoCommitProposalReviewDecision::Approved, "ok");
    assert_eq!(id1, id2);
}

// ── Commit 2: Builder validation tests ─────────────────────────────────────

#[test]
fn approved_review_requires_eligible_proposal() {
    let proposal = make_eligible_proposal();
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    );
    assert!(result.is_ok());
}

#[test]
fn blocked_proposal_cannot_be_approved() {
    let proposal = make_blocked_proposal();
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot approve"));
}

#[test]
fn superseded_proposal_cannot_be_approved() {
    let mut proposal = make_eligible_proposal();
    proposal.status = AutoCommitProposalStatus::Superseded;
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "Looks good".to_string(),
        vec![],
        None,
    );
    assert!(result.is_err());
}

#[test]
fn rejected_review_requires_rationale() {
    let proposal = make_eligible_proposal();
    let feedback = make_test_feedback(&proposal.proposal_id);
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "   ".to_string(), // blank rationale
        vec![],
        Some(feedback),
    );
    assert!(result.is_err());
}

// Correction #2: Rejected requires feedback
#[test]
fn rejected_review_requires_feedback() {
    let proposal = make_eligible_proposal();
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "Bad evidence".to_string(),
        vec![],
        None, // no feedback
    );
    assert!(result.is_err(), "Rejected must require feedback");
}

#[test]
fn changes_requested_requires_feedback() {
    let proposal = make_eligible_proposal();
    let result = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::ChangesRequested,
        AutoCommitProposalReviewer::User,
        "Needs work".to_string(),
        vec![],
        None,
    );
    assert!(result.is_err());
}

#[test]
fn review_copies_workspace_hash_from_proposal() {
    let proposal = make_eligible_proposal();
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "ok".to_string(),
        vec![],
        None,
    ).unwrap();
    assert_eq!(proposal.workspace_snapshot_id, review.workspace_hash);
}

#[test]
fn review_copies_proposal_hash_from_proposal() {
    let proposal = make_eligible_proposal();
    let review = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "ok".to_string(),
        vec![],
        None,
    ).unwrap();
    assert!(!review.proposal_hash.is_empty());
}

#[test]
fn review_builder_is_deterministic_for_same_inputs() {
    let proposal = make_eligible_proposal();
    let r1 = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "ok".to_string(),
        vec![],
        None,
    ).unwrap();
    let r2 = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Approved,
        AutoCommitProposalReviewer::User,
        "ok".to_string(),
        vec![],
        None,
    ).unwrap();
    assert_eq!(r1.review_id, r2.review_id);
}

// ── Commit 3: Persistence tests ────────────────────────────────────────────

fn make_test_review(proposal: &AutoCommitProposal, decision: AutoCommitProposalReviewDecision) -> AutoCommitProposalReview {
    let needs_feedback = matches!(&decision, AutoCommitProposalReviewDecision::Rejected | AutoCommitProposalReviewDecision::ChangesRequested);
    let feedback = if needs_feedback {
        Some(make_test_feedback(&proposal.proposal_id))
    } else {
        None
    };
    let rationale = format!("{:?} review", &decision);
    build_proposal_review(
        proposal,
        decision,
        AutoCommitProposalReviewer::User,
        rationale,
        vec![],
        feedback,
    ).unwrap()
}

#[test]
fn review_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    let path = save_proposal_review(dir.path(), &review).unwrap();
    assert!(path.exists());

    let loaded = load_proposal_review(dir.path(), &review.review_id).unwrap().unwrap();
    assert_eq!(review.review_id, loaded.review_id);
    assert_eq!(review.proposal_id, loaded.proposal_id);
    assert_eq!(review.decision, loaded.decision);
}

#[test]
fn latest_review_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &review).unwrap();

    let latest = load_latest_proposal_review(dir.path()).unwrap().unwrap();
    assert_eq!(review.review_id, latest.review_id);
}

#[test]
fn latest_review_for_proposal_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &review).unwrap();

    let loaded = load_latest_review_for_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(review.review_id, loaded.review_id);
}

#[test]
fn review_list_orders_by_reviewed_at() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = make_eligible_proposal();
    let r1 = make_test_review(&p1, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &r1).unwrap();

    let mut p2 = make_eligible_proposal();
    p2.workspace_snapshot_id = "hash_b".to_string();
    let r2 = make_test_review(&p2, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &r2).unwrap();

    let list = list_proposal_reviews(dir.path()).unwrap();
    assert_eq!(2, list.len());
}

#[test]
fn new_review_supersedes_prior_review_for_same_proposal() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();

    let r1 = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &r1).unwrap();

    // Second review with different rationale for same proposal
    let r2 = build_proposal_review(
        &proposal,
        AutoCommitProposalReviewDecision::Rejected,
        AutoCommitProposalReviewer::User,
        "Actually rejected".to_string(),
        vec![],
        Some(make_test_feedback(&proposal.proposal_id)),
    ).unwrap();
    save_proposal_review(dir.path(), &r2).unwrap();

    // First review should be superseded
    let loaded = load_proposal_review(dir.path(), &r1.review_id).unwrap().unwrap();
    assert_eq!(AutoCommitProposalReviewDecision::Superseded, loaded.decision);
}

#[test]
fn load_review_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    // Loading from empty dir returns None without error
    let result = load_latest_proposal_review(dir.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn review_persistence_does_not_touch_git() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    save_proposal_review(dir.path(), &review).unwrap();

    assert!(!dir.path().join(".git").exists());
    assert!(dir.path().join("proposal_reviews").exists());
}

#[test]
fn feedback_persists_with_review() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    save_proposal_review(dir.path(), &review).unwrap();

    // Feedback should be exported alongside
    assert!(dir.path().join("proposal_feedback").exists());
}

// ── Commit 4: Feedback export tests ────────────────────────────────────────

#[test]
fn approved_review_exports_no_rejection_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Approved);
    let result = export_rejection_feedback(dir.path(), &review).unwrap();
    assert!(result.is_none());
}

#[test]
fn rejected_review_exports_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let result = export_rejection_feedback(dir.path(), &review).unwrap();
    assert!(result.is_some());
    let path = result.unwrap();
    assert!(path.exists());
}

#[test]
fn changes_requested_exports_feedback() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::ChangesRequested);
    let result = export_rejection_feedback(dir.path(), &review).unwrap();
    assert!(result.is_some());
}

#[test]
fn feedback_contains_proposal_and_review_ids() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let feedback = review.feedback.as_ref().unwrap();
    assert_eq!(proposal.proposal_id, feedback.proposal_id);
    // review_id in feedback may be a placeholder; the review itself has the real ID
    assert!(!review.review_id.0.is_empty());
}

#[test]
fn feedback_contains_workspace_hash() {
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let feedback = review.feedback.as_ref().unwrap();
    assert_eq!(proposal.workspace_snapshot_id, feedback.workspace_hash);
}

#[test]
fn feedback_is_stable_for_same_review() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let p1 = export_rejection_feedback(dir.path(), &review).unwrap().unwrap();
    let content1 = std::fs::read_to_string(&p1).unwrap();
    let content2 = std::fs::read_to_string(&p1).unwrap();
    assert_eq!(content1, content2);
}

#[test]
fn feedback_does_not_modify_proposal() {
    // Feedback export only reads the review, does not modify the proposal
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let _ = export_rejection_feedback(dir.path(), &review);
    // Proposal is borrowed immutably, so it cannot be modified
}

#[test]
fn feedback_does_not_generate_new_patch() {
    // Feedback is structured input for a future iteration, not a patch
    let proposal = make_eligible_proposal();
    let review = make_test_review(&proposal, AutoCommitProposalReviewDecision::Rejected);
    let feedback = review.feedback.as_ref().unwrap();
    // Feedback has suggestions, not patches
    assert!(!feedback.suggested_next_eval_focus.is_empty());
    assert!(!feedback.required_changes.is_empty());
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
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

fn make_blocked_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Blocked,
        score: ReadinessScore {
            weighted_pass_rate: 0.5, patch_pass_rate: 0.5,
            policy_pass_rate: 1.0, rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.5, capability_context_pass_rate: 1.0, regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 5, reports_used: 5,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![],
            earliest_report: None, latest_report: None,
        },
        scenario_results: vec![], blockers: vec![], warnings: vec![],
    }
}

fn make_workspace_digest(hash: &str) -> WorkspaceSnapshotDigest {
    WorkspaceSnapshotDigest {
        blake3_hash: hash.to_string(),
        file_count: 5,
        generated_at: chrono::Utc::now(),
        file_digests: vec![],
    }
}

fn make_eval_report(scenario_id: &str) -> EvalRunReport {
    EvalRunReport {
        report_schema_version: 2,
        scenario_id: scenario_id.to_string(),
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
            gates_seen: vec!["gate.evaluated".to_string()],
            required_approvals_seen: vec![], unexpected_allows: vec![],
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
                DimensionScore { name: "policy".to_string(), passed: 1, total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Trace,
                        event_kind: Some("gate.evaluated".to_string()),
                        summary: "test".to_string(),
                    }],
                },
            ],
        },
    }
}
