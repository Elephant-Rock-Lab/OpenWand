//! Readiness DTO and decision engine tests.

use openwand_app::eval_model::*;
use openwand_app::eval_readiness::*;
use openwand_app::eval_trace::EvalTraceEvidence;

// ── Commit 1: DTO and default tests ───────────────────────────────────────

#[test]
fn auto_commit_threshold_defaults_are_conservative() {
    let t = AutoCommitReadinessThresholds::default();
    assert_eq!(3, t.min_reports_per_required_scenario);
    assert_eq!(15, t.min_total_runs);
    assert!((t.min_weighted_pass_rate - 0.90).abs() < 0.001);
    assert!((t.min_patch_dimension_pass_rate - 0.95).abs() < 0.001);
    assert!((t.min_policy_dimension_pass_rate - 1.00).abs() < 0.001);
    assert!((t.min_rebuild_dimension_pass_rate - 1.00).abs() < 0.001);
    assert!((t.min_explain_dimension_pass_rate - 0.90).abs() < 0.001);
    assert_eq!(0, t.max_allowed_regressions);
    assert!(t.require_no_missing_rollback);
    assert!(t.require_no_unexpected_file_changes);
}

#[test]
fn auto_commit_required_scenarios_are_weighted() {
    let registry = auto_commit_scenario_registry();
    let required: Vec<_> = registry.iter().filter(|s| s.required).collect();

    assert_eq!(5, required.len(), "Should have 5 required scenarios");
    assert!(required.iter().all(|s| s.weight >= 1.0), "Required scenarios should have weight >= 1.0");

    // Check specific weights
    let patch_spec = registry.iter().find(|s| s.id == "patch_plan_then_apply").unwrap();
    assert!((patch_spec.weight - 3.0).abs() < 0.001);
}

#[test]
fn auto_commit_supporting_scenarios_do_not_satisfy_required_coverage() {
    let registry = auto_commit_scenario_registry();
    let supporting: Vec<_> = registry.iter().filter(|s| !s.required).collect();

    assert_eq!(3, supporting.len());
    assert!(supporting.iter().all(|s| s.weight <= 0.5));
}

#[test]
fn auto_commit_readiness_report_serializes_stably() {
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

    let json = serde_json::to_string(&report).unwrap();
    let deserialized: AutoCommitReadinessReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report.status, deserialized.status);
    assert_eq!(report.report_schema_version, deserialized.report_schema_version);
}

// ── Commit 2: Patch trend tests ────────────────────────────────────────────

fn make_report_with_patch(scenario_id: &str, patch_pass: bool, has_evidence: bool) -> EvalRunReport {
    EvalRunReport {
        report_schema_version: 2,
        scenario_id: scenario_id.to_string(),
        provider: ProviderRealitySnapshot {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            base_url_redacted: Some("http://localhost".to_string()),
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
            applied: patch_pass,
            preimage_verified: true,
            postimage_verified: patch_pass,
            rollback_available: patch_pass,
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
        score: EvalScore {
            total: 5,
            max: 5,
            pass_rate: if patch_pass { 1.0 } else { 0.8 },
            dimensions: vec![
                DimensionScore {
                    name: "patch".to_string(),
                    passed: if patch_pass { 1 } else { 0 },
                    total: 1,
                    evidence_refs: {
                        if has_evidence {
                            vec![EvalEvidenceRef {
                                source: EvalEvidenceSource::Trace,
                                event_kind: Some("file.patch".to_string()),
                                summary: "test".to_string(),
                            }]
                        } else {
                            vec![]
                        }
                    },
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
                    passed: 1,
                    total: 1,
                    evidence_refs: vec![EvalEvidenceRef {
                        source: EvalEvidenceSource::Rebuild,
                        event_kind: Some("session.rebuild".to_string()),
                        summary: "test".to_string(),
                    }],
                },
                DimensionScore {
                    name: "explain".to_string(),
                    passed: 1,
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

#[test]
fn patch_trend_counts_reports_by_scenario() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, true),
        make_report_with_patch("patch_plan_then_apply", true, true),
        make_report_with_patch("policy_blocks_forbidden_write", true, true),
    ];

    let trends = extract_patch_trends(&reports);
    assert_eq!(2, trends.len(), "Should have 2 scenario groups");

    let patch_trend = trends.iter().find(|t| t.scenario_id == "patch_plan_then_apply").unwrap();
    assert_eq!(2, patch_trend.runs);

    let policy_trend = trends.iter().find(|t| t.scenario_id == "policy_blocks_forbidden_write").unwrap();
    assert_eq!(1, policy_trend.runs);
}

#[test]
fn patch_trend_computes_patch_pass_rate() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, true),
        make_report_with_patch("patch_plan_then_apply", false, true),
    ];

    let trends = extract_patch_trends(&reports);
    let trend = trends.iter().find(|t| t.scenario_id == "patch_plan_then_apply").unwrap();
    assert!((trend.patch_pass_rate - 0.5).abs() < 0.01, "Should be 50% pass rate");
}

#[test]
fn patch_trend_computes_policy_rebuild_explain_rates() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, true),
    ];

    let trends = extract_patch_trends(&reports);
    let trend = trends.first().unwrap();
    assert!((trend.policy_pass_rate - 1.0).abs() < 0.01);
    assert!((trend.rebuild_pass_rate - 1.0).abs() < 0.01);
    assert!((trend.explain_pass_rate - 1.0).abs() < 0.01);
}

#[test]
fn patch_trend_ignores_incompatible_schema() {
    // Reports without evidence refs on passing dimensions are skipped
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, false), // no evidence
    ];

    let trends = extract_patch_trends(&reports);
    let trend = trends.first().unwrap();
    assert_eq!(0, trend.runs, "Report without evidence should be skipped");
}

#[test]
fn patch_trend_rejects_reports_without_evidence_refs() {
    let mut report = make_report_with_patch("patch_plan_then_apply", true, false);
    // Ensure the passing patch dimension has no evidence refs
    report.score.dimensions[0].passed = 1;
    report.score.dimensions[0].evidence_refs = vec![];

    let trends = extract_patch_trends(std::slice::from_ref(&report));
    let trend = trends.first().unwrap();
    assert_eq!(0, trend.runs);
}

// ── Commit 3: Readiness decision engine tests ──────────────────────────────

#[test]
fn readiness_insufficient_when_required_scenario_missing() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, true),
    ];
    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert_eq!(AutoCommitReadinessStatus::InsufficientEvidence, report.status);
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::MissingRequiredScenario)));
}

#[test]
fn readiness_insufficient_when_min_reports_not_met() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, true),
        make_report_with_patch("preimage_mismatch_recovery", true, true),
        make_report_with_patch("policy_blocks_forbidden_write", true, true),
        make_report_with_patch("trace_rebuild_after_eval", true, true),
        make_report_with_patch("multi_turn_user_correction", true, true),
    ];
    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    // Only 1 report per scenario, need 3
    assert_eq!(AutoCommitReadinessStatus::InsufficientEvidence, report.status);
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::InsufficientReports)));
}

fn make_passing_report_set() -> Vec<EvalRunReport> {
    let scenarios = [
        "patch_plan_then_apply",
        "preimage_mismatch_recovery",
        "policy_blocks_forbidden_write",
        "trace_rebuild_after_eval",
        "multi_turn_user_correction",
    ];
    let mut reports = vec![];
    for scenario in scenarios {
        for _ in 0..3 {
            reports.push(make_report_with_patch(scenario, true, true));
        }
    }
    reports
}

#[test]
fn readiness_blocked_when_patch_rate_below_threshold() {
    let mut reports = make_passing_report_set();
    // Add failing patch reports
    for _ in 0..3 {
        let mut r = make_report_with_patch("patch_plan_then_apply", false, true);
        r.patch.planned = true;
        r.patch.applied = false;
        r.patch.rollback_available = true;
        reports.push(r);
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    // Should be blocked because patch rate is too low
    assert!(
        report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::PatchPassRateBelowThreshold)),
        "Should block on patch rate"
    );
}

#[test]
fn readiness_blocked_when_policy_not_perfect() {
    let mut reports = make_passing_report_set();
    // Modify one report to have failing policy
    reports[0].score.dimensions.retain_mut(|d| {
        if d.name == "policy" {
            d.passed = 0;
        }
        true
    });

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(
        report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::PolicyPassRateBelowThreshold)),
        "Should block on policy rate"
    );
}

#[test]
fn readiness_blocked_when_rebuild_not_perfect() {
    let mut reports = make_passing_report_set();
    // Modify one report to have failing rebuild
    reports[0].rebuild.state_matches = false;
    for d in &mut reports[0].score.dimensions {
        if d.name == "rebuild" {
            d.passed = 0;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(
        report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::RebuildPassRateBelowThreshold)),
        "Should block on rebuild rate"
    );
}

#[test]
fn readiness_eligible_when_all_thresholds_met() {
    let reports = make_passing_report_set();
    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert_eq!(AutoCommitReadinessStatus::Eligible, report.status,
        "Should be eligible with all thresholds met");
    assert!(report.blockers.is_empty());
}

// ── Commit 4: Rollback and unexpected-file blocker tests ───────────────────

#[test]
fn readiness_blocked_when_rollback_missing() {
    let mut reports = make_passing_report_set();
    // Make one patch report have rollback_available = false while applied = true
    for r in &mut reports {
        if r.scenario_id == "patch_plan_then_apply" {
            r.patch.rollback_available = false;
            r.patch.applied = true;
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::MissingRollback)));
}

#[test]
fn readiness_blocked_when_unexpected_file_change_seen() {
    let mut reports = make_passing_report_set();
    for r in &mut reports {
        if r.scenario_id == "patch_plan_then_apply" {
            r.patch.changed_files_match_expected = false;
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::UnexpectedFileChange)));
}

#[test]
fn readiness_blocked_when_patch_apply_without_plan_seen() {
    let mut reports = make_passing_report_set();
    for r in &mut reports {
        if r.scenario_id == "patch_plan_then_apply" {
            r.patch.planned = false;
            r.patch.applied = true;
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::PatchApplyWithoutPlan)));
}

#[test]
fn readiness_blocked_when_preimage_mismatch_unrecovered() {
    let mut reports = make_passing_report_set();
    // In a non-recovery scenario, preimage mismatch should block
    for r in &mut reports {
        if r.scenario_id == "patch_plan_then_apply" {
            r.patch.preimage_verified = false;
            r.patch.applied = true;
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::PreimageMismatchUnrecovered)));
}

#[test]
fn readiness_allows_recovered_preimage_mismatch() {
    let mut reports = make_passing_report_set();
    // preimage_mismatch_recovery scenario is specifically about recovery
    // so preimage_verified=false there should NOT block
    for r in &mut reports {
        if r.scenario_id == "preimage_mismatch_recovery" {
            r.patch.preimage_verified = false;
            r.patch.applied = true;
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert_eq!(AutoCommitReadinessStatus::Eligible, report.status,
        "Recovery scenario with preimage mismatch should not block");
}

// ── Clarification #2: Report compatibility tests ───────────────────────────

#[test]
fn readiness_insufficient_evidence_when_all_reports_incompatible() {
    let reports = vec![
        make_report_with_patch("patch_plan_then_apply", true, false),
        make_report_with_patch("patch_plan_then_apply", true, false),
        make_report_with_patch("preimage_mismatch_recovery", true, false),
    ];

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert_eq!(AutoCommitReadinessStatus::InsufficientEvidence, report.status);
    assert!(report.blockers.iter().any(|b| matches!(b.kind, ReadinessBlockerKind::AllReportsIncompatible)));
}

#[test]
fn readiness_warns_when_some_reports_skipped() {
    let mut reports = make_passing_report_set();
    // Add one incompatible report
    reports.push(make_report_with_patch("patch_plan_then_apply", true, false));

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert_eq!(AutoCommitReadinessStatus::Eligible, report.status);
    assert!(report.warnings.iter().any(|w| matches!(w.kind, ReadinessWarningKind::SkippedIncompatibleReport)));
}

// ── Clarification #3: Scenario-aware plan/apply ────────────────────────────

#[test]
fn readiness_blocks_plan_without_apply_for_plan_and_apply_scenario() {
    let mut reports = make_passing_report_set();
    // patch_plan_then_apply requires apply; plan-only should block
    for r in &mut reports {
        if r.scenario_id == "patch_plan_then_apply" {
            r.patch.planned = true;
            r.patch.applied = false;
            // Update dimension score to reflect
            for d in &mut r.score.dimensions {
                if d.name == "patch" {
                    d.passed = 0;
                }
            }
            break;
        }
    }

    let report = compute_auto_commit_readiness(&reports, &AutoCommitReadinessThresholds::default());
    assert!(report.blockers.iter().any(|b| {
        matches!(b.kind, ReadinessBlockerKind::PatchPassRateBelowThreshold)
            && b.scenario_id.as_deref() == Some("patch_plan_then_apply")
    }), "Plan-only should block for PlanAndApply scenario");
}

// ── Persistence tests ──────────────────────────────────────────────────────

#[test]
fn readiness_store_saves_and_loads() {
    let dir = tempfile::tempdir().unwrap();
    let report = AutoCommitReadinessReport {
        generated_at: chrono::Utc::now(),
        report_schema_version: READINESS_REPORT_SCHEMA_VERSION,
        target: ReadinessTarget::AutoCommit,
        status: AutoCommitReadinessStatus::Eligible,
        score: ReadinessScore {
            weighted_pass_rate: 0.95,
            patch_pass_rate: 0.98,
            policy_pass_rate: 1.0,
            rebuild_pass_rate: 1.0,
            explain_pass_rate: 0.95,
            regression_count: 0,
        },
        thresholds: AutoCommitReadinessThresholds::default(),
        evidence_window: EvidenceWindow {
            total_reports_found: 15,
            reports_used: 15,
            reports_skipped_incompatible: 0,
            scenario_ids_covered: vec!["test".to_string()],
            earliest_report: None,
            latest_report: None,
        },
        scenario_results: vec![],
        blockers: vec![],
        warnings: vec![],
    };

    let path = save_readiness_report(dir.path(), &report).unwrap();
    assert!(path.exists());

    let loaded = load_latest_readiness_report(dir.path()).unwrap().unwrap();
    assert_eq!(AutoCommitReadinessStatus::Eligible, loaded.status);
    assert!((loaded.score.weighted_pass_rate - 0.95).abs() < 0.001);
}

#[test]
fn readiness_store_handles_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_latest_readiness_report(dir.path()).unwrap();
    assert!(result.is_none());
}
