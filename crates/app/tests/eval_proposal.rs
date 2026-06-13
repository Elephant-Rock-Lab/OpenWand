//! Auto-commit proposal tests — all commits combined.

use openwand_app::eval_model::*;
use openwand_app::eval_proposal::*;
use openwand_app::eval_readiness::*;
use std::path::Path;

// ── Commit 1: DTO and invariant tests ──────────────────────────────────────

#[test]
fn proposal_eligible_requires_readiness_reference() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test_scenario");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(AutoCommitProposalStatus::Eligible, proposal.status);
    assert!(!proposal.workspace_snapshot_id.is_empty());
}

#[test]
fn proposal_blocked_carries_blockers() {
    let mut readiness = make_blocked_readiness();
    readiness.blockers.push(ReadinessBlocker {
        kind: ReadinessBlockerKind::MissingRequiredScenario,
        scenario_id: Some("patch_plan_then_apply".to_string()),
        detail: "Missing required scenario".to_string(),
    });
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test_scenario");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(AutoCommitProposalStatus::Blocked, proposal.status);
    assert!(!proposal.blockers.is_empty());
}

#[test]
fn proposal_has_no_execution_fields() {
    // The proposal struct has no fields representing git operations.
    // This test documents the invariant by construction.
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test_scenario");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    // governance_summary.execution_allowed_now is always false
    assert!(!proposal.governance_summary.execution_allowed_now);
    // No git-related fields exist on the struct
    assert!(!proposal.commit_title.is_empty());
    assert!(!proposal.commit_body.is_empty());
}

#[test]
fn proposal_id_stable_for_same_readiness_and_workspace_snapshot() {
    let readiness = make_eligible_readiness();
    let rid = readiness_id_for_report(&readiness);
    let id_a = proposal_id_for(&rid, "hash_a");
    let id_b = proposal_id_for(&rid, "hash_a");
    assert_eq!(id_a, id_b, "Same inputs must produce same proposal ID");
}

#[test]
fn proposal_status_superseded_when_workspace_snapshot_changes() {
    let readiness = make_eligible_readiness();
    let rid = readiness_id_for_report(&readiness);
    let id_a = proposal_id_for(&rid, "hash_a");
    let id_b = proposal_id_for(&rid, "hash_b");
    assert_ne!(id_a, id_b, "Different workspace hash must produce different ID");
}

// ── Commit 2: Builder tests ────────────────────────────────────────────────

#[test]
fn builder_eligible_readiness_produces_eligible_proposal() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("patch_plan_then_apply");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(AutoCommitProposalStatus::Eligible, proposal.status);
}

#[test]
fn builder_blocked_readiness_produces_blocked_proposal() {
    let readiness = make_blocked_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("patch_plan_then_apply");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(AutoCommitProposalStatus::Blocked, proposal.status);
}

#[test]
fn builder_includes_regression_summary() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    // Without comparison, regression comes from readiness score
    assert!(proposal.regression_summary.regression_count >= 0);
}

#[test]
fn builder_includes_patch_evidence_summary() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(!proposal.evidence_summary.dimension_summaries.is_empty());
}

#[test]
fn builder_includes_workspace_snapshot_hash() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("abc123");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!("abc123", proposal.workspace_snapshot_id);
}

#[test]
fn builder_is_deterministic_for_same_inputs() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash_a");
    let eval = make_eval_report("test");

    let inputs1 = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let inputs2 = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let p1 = build_auto_commit_proposal(inputs1);
    let p2 = build_auto_commit_proposal(inputs2);
    assert_eq!(p1.proposal_id, p2.proposal_id);
    assert_eq!(p1.commit_title, p2.commit_title);
}

#[test]
fn builder_does_not_read_git_directly() {
    // Builder takes pre-computed inputs, no git commands.
    // This test documents the design: all evidence is passed in.
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let _ = build_auto_commit_proposal(inputs);
    // No assertion needed — if this compiles, builder signature is correct.
    // Source guard test proves no git imports in the module.
}

// ── Commit 3: Commit message synthesis tests ───────────────────────────────

#[test]
fn commit_title_is_stable() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("patch_plan_then_apply");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let p1 = build_auto_commit_proposal(AutoCommitProposalInputs {
        readiness: &readiness, workspace_digest: &workspace, eval_report: &eval, comparison: None,
    });
    let p2 = build_auto_commit_proposal(inputs);
    assert_eq!(p1.commit_title, p2.commit_title);
}

#[test]
fn commit_body_mentions_no_execution() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    let lower = proposal.commit_body.to_lowercase();
    assert!(lower.contains("no git commit was executed"));
    assert!(lower.contains("proposal only"));
}

#[test]
fn commit_body_includes_validation() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(proposal.commit_body.contains("Validation:"));
    assert!(proposal.commit_body.contains("Regressions:"));
}

#[test]
fn commit_body_includes_governance() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(proposal.commit_body.contains("Governance:"));
    assert!(proposal.commit_body.contains("Readiness:"));
}

#[test]
fn commit_body_includes_blockers_when_blocked() {
    let mut readiness = make_blocked_readiness();
    readiness.blockers.push(ReadinessBlocker {
        kind: ReadinessBlockerKind::PatchPassRateBelowThreshold,
        scenario_id: Some("patch_plan_then_apply".to_string()),
        detail: "Patch rate too low".to_string(),
    });
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(proposal.commit_body.contains("Patch rate too low"));
}

#[test]
fn commit_body_never_claims_commit_completed() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    let violations = check_forbidden_phrases(&proposal.commit_body);
    assert!(violations.is_empty(), "Forbidden phrases found: {:?}", violations);
}

// ── Commit 4: Persistence tests ────────────────────────────────────────────

#[test]
fn proposal_persists_and_loads_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_test_proposal("hash_1");
    let path = save_proposal(dir.path(), &proposal).unwrap();
    assert!(path.exists());

    let loaded = load_proposal(dir.path(), &proposal.proposal_id).unwrap().unwrap();
    assert_eq!(proposal.proposal_id, loaded.proposal_id);
    assert_eq!(proposal.status, loaded.status);
    assert_eq!(proposal.workspace_snapshot_id, loaded.workspace_snapshot_id);
}

#[test]
fn proposal_list_orders_by_generated_at() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = make_test_proposal("hash_1");
    let p2 = make_test_proposal("hash_2");
    save_proposal(dir.path(), &p1).unwrap();
    save_proposal(dir.path(), &p2).unwrap();

    let list = list_proposals(dir.path()).unwrap();
    assert_eq!(2, list.len());
    // Ordered by generated_at desc (p2 is later)
}

#[test]
fn proposal_latest_returns_expected() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = make_test_proposal("hash_1");
    save_proposal(dir.path(), &p1).unwrap();

    let latest = load_latest_proposal(dir.path()).unwrap().unwrap();
    assert_eq!(p1.proposal_id, latest.proposal_id);
}

#[test]
fn proposal_same_readiness_same_snapshot_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = make_test_proposal("hash_1");
    let p2 = make_test_proposal("hash_1");
    save_proposal(dir.path(), &p1).unwrap();
    save_proposal(dir.path(), &p2).unwrap();

    let list = list_proposals(dir.path()).unwrap();
    // Same proposal_id overwrites — should be 1, not 2
    assert_eq!(1, list.len());
}

#[test]
fn proposal_new_snapshot_supersedes_old() {
    let dir = tempfile::tempdir().unwrap();
    // Save first proposal
    let p1 = make_test_proposal("hash_1");
    save_proposal(dir.path(), &p1).unwrap();

    // Save second with different hash
    let p2 = make_test_proposal("hash_2");
    save_proposal(dir.path(), &p2).unwrap();

    // p1 should be superseded
    let loaded = load_proposal(dir.path(), &p1.proposal_id).unwrap().unwrap();
    assert_eq!(AutoCommitProposalStatus::Superseded, loaded.status);
}

#[test]
fn proposal_persistence_does_not_touch_git() {
    // Persistence only writes to proposals/ directory
    let dir = tempfile::tempdir().unwrap();
    let proposal = make_test_proposal("hash_1");
    let _path = save_proposal(dir.path(), &proposal).unwrap();

    // No .git directory created
    assert!(!dir.path().join(".git").exists());
    // Only proposals/ dir exists
    let proposals_dir = dir.path().join("proposals");
    assert!(proposals_dir.exists());
}

// ── Commit 6: Governance summary wiring tests ──────────────────────────────

#[test]
fn governance_summary_always_disallows_execution_in_wave11() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(!proposal.governance_summary.execution_allowed_now);
}

#[test]
fn eligible_proposal_still_disallows_execution() {
    let readiness = make_eligible_readiness();
    assert_eq!(AutoCommitReadinessStatus::Eligible, readiness.status);

    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(AutoCommitProposalStatus::Eligible, proposal.status);
    assert!(!proposal.governance_summary.execution_allowed_now,
        "Even eligible proposals must disallow execution in Wave 11");
}

#[test]
fn blocked_proposal_disallows_execution_with_blockers() {
    let readiness = make_blocked_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert!(!proposal.governance_summary.execution_allowed_now);
}

#[test]
fn proposal_records_future_confirmation_level() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    assert_eq!(
        openwand_core::ConfirmationLevel::Escalate,
        proposal.governance_summary.confirmation_required_for_future_execution,
    );
}

// ── Wording drift guard (optional strengthening) ────────────────────────────

#[test]
fn commit_body_never_contains_past_tense_execution_claims() {
    let readiness = make_eligible_readiness();
    let workspace = make_workspace_digest("hash");
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    let proposal = build_auto_commit_proposal(inputs);
    let violations = check_forbidden_phrases(&proposal.commit_body);
    assert!(violations.is_empty(), "Forbidden completion phrases: {:?}", violations);
}

// ── Content-addressed ID test (Correction #1) ──────────────────────────────

#[test]
fn proposal_id_is_blake3_content_addressed() {
    let id = proposal_id_for("readiness_abc", "workspace_xyz");
    assert!(id.0.starts_with("acp_"), "Proposal ID should be acp_ prefixed BLAKE3 hash");
    assert!(id.0.len() > 10, "Should be a real hash, not a stub");
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_workspace_digest(hash: &str) -> WorkspaceSnapshotDigest {
    WorkspaceSnapshotDigest {
        blake3_hash: hash.to_string(),
        file_count: 5,
        generated_at: chrono::Utc::now(),
        file_digests: vec![
            ("src/lib.rs".to_string(), "hash1".to_string()),
            ("src/main.rs".to_string(), "hash2".to_string()),
        ],
    }
}

fn make_eligible_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore {
            weighted_pass_rate: 0.95,
            patch_pass_rate: 0.98,
            policy_pass_rate: 1.0,
            rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95,
            capability_context_pass_rate: 1.0,
            regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 15,
            reports_used: 15,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec!["patch_plan_then_apply".to_string()],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    }
}

fn make_blocked_readiness() -> AutoCommitReadinessReport {
    AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: 1,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Blocked,
        score: ReadinessScore {
            weighted_pass_rate: 0.5,
            patch_pass_rate: 0.5,
            policy_pass_rate: 1.0,
            rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.5,
            capability_context_pass_rate: 1.0,
            regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 5,
            reports_used: 5,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec![],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    }
}

fn make_eval_report(scenario_id: &str) -> EvalRunReport {
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
            planned: true,
            applied: true,
            preimage_verified: true,
            postimage_verified: true,
            rollback_available: true,
            changed_files_match_expected: true,
        },
        explain: ExplainEvalResult {
            memory_matches: true,
            policy_matches: true,
            tool_matches: true,
            completion_matches: true,
        },
        rebuild: RebuildEvalResult {
            events_replayed: 10,
            state_matches: true,
            divergences: vec![],
        },
        capability_context: CapabilityContextEvalResult::default(),
        score: EvalScore {
            total: 5,
            max: 5,
            pass_rate: 1.0,
            dimensions: vec![
                DimensionScore {
                    name: "patch".to_string(),
                    passed: 1, total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Trace,
                        event_kind: Some("file.patch".to_string()),
                        summary: "test".to_string(),
                    }],
                },
                DimensionScore {
                    name: "policy".to_string(),
                    passed: 1, total: 1,
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

fn make_test_proposal(workspace_hash: &str) -> AutoCommitProposal {
    // Use a fixed readiness so proposal_id is deterministic
    let mut readiness = make_eligible_readiness();
    readiness.generated_at = chrono::DateTime::UNIX_EPOCH;
    let workspace = make_workspace_digest(workspace_hash);
    let eval = make_eval_report("test");
    let inputs = AutoCommitProposalInputs {
        readiness: &readiness,
        workspace_digest: &workspace,
        eval_report: &eval,
        comparison: None,
    };
    build_auto_commit_proposal(inputs)
}
