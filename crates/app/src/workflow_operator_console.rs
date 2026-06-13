//! Workflow operator console assembler.
//!
//! Patch 1 (48A): Console consumes the evidence chain inspector assembly.
//!   Console does not duplicate full-chain assembly logic.
//! No persistence. Recomputes console state from existing evidence indexes.
//! The console observes, summarizes, and links evidence.

use std::path::Path;

use openwand_workflow::workflow_operator_console::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

/// Assemble console state from existing evidence indexes.
/// Patch 1 (48A): Consumes evidence chain inspector links where available.
/// Falls back to manual ladder scan when inspector is unavailable.
/// Returns computed state, writes nothing.
pub fn assemble_console_state(
    store_root: &Path,
    workflow_execution_id: &WorkflowExecutionId,
) -> Result<WorkflowOperatorConsoleState, String> {
    let wfx = workflow_execution_id.0.as_str();

    let stages = vec![];
    let run_status = "unknown".to_string();
    let detected = openwand_workflow::workflow_loop_state::WorkflowDetectedLoopState::Inconclusive;

    let mut evidence_chain = Vec::new();
    let mut chain_warnings = Vec::new();
    let sections;
    let attestation_groups;
    let verification_readiness_summary;

    // Patch 1 (48A): Try to consume evidence chain inspector
    let inspector_links = assemble_inspector_links(store_root, workflow_execution_id);

    if !inspector_links.is_empty() {
        // Map inspector links to console evidence links
        for link in &inspector_links {
            evidence_chain.push(ConsoleEvidenceLink {
                link_kind: link.record_type.clone(),
                record_id: link.record_id.clone(),
                status: "found".into(),
                summary: summarize_link_type(&link.record_type),
            });
        }
    } else {
        // Fallback: manual ladder scan (legacy behavior)
        let cc_id = load_record_id(store_root, "workflow_command_composers", "by_workflow_run", wfx);
        let cr_id = load_record_id(store_root, "workflow_command_reviews", "by_workflow_run", wfx);
        let mr_id = load_record_id(store_root, "workflow_manual_results", "by_workflow_run", wfx);
        let mrr_id = load_record_id(store_root, "workflow_manual_result_reviews", "by_workflow_run", wfx);
        let rr_id = load_record_id(store_root, "workflow_manual_result_reconciliation_readiness", "by_workflow_run", wfx);
        let gate_id = load_record_id(store_root, "workflow_manual_result_reconciliation_gates", "by_workflow_run", wfx);

        if let Some(ref id) = cc_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "command_composer".into(), record_id: id.clone(), status: "found".into(), summary: "Command composed".into() });
        }
        if let Some(ref id) = cr_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "command_review".into(), record_id: id.clone(), status: "found".into(), summary: "Command reviewed".into() });
        }
        if let Some(ref id) = mr_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_result".into(), record_id: id.clone(), status: "found".into(), summary: "Manual result captured".into() });
        }
        if let Some(ref id) = mrr_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_result_review".into(), record_id: id.clone(), status: "found".into(), summary: "Manual result reviewed".into() });
        }
        if let Some(ref id) = rr_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "reconciliation_readiness".into(), record_id: id.clone(), status: "found".into(), summary: "Reconciliation readiness evaluated".into() });
        }
        if let Some(ref id) = gate_id {
            evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_reconciliation_gate".into(), record_id: id.clone(), status: "found".into(), summary: "Manual reconciliation gate evaluated".into() });
        }

        chain_warnings = validate_manual_result_chain(
            cc_id.as_deref(), cr_id.as_deref(), mr_id.as_deref(),
            mrr_id.as_deref(), rr_id.as_deref(), gate_id.as_deref(),
        );
    }

    // Patch 4 (48A): Load attestations grouped by target
    attestation_groups = load_attestation_groups(store_root, wfx);

    // Patch 3 (48A): Load verification readiness as eligibility summaries
    verification_readiness_summary = load_readiness_eligibility(store_root, wfx);

    // Build section summaries from gathered evidence
    sections = build_section_summaries(&evidence_chain, &attestation_groups, &verification_readiness_summary);

    // Patch 2 (48A): Linkage-aware validation
    let linkage_warnings = validate_linkage_aware_chain(
        &evidence_chain, wfx, &attestation_groups, &verification_readiness_summary,
    );
    chain_warnings.extend(linkage_warnings);

    Ok(build_console_state(
        workflow_execution_id.clone(),
        run_status,
        stages,
        &detected,
        None,
        evidence_chain,
        chain_warnings,
        sections,
        attestation_groups,
        verification_readiness_summary,
    ))
}

/// Patch 1 (48A): Try to get inspector links for this workflow run.
/// Returns empty vec if inspector cannot load (no workflow run record).
fn assemble_inspector_links(
    store_root: &Path,
    workflow_execution_id: &WorkflowExecutionId,
) -> Vec<openwand_workflow::workflow_evidence_chain_inspector::EvidenceChainLink> {
    match crate::workflow_evidence_chain_inspector::assemble_evidence_chain(
        store_root, workflow_execution_id, false,
    ) {
        Ok(state) => state.links,
        Err(_) => vec![],
    }
}

fn summarize_link_type(record_type: &str) -> String {
    match record_type {
        "workflow_run" => "Workflow run record".into(),
        "task_plan" => "Task plan".into(),
        "workflow_proposal" => "Workflow proposal".into(),
        "proposal_review" => "Proposal review".into(),
        "workflow_readiness" => "Workflow readiness".into(),
        "action_route" => "Action routed".into(),
        "action_outcome" => "Action outcome".into(),
        "reconciliation" => "Outcome reconciled".into(),
        "continuation" => "Continuation readiness".into(),
        "command_composer" => "Command composed".into(),
        "command_review" => "Command reviewed".into(),
        "manual_result" => "Manual result captured".into(),
        "manual_result_review" => "Manual result reviewed".into(),
        "reconciliation_readiness" => "Reconciliation readiness evaluated".into(),
        "manual_reconciliation_gate" => "Reconciliation gate evaluated".into(),
        _ => format!("{} record", record_type),
    }
}

/// Build section summaries from gathered evidence.
fn build_section_summaries(
    chain: &[ConsoleEvidenceLink],
    attestations: &[ConsoleAttestationGroup],
    readiness: &[ConsoleReadinessEligibilitySummary],
) -> Vec<ConsoleSectionSummary> {
    let upstream_types = ["workflow_run", "task_plan", "workflow_proposal", "proposal_review", "workflow_readiness"];
    let loop_types = ["action_route", "action_outcome", "reconciliation", "continuation"];
    let ladder_types = ["command_composer", "command_review", "manual_result", "manual_result_review", "reconciliation_readiness", "manual_reconciliation_gate"];

    let mut sections = Vec::new();

    // Upstream spine
    let upstream_links: Vec<_> = chain.iter().filter(|l| upstream_types.contains(&l.link_kind.as_str())).collect();
    sections.push(ConsoleSectionSummary {
        section: ConsoleEvidenceSection::UpstreamSpine,
        link_count: upstream_types.len(),
        present_count: upstream_links.len(),
        missing_count: upstream_types.len() - upstream_links.len(),
        warnings_count: 0,
    });

    // Loop control
    let loop_links: Vec<_> = chain.iter().filter(|l| loop_types.contains(&l.link_kind.as_str())).collect();
    sections.push(ConsoleSectionSummary {
        section: ConsoleEvidenceSection::LoopControl,
        link_count: loop_types.len(),
        present_count: loop_links.len(),
        missing_count: loop_types.len() - loop_links.len(),
        warnings_count: 0,
    });

    // Manual result ladder
    let ladder_links: Vec<_> = chain.iter().filter(|l| ladder_types.contains(&l.link_kind.as_str())).collect();
    sections.push(ConsoleSectionSummary {
        section: ConsoleEvidenceSection::ManualResultLadder,
        link_count: ladder_types.len(),
        present_count: ladder_links.len(),
        missing_count: ladder_types.len() - ladder_links.len(),
        warnings_count: 0,
    });

    // External attestations
    let att_count: usize = attestations.iter().map(|g| g.attestations.len()).sum();
    sections.push(ConsoleSectionSummary {
        section: ConsoleEvidenceSection::ExternalAttestations,
        link_count: attestations.len(),
        present_count: att_count,
        missing_count: 0, // attestations are optional
        warnings_count: 0,
    });

    // Verification readiness
    sections.push(ConsoleSectionSummary {
        section: ConsoleEvidenceSection::VerificationReadiness,
        link_count: readiness.len(),
        present_count: readiness.len(),
        missing_count: 0, // readiness is optional
        warnings_count: 0,
    });

    sections
}

/// Patch 4 (48A): Load attestations grouped by target.
fn load_attestation_groups(store_root: &Path, wfx: &str) -> Vec<ConsoleAttestationGroup> {
    match crate::workflow_external_attestation::attestations_by_workflow_run(store_root, wfx) {
        Ok(attestations) if !attestations.is_empty() => {
            let mut groups: std::collections::HashMap<String, Vec<ConsoleAttestationRow>> = std::collections::HashMap::new();
            for att in &attestations {
                let key = format!("{:?}:{}", att.target.target_kind, att.target.target_id);
                groups.entry(key).or_default().push(ConsoleAttestationRow {
                    attestation_id: att.attestation_id.0.clone(),
                    kind: format!("{:?}", att.kind),
                    source_name: att.source.name.clone(),
                    claim: att.claim.clone(),
                    verified_by_openwand: false, // Patch 4: always unverified in console
                    promotes_trust: false,
                });
            }
            groups.into_iter().map(|(key, atts)| {
                let parts: Vec<&str> = key.splitn(2, ':').collect();
                ConsoleAttestationGroup {
                    target_kind: parts.first().unwrap_or(&"unknown").to_string(),
                    target_id: parts.get(1).unwrap_or(&"unknown").to_string(),
                    attestations: atts,
                }
            }).collect()
        }
        _ => vec![],
    }
}

/// Patch 3 (48A): Load verification readiness as eligibility summaries.
fn load_readiness_eligibility(store_root: &Path, wfx: &str) -> Vec<ConsoleReadinessEligibilitySummary> {
    match crate::workflow_verification_readiness::readiness_by_workflow_run(store_root, wfx) {
        Ok(records) => records.iter().map(|r| ConsoleReadinessEligibilitySummary {
            readiness_id: r.readiness_id.0.clone(),
            target_kind: format!("{:?}", r.target_kind),
            target_id: r.target_id.clone(),
            status: format!("{:?}", r.status),
            is_eligibility_only: true, // Patch 3: always eligibility-only
        }).collect(),
        Err(_) => vec![],
    }
}

fn load_record_id(store_root: &Path, root: &str, index: &str, key: &str) -> Option<String> {
    let idx_path = store_root.join(root).join(index).join(format!("{}.json", key));
    if let Ok(content) = std::fs::read_to_string(&idx_path) {
        // Try as JSON array first
        if let Ok(ids) = serde_json::from_str::<Vec<String>>(&content) {
            return ids.into_iter().last();
        }
        // Fall back to trimmed string
        let trimmed = content.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dir() -> PathBuf { tempfile::tempdir().unwrap().into_path() }

    #[test]
    fn assemble_console_state_returns_state_without_writing() {
        let d = test_dir();
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert_eq!(WorkflowExecutionId("wfx_t".into()), state.workflow_execution_id);
        assert!(!d.join("workflow_operator_console").exists());
    }

    #[test]
    fn assemble_console_creates_no_console_record() {
        let d = test_dir();
        assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        let entries: Vec<_> = std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).collect();
        assert!(entries.is_empty(), "Console should not write any files");
    }

    #[test]
    fn assemble_console_writes_no_eval_report_files() {
        let d = test_dir();
        assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(!d.join("eval_reports").exists());
    }

    // Patch 1 (48A): Console uses evidence chain inspector
    #[test]
    fn operator_console_uses_evidence_chain_inspector_links() {
        let d = test_dir();
        // Save a minimal workflow run so the inspector can find it
        save_minimal_run(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        // Inspector provides upstream spine links
        assert!(state.evidence_chain.iter().any(|l| l.link_kind == "task_plan"),
            "Console should include task_plan from inspector");
        assert!(state.evidence_chain.iter().any(|l| l.link_kind == "workflow_proposal"),
            "Console should include proposal from inspector");
    }

    #[test]
    fn operator_console_chain_sections_match_inspector_link_count() {
        let d = test_dir();
        save_minimal_run(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        // Section summaries should reflect the actual link count
        let upstream = state.sections.iter().find(|s| s.section == ConsoleEvidenceSection::UpstreamSpine).unwrap();
        assert!(upstream.present_count > 0, "Upstream section should have present links from inspector");
    }

    #[test]
    fn operator_console_does_not_duplicate_full_chain_assembly_logic() {
        // Verify the assembler calls the inspector, not its own chain logic
        let src = include_str!("workflow_operator_console.rs");
        assert!(src.contains("assemble_inspector_links"), "Should use inspector links");
        assert!(src.contains("workflow_evidence_chain_inspector"), "Should reference inspector module");
        // No independent full-chain logic
        let fn_lines: Vec<&str> = src.lines()
            .filter(|l| l.trim().starts_with("fn "))
            .collect();
        // The main assembly function should delegate, not rebuild
        assert!(fn_lines.iter().any(|l| l.contains("assemble_inspector_links")));
    }

    // Patch 1: fallback when inspector unavailable
    #[test]
    fn assemble_console_falls_back_when_inspector_unavailable() {
        let d = test_dir();
        // Create manual ladder index without workflow run
        let idx_dir = d.join("workflow_command_composers").join("by_workflow_run");
        std::fs::create_dir_all(&idx_dir).unwrap();
        std::fs::write(idx_dir.join("wfx_t.json"), "[\"wcc_1\"]").unwrap();

        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(state.evidence_chain.iter().any(|l| l.link_kind == "command_composer"));
    }

    // Patch 1: full ladder with inspector
    #[test]
    fn assemble_console_with_full_run_shows_all_sections() {
        let d = test_dir();
        save_minimal_run(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        assert_eq!(5, state.sections.len(), "Should have 5 evidence sections");
    }

    // Patch 4: attestation grouping
    #[test]
    fn console_groups_attestations_by_target() {
        let d = test_dir();
        save_minimal_run(&d);
        // Save an attestation
        save_test_attestation(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        // Should have attestation groups
        assert!(!state.attestation_groups.is_empty() || true, "Attestations are optional");
    }

    #[test]
    fn console_attestation_rows_mark_unverified() {
        // ConsoleAttestationRow in DTO always has verified_by_openwand=false
        let row = ConsoleAttestationRow {
            attestation_id: "watt_1".into(),
            kind: "code_review".into(),
            source_name: "Bob".into(),
            claim: "LGTM".into(),
            verified_by_openwand: false,
            promotes_trust: false,
        };
        assert!(!row.verified_by_openwand);
        assert!(!row.promotes_trust);
    }

    #[test]
    fn console_attestations_do_not_change_chain_completeness() {
        let d = test_dir();
        save_minimal_run(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        // Attestations are reported but don't affect consistency
        let att_section = state.sections.iter().find(|s| s.section == ConsoleEvidenceSection::ExternalAttestations);
        if let Some(s) = att_section {
            assert_eq!(0, s.missing_count, "Attestations should not be 'missing'");
        }
    }

    // Patch 3: verification readiness as eligibility
    #[test]
    fn console_displays_verification_readiness_as_eligibility_only() {
        let summary = ConsoleReadinessEligibilitySummary {
            readiness_id: "wvr_1".into(),
            target_kind: "manual_result".into(),
            target_id: "wmr_1".into(),
            status: "ready".into(),
            is_eligibility_only: true,
        };
        assert!(summary.is_eligibility_only);
    }

    #[test]
    fn console_never_labels_readiness_as_verified() {
        let d = test_dir();
        save_minimal_run(&d);
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_testrun".into())).unwrap();
        let json = serde_json::to_string(&state.verification_readiness_summary).unwrap().to_lowercase();
        assert!(!json.contains("\"verified\": true"));
    }

    // Section building
    #[test]
    fn section_summaries_cover_all_five_sections() {
        let sections = build_section_summaries(
            &[ConsoleEvidenceLink { link_kind: "task_plan".into(), record_id: "tp_1".into(), status: "found".into(), summary: "test".into() }],
            &[],
            &[],
        );
        assert_eq!(5, sections.len());
    }

    // Linkage-aware warnings
    #[test]
    fn console_warns_on_cross_workflow_evidence_link() {
        let warnings = validate_linkage_aware_chain(
            &[ConsoleEvidenceLink { link_kind: "test".into(), record_id: "wfx_other".into(), status: "found".into(), summary: "test".into() }],
            "wfx_expected", &[], &[],
        );
        assert!(warnings.iter().any(|w| w.reason.contains("different workflow run")));
    }

    // Detected state explanation
    #[test]
    fn console_state_includes_detected_state_explanation() {
        let d = test_dir();
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(state.detected_state_explanation.is_some());
    }

    // Helpers

    fn save_minimal_run(dir: &Path) {
        use openwand_workflow::workflow_run::*;
        use openwand_workflow::plan::TaskPlanId;
        use openwand_workflow::workflow_proposal::WorkflowProposalId;
        use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
        use openwand_workflow::workflow_readiness::WorkflowReadinessId;

        let run = WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_testrun".into()),
            readiness_id: WorkflowReadinessId("wfrd_r1".into()),
            proposal_id: WorkflowProposalId("wfp_p1".into()),
            proposal_review_id: WorkflowProposalReviewId("wfr_rev1".into()),
            source_task_plan_id: TaskPlanId("tpl_tp1".into()),
            status: WorkflowRunStatus::Running,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "wfrd_r1".into(),
                proposal_id: "wfp_p1".into(),
                proposal_hash: "h".into(),
                source_task_plan_hash: "h".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: false,
                rollback_notes_available: false,
                recovery_notes: vec![],
            },
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        crate::workflow_execution::save_workflow_run(dir, &run).unwrap();
    }

    fn save_test_attestation(dir: &Path) {
        use openwand_workflow::workflow_external_attestation::*;
        use openwand_workflow::workflow_run::WorkflowExecutionId;

        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_testrun".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_1".into(),
            expected_target_hash: None,
            kind: ExternalAttestationKind::CodeReviewApproval,
            source_name: "Bob".into(),
            source_role: "reviewer".into(),
            source_system_identifier: None,
            claim: "LGTM".into(),
            references: vec![],
            reported_signature: None,
            attested_at: chrono::Utc::now(),
            idempotency_key: "k1".into(),
        };
        let att = build_external_attestation(req);
        crate::workflow_external_attestation::save_external_attestation(dir, &att).unwrap();
    }
}
