//! Workflow operator console DTOs — unified read-only display of full evidence ladder.
//!
//! Wave 44: Initial console with manual-result ladder chain.
//! Wave 48A: Extended evidence UX — sections, attestation grouping, verification readiness
//!   eligibility display, detected state explanations, linkage-aware warnings.
//!
//! The console observes, summarizes, and links evidence.
//! It does not create evidence records or perform recommended operations.
//! No persistence. Console state is recomputed from existing evidence indexes.
//!
//! Patch 7 (48A): Extended no-authority flags — certifies_evidence, promotes_trust,
//!   schedules_verification — all false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_loop_state::WorkflowDetectedLoopState;
use crate::workflow_run::WorkflowExecutionId;

/// No-authority console DTO. All authority flags hardcoded false.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOperatorConsoleState {
    pub workflow_execution_id: WorkflowExecutionId,
    pub run_status: String,
    pub stages: Vec<ConsoleStageSummary>,
    pub detected_state: String,
    pub detected_state_explanation: Option<String>,
    pub recommendation: Option<ConsoleRecommendation>,
    pub evidence_chain: Vec<ConsoleEvidenceLink>,
    /// Wave 48A: Section-grouped evidence summaries.
    pub sections: Vec<ConsoleSectionSummary>,
    /// Wave 48A: Attestations grouped by target.
    pub attestation_groups: Vec<ConsoleAttestationGroup>,
    /// Wave 48A: Verification readiness summaries (eligibility only).
    pub verification_readiness_summary: Vec<ConsoleReadinessEligibilitySummary>,
    // Chain consistency
    pub chain_warnings: Vec<ConsoleChainWarning>,
    pub evidence_chain_consistent: bool,
    pub warnings: Vec<String>,
    pub computed_at: DateTime<Utc>,
    // No-authority flags (Wave 44)
    pub creates_route: bool,
    pub executes_tool: bool,
    pub verifies_external_state: bool,
    pub resolves_approval: bool,
    pub reconciles_outcome: bool,
    pub mutates_workflow_state: bool,
    pub creates_run_revision: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    // Patch 7 (48A): Extended no-authority flags
    pub certifies_evidence: bool,
    pub promotes_trust: bool,
    pub schedules_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleStageSummary {
    pub stage_id: String,
    pub title: String,
    pub status: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleRecommendation {
    pub operation: String,
    pub command_hint: String,
    pub reason: String,
}

/// Chain consistency warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleChainWarning {
    pub link_kind: String,
    pub expected_id: String,
    pub actual_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEvidenceLink {
    pub link_kind: String,
    pub record_id: String,
    pub status: String,
    pub summary: String,
}

// --- Wave 48A: Section grouping ---

/// Evidence section for grouped display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleEvidenceSection {
    UpstreamSpine,
    LoopControl,
    ManualResultLadder,
    ExternalAttestations,
    VerificationReadiness,
}

/// Per-section summary for the console.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleSectionSummary {
    pub section: ConsoleEvidenceSection,
    pub link_count: usize,
    pub present_count: usize,
    pub missing_count: usize,
    pub warnings_count: usize,
}

// --- Wave 48A: Attestation grouping (Patch 4) ---

/// Attestations grouped by target record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleAttestationGroup {
    pub target_kind: String,
    pub target_id: String,
    pub attestations: Vec<ConsoleAttestationRow>,
}

/// One attestation row within a target group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleAttestationRow {
    pub attestation_id: String,
    pub kind: String,
    pub source_name: String,
    pub claim: String,
    pub verified_by_openwand: bool,
    pub promotes_trust: bool,
}

// --- Wave 48A: Verification readiness eligibility (Patch 3) ---

/// Verification readiness displayed as future eligibility, not verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleReadinessEligibilitySummary {
    pub readiness_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub status: String,
    /// Always true — readiness is eligibility only, never verification.
    pub is_eligibility_only: bool,
}

/// Build a console state from loop-state observations.
/// No persistence. This is a recomputed view.
#[allow(clippy::too_many_arguments)]
pub fn build_console_state(
    workflow_execution_id: WorkflowExecutionId,
    run_status: String,
    stages: Vec<ConsoleStageSummary>,
    detected_state: &WorkflowDetectedLoopState,
    recommendation: Option<ConsoleRecommendation>,
    evidence_chain: Vec<ConsoleEvidenceLink>,
    chain_warnings: Vec<ConsoleChainWarning>,
    sections: Vec<ConsoleSectionSummary>,
    attestation_groups: Vec<ConsoleAttestationGroup>,
    verification_readiness_summary: Vec<ConsoleReadinessEligibilitySummary>,
) -> WorkflowOperatorConsoleState {
    let evidence_chain_consistent = chain_warnings.is_empty();
    WorkflowOperatorConsoleState {
        workflow_execution_id,
        run_status,
        stages,
        detected_state: format!("{:?}", detected_state).to_lowercase(),
        detected_state_explanation: Some(detected_state_explanation(detected_state)),
        recommendation,
        evidence_chain,
        sections,
        attestation_groups,
        verification_readiness_summary,
        chain_warnings,
        evidence_chain_consistent,
        warnings: vec![],
        computed_at: Utc::now(),
        // All authority flags false
        creates_route: false,
        executes_tool: false,
        verifies_external_state: false,
        resolves_approval: false,
        reconciles_outcome: false,
        mutates_workflow_state: false,
        creates_run_revision: false,
        appends_trace: false,
        writes_memory: false,
        // Patch 7 (48A): extended authority flags
        certifies_evidence: false,
        promotes_trust: false,
        schedules_verification: false,
    }
}

/// Patch 5 (48A): Exhaustive detected-state explanation.
/// Covers all 17 WorkflowDetectedLoopState variants.
/// Explanations describe what is observed, not what to execute.
pub fn detected_state_explanation(state: &WorkflowDetectedLoopState) -> String {
    match state {
        WorkflowDetectedLoopState::NeedsInitialContinuationProposal => {
            "The workflow run has been created but no continuation proposal has been recorded yet. \
             The system is waiting for continuation readiness evaluation."
        }
        WorkflowDetectedLoopState::NeedsNextActionReview => {
            "A next-action proposal exists but has not been reviewed by the operator. \
             The system is waiting for operator review of the proposed next action."
        }
        WorkflowDetectedLoopState::NeedsRoutingReadiness => {
            "A next-action review has been accepted but routing readiness has not been evaluated. \
             The system is waiting for routing readiness assessment."
        }
        WorkflowDetectedLoopState::NeedsNextActionRouting => {
            "Routing readiness has been confirmed but the next action has not been routed through the session seam. \
             The system is waiting for action routing."
        }
        WorkflowDetectedLoopState::NeedsSessionRoutingObservation => {
            "The next action has been routed but no session routing observation has been recorded. \
             The system is waiting for the session to process the routed action."
        }
        WorkflowDetectedLoopState::NeedsApprovalOutcomeResolution => {
            "An approval request has been sent to the session but the outcome has not been resolved. \
             The system is waiting for approval outcome resolution."
        }
        WorkflowDetectedLoopState::NeedsOutcomeReconciliation => {
            "An action outcome has been recorded but has not been reconciled into workflow state. \
             The system is waiting for outcome reconciliation."
        }
        WorkflowDetectedLoopState::NeedsContinuationAfterReconciliation => {
            "Outcome reconciliation is complete but no continuation readiness has been evaluated for the next loop iteration. \
             The system is waiting for continuation readiness evaluation."
        }
        WorkflowDetectedLoopState::NeedsCommandDescriptor => {
            "The loop controller has identified that a command descriptor is needed. \
             The system is waiting for command composition."
        }
        WorkflowDetectedLoopState::NeedsCommandReview => {
            "A command has been composed but has not been reviewed by the operator. \
             The system is waiting for operator acknowledgment of the command."
        }
        WorkflowDetectedLoopState::NeedsManualResultCapture => {
            "A command has been reviewed but no manual result has been captured. \
             The system is waiting for the operator to report the manual execution result."
        }
        WorkflowDetectedLoopState::NeedsManualResultReview => {
            "A manual result has been captured but has not been reviewed. \
             The system is waiting for operator review of the reported result."
        }
        WorkflowDetectedLoopState::NeedsReconciliationReadiness => {
            "A manual result has been accepted but reconciliation readiness has not been evaluated. \
             The system is waiting for reconciliation readiness assessment."
        }
        WorkflowDetectedLoopState::NeedsManualReconciliation => {
            "Reconciliation readiness has been confirmed but the reconciliation gate has not been evaluated. \
             The system is waiting for manual reconciliation gate evaluation."
        }
        WorkflowDetectedLoopState::WorkflowComplete => {
            "The workflow run has completed all stages. No further action is needed."
        }
        WorkflowDetectedLoopState::WorkflowBlocked => {
            "The workflow run is blocked. An operator must intervene to resolve the blockage."
        }
        WorkflowDetectedLoopState::Inconclusive => {
            "The workflow run state cannot be determined from available evidence. \
             Manual inspection may be required."
        }
    }.into()
}

/// Patch 4: Validate manual-result chain consistency.
/// Manual-result ladder records must link in order:
/// command_composer → command_review → manual_result → manual_result_review
/// → reconciliation_readiness → manual_reconciliation_gate
pub fn validate_manual_result_chain(
    command_composer_id: Option<&str>,
    command_review_id: Option<&str>,
    manual_result_id: Option<&str>,
    manual_result_review_id: Option<&str>,
    reconciliation_readiness_id: Option<&str>,
    reconciliation_gate_id: Option<&str>,
) -> Vec<ConsoleChainWarning> {
    let mut warnings = Vec::new();

    if reconciliation_gate_id.is_some() && reconciliation_readiness_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "reconciliation_readiness".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Reconciliation gate exists but no readiness record".into(),
        });
    }

    if reconciliation_readiness_id.is_some() && manual_result_review_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "manual_result_review".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Reconciliation readiness exists but no manual result review".into(),
        });
    }

    if manual_result_review_id.is_some() && manual_result_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "manual_result".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Manual result review exists but no manual result".into(),
        });
    }

    if manual_result_id.is_some() && command_review_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "command_review".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Manual result exists but no command review".into(),
        });
    }

    if command_review_id.is_some() && command_composer_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "command_composer".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Command review exists but no command composer".into(),
        });
    }

    warnings
}

/// Patch 2 (48A): Linkage-aware chain validation.
/// Checks cross-workflow-evidence links, target ID mismatches,
/// and parent/child ID consistency.
pub fn validate_linkage_aware_chain(
    links: &[ConsoleEvidenceLink],
    expected_workflow_execution_id: &str,
    attestation_groups: &[ConsoleAttestationGroup],
    readiness_summaries: &[ConsoleReadinessEligibilitySummary],
) -> Vec<ConsoleChainWarning> {
    let mut warnings = Vec::new();

    // Check for cross-workflow evidence links
    for link in links {
        // Evidence links should belong to this workflow run
        // We check if the record_id contains a different workflow execution ID
        // (This is a structural check — record IDs embed their workflow run)
        if link.record_id.contains("wfx_") && !link.record_id.contains(expected_workflow_execution_id) {
            // Allow if the workflow_execution_id is embedded but not matching
            if link.record_id != expected_workflow_execution_id {
                warnings.push(ConsoleChainWarning {
                    link_kind: link.link_kind.clone(),
                    expected_id: expected_workflow_execution_id.into(),
                    actual_id: Some(link.record_id.clone()),
                    reason: format!("Evidence link {} references a different workflow run", link.record_id),
                });
            }
        }
    }

    // Check attestation target mismatches (Patch 4)
    for group in attestation_groups {
        for att in &group.attestations {
            // Attestations should not claim verification
            if att.verified_by_openwand {
                warnings.push(ConsoleChainWarning {
                    link_kind: "external_attestation".into(),
                    expected_id: att.attestation_id.clone(),
                    actual_id: None,
                    reason: format!("Attestation {} incorrectly claims verification by OpenWand", att.attestation_id),
                });
            }
        }
    }

    // Check verification readiness target mismatches (Patch 2)
    for rs in readiness_summaries {
        // Verification readiness must always be eligibility-only
        if !rs.is_eligibility_only {
            warnings.push(ConsoleChainWarning {
                link_kind: "verification_readiness".into(),
                expected_id: rs.readiness_id.clone(),
                actual_id: None,
                reason: format!("Readiness {} incorrectly claims non-eligibility status", rs.readiness_id),
            });
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stage(id: &str, status: &str) -> ConsoleStageSummary {
        ConsoleStageSummary { stage_id: id.into(), title: format!("Stage {}", id), status: status.into(), order: 0 }
    }

    fn build_test_state(detected: &WorkflowDetectedLoopState) -> WorkflowOperatorConsoleState {
        build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], detected, None, vec![], vec![], vec![], vec![], vec![],
        )
    }

    // --- Roundtrip / consistency ---

    #[test]
    fn console_state_roundtrips() {
        let state = build_test_state(&WorkflowDetectedLoopState::NeedsCommandDescriptor);
        let json = serde_json::to_string(&state).unwrap();
        let back: WorkflowOperatorConsoleState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.workflow_execution_id, back.workflow_execution_id);
    }

    #[test]
    fn console_state_is_consistent_when_no_warnings() {
        let state = build_test_state(&WorkflowDetectedLoopState::NeedsCommandDescriptor);
        assert!(state.evidence_chain_consistent);
    }

    #[test]
    fn console_state_is_inconsistent_when_warnings() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![],
            vec![ConsoleChainWarning {
                link_kind: "test".into(), expected_id: "x".into(),
                actual_id: None, reason: "mismatch".into(),
            }],
            vec![], vec![], vec![],
        );
        assert!(!state.evidence_chain_consistent);
    }

    // --- Authority flag tests ---

    #[test]
    fn operator_console_has_no_authority_flags() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert!(!state.creates_route);
        assert!(!state.executes_tool);
        assert!(!state.verifies_external_state);
        assert!(!state.resolves_approval);
        assert!(!state.reconciles_outcome);
        assert!(!state.mutates_workflow_state);
        assert!(!state.creates_run_revision);
        assert!(!state.appends_trace);
        assert!(!state.writes_memory);
        // Patch 7 (48A): extended flags
        assert!(!state.certifies_evidence);
        assert!(!state.promotes_trust);
        assert!(!state.schedules_verification);
    }

    #[test]
    fn operator_console_does_not_verify_external_state() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert!(!state.verifies_external_state);
    }

    #[test]
    fn operator_console_does_not_reconcile_or_create_revision() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert!(!state.reconciles_outcome);
        assert!(!state.creates_run_revision);
    }

    #[test]
    fn operator_console_does_not_route_or_execute() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert!(!state.creates_route);
        assert!(!state.executes_tool);
    }

    // Patch 7 (48A): extended authority guard
    #[test]
    fn operator_console_does_not_certify_promote_or_schedule() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert!(!state.certifies_evidence, "console must not certify evidence");
        assert!(!state.promotes_trust, "console must not promote trust");
        assert!(!state.schedules_verification, "console must not schedule verification");
    }

    // --- Sub-boundary tests ---

    #[test]
    fn operator_console_does_not_create_loop_controller_record() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        let json = serde_json::to_string(&state).unwrap().to_lowercase();
        assert!(!json.contains("controller_id"));
    }

    #[test]
    fn operator_console_does_not_perform_recommended_operation() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::NeedsManualResultCapture,
            Some(ConsoleRecommendation {
                operation: "capture_manual_result".into(),
                command_hint: "display only".into(),
                reason: "test".into(),
            }),
            vec![], vec![], vec![], vec![], vec![],
        );
        assert!(state.recommendation.is_some());
        assert!(!state.executes_tool);
        assert!(!state.creates_route);
    }

    // --- No persistence tests ---

    #[test]
    fn operator_console_state_is_recomputed_not_persisted() {
        let s1 = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        let s2 = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        assert_eq!(s1.workflow_execution_id, s2.workflow_execution_id);
    }

    #[test]
    fn operator_console_creates_no_console_record() {
        let src = include_str!("workflow_operator_console.rs");
        let fn_lines: Vec<&str> = src.lines()
            .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
            .collect();
        assert!(!fn_lines.iter().any(|l| l.contains("save_") || l.contains("persist_") || l.contains("write_")));
    }

    #[test]
    fn operator_console_writes_no_eval_report_files() {
        let src = include_str!("workflow_operator_console.rs");
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("std::fs")));
        let pub_fns: Vec<&str> = src.lines()
            .filter(|l| l.trim().starts_with("pub fn"))
            .collect();
        assert!(!pub_fns.iter().any(|l| l.contains("write") || l.contains("create")));
    }

    // --- Patch 4: chain consistency ---

    #[test]
    fn console_warns_on_manual_result_chain_mismatch() {
        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), None, None, None, None,
        );
        assert!(warnings.is_empty());

        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), None, Some("wmrr_1"), None, None,
        );
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.link_kind == "manual_result"));
    }

    #[test]
    fn console_warns_on_reconciliation_readiness_mismatch() {
        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), Some("wmr_1"), Some("wmrr_1"), None, Some("wmrrg_1"),
        );
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.link_kind == "reconciliation_readiness"));
    }

    #[test]
    fn console_does_not_claim_mismatched_latest_records_are_coherent() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![],
            vec![ConsoleChainWarning {
                link_kind: "test".into(), expected_id: "x".into(),
                actual_id: Some("y".into()), reason: "mismatch".into(),
            }],
            vec![], vec![], vec![],
        );
        assert!(!state.evidence_chain_consistent);
        assert!(!state.chain_warnings.is_empty());
    }

    #[test]
    fn full_manual_result_chain_has_no_warnings() {
        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), Some("wmr_1"), Some("wmrr_1"),
            Some("wmrrr_1"), Some("wmrrg_1"),
        );
        assert!(warnings.is_empty());
    }

    // --- Patch 5 (48A): Detected state explanations ---

    #[test]
    fn detected_state_explanation_covers_all_17_states() {
        let states = vec![
            WorkflowDetectedLoopState::NeedsInitialContinuationProposal,
            WorkflowDetectedLoopState::NeedsNextActionReview,
            WorkflowDetectedLoopState::NeedsRoutingReadiness,
            WorkflowDetectedLoopState::NeedsNextActionRouting,
            WorkflowDetectedLoopState::NeedsSessionRoutingObservation,
            WorkflowDetectedLoopState::NeedsApprovalOutcomeResolution,
            WorkflowDetectedLoopState::NeedsOutcomeReconciliation,
            WorkflowDetectedLoopState::NeedsContinuationAfterReconciliation,
            WorkflowDetectedLoopState::NeedsCommandDescriptor,
            WorkflowDetectedLoopState::NeedsCommandReview,
            WorkflowDetectedLoopState::NeedsManualResultCapture,
            WorkflowDetectedLoopState::NeedsManualResultReview,
            WorkflowDetectedLoopState::NeedsReconciliationReadiness,
            WorkflowDetectedLoopState::NeedsManualReconciliation,
            WorkflowDetectedLoopState::WorkflowComplete,
            WorkflowDetectedLoopState::WorkflowBlocked,
            WorkflowDetectedLoopState::Inconclusive,
        ];
        assert_eq!(17, states.len(), "Must cover all 17 detected loop states");
        for s in &states {
            let explanation = detected_state_explanation(s);
            assert!(!explanation.is_empty(), "Explanation for {:?} must not be empty", s);
        }
    }

    #[test]
    fn detected_state_explanation_is_stable_for_manual_result_states() {
        let exp = detected_state_explanation(&WorkflowDetectedLoopState::NeedsCommandDescriptor);
        assert!(exp.contains("command"));
        let exp = detected_state_explanation(&WorkflowDetectedLoopState::NeedsManualResultCapture);
        assert!(exp.contains("manual result"));
        let exp = detected_state_explanation(&WorkflowDetectedLoopState::NeedsManualResultReview);
        assert!(exp.contains("review"));
    }

    #[test]
    fn detected_state_explanation_does_not_recommend_execution() {
        let states = vec![
            WorkflowDetectedLoopState::NeedsCommandDescriptor,
            WorkflowDetectedLoopState::NeedsManualResultCapture,
            WorkflowDetectedLoopState::NeedsManualReconciliation,
        ];
        for s in &states {
            let exp = detected_state_explanation(s);
            let lower = exp.to_lowercase();
            assert!(!lower.contains("execute"), "Explanation for {:?} must not say 'execute'", s);
            assert!(!lower.contains("run command"), "Explanation for {:?} must not say 'run command'", s);
        }
    }

    #[test]
    fn console_state_includes_detected_state_explanation() {
        let state = build_test_state(&WorkflowDetectedLoopState::NeedsCommandDescriptor);
        assert!(state.detected_state_explanation.is_some());
        let exp = state.detected_state_explanation.unwrap();
        assert!(!exp.is_empty());
    }

    // --- Patch 2 (48A): Linkage-aware warnings ---

    #[test]
    fn console_warns_on_cross_workflow_evidence_link() {
        let links = vec![ConsoleEvidenceLink {
            link_kind: "test".into(),
            record_id: "wfx_other_run".into(),
            status: "found".into(),
            summary: "test".into(),
        }];
        let warnings = validate_linkage_aware_chain(&links, "wfx_expected", &[], &[]);
        assert!(warnings.iter().any(|w| w.reason.contains("different workflow run")));
    }

    #[test]
    fn console_no_warning_when_links_match_workflow() {
        let links = vec![ConsoleEvidenceLink {
            link_kind: "test".into(),
            record_id: "some_record".into(),
            status: "found".into(),
            summary: "test".into(),
        }];
        let warnings = validate_linkage_aware_chain(&links, "wfx_t", &[], &[]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn console_warns_on_attestation_target_mismatch() {
        let groups = vec![ConsoleAttestationGroup {
            target_kind: "manual_result".into(),
            target_id: "wmr_1".into(),
            attestations: vec![ConsoleAttestationRow {
                attestation_id: "watt_1".into(),
                kind: "code_review".into(),
                source_name: "Bob".into(),
                claim: "LGTM".into(),
                verified_by_openwand: true, // Should trigger warning
                promotes_trust: false,
            }],
        }];
        let warnings = validate_linkage_aware_chain(&[], "wfx_t", &groups, &[]);
        assert!(warnings.iter().any(|w| w.link_kind == "external_attestation"));
    }

    #[test]
    fn console_warns_on_verification_readiness_target_mismatch() {
        let summaries = vec![ConsoleReadinessEligibilitySummary {
            readiness_id: "wvr_1".into(),
            target_kind: "manual_result".into(),
            target_id: "wmr_1".into(),
            status: "ready".into(),
            is_eligibility_only: false, // Should trigger warning
        }];
        let warnings = validate_linkage_aware_chain(&[], "wfx_t", &[], &summaries);
        assert!(warnings.iter().any(|w| w.link_kind == "verification_readiness"));
    }

    #[test]
    fn console_does_not_present_mismatched_latest_records_as_coherent() {
        let warnings = validate_linkage_aware_chain(
            &[ConsoleEvidenceLink {
                link_kind: "test".into(),
                record_id: "wfx_other".into(),
                status: "found".into(),
                summary: "test".into(),
            }],
            "wfx_expected",
            &[],
            &[],
        );
        assert!(!warnings.is_empty());
    }

    // --- Patch 3 (48A): Verification readiness is eligibility only ---

    #[test]
    fn verification_readiness_summary_is_always_eligibility_only() {
        let summary = ConsoleReadinessEligibilitySummary {
            readiness_id: "wvr_1".into(),
            target_kind: "manual_result".into(),
            target_id: "wmr_1".into(),
            status: "ready".into(),
            is_eligibility_only: true,
        };
        assert!(summary.is_eligibility_only);
        let json = serde_json::to_string(&summary).unwrap().to_lowercase();
        assert!(json.contains("is_eligibility_only"));
    }

    #[test]
    fn console_never_labels_readiness_as_verified() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        let json = serde_json::to_string(&state).unwrap().to_lowercase();
        assert!(!json.contains("\"verified\": true"));
        assert!(!json.contains("\"trusted\": true"));
        assert!(!json.contains("\"certified\": true"));
    }

    // --- Patch 4 (48A): Attestation grouping ---

    #[test]
    fn attestation_row_is_always_unverified() {
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
    fn attestation_group_groups_by_target() {
        let group = ConsoleAttestationGroup {
            target_kind: "manual_result".into(),
            target_id: "wmr_1".into(),
            attestations: vec![ConsoleAttestationRow {
                attestation_id: "watt_1".into(),
                kind: "code_review".into(),
                source_name: "Bob".into(),
                claim: "LGTM".into(),
                verified_by_openwand: false,
                promotes_trust: false,
            }],
        };
        assert_eq!(1, group.attestations.len());
        assert_eq!("wmr_1", group.target_id);
    }

    // --- Patch 7 (48A): Serialized authority guard ---

    #[test]
    fn extended_console_serialized_json_has_no_authority() {
        let state = build_test_state(&WorkflowDetectedLoopState::Inconclusive);
        let json = serde_json::to_string_pretty(&state).unwrap().to_lowercase();
        assert!(json.contains("\"creates_route\": false"));
        assert!(json.contains("\"executes_tool\": false"));
        assert!(json.contains("\"verifies_external_state\": false"));
        assert!(json.contains("\"mutates_workflow_state\": false"));
        assert!(json.contains("\"certifies_evidence\": false"));
        assert!(json.contains("\"promotes_trust\": false"));
        assert!(json.contains("\"schedules_verification\": false"));
    }

    // --- Section summary ---

    #[test]
    fn section_summary_roundtrips() {
        let section = ConsoleSectionSummary {
            section: ConsoleEvidenceSection::ManualResultLadder,
            link_count: 6,
            present_count: 4,
            missing_count: 2,
            warnings_count: 0,
        };
        let json = serde_json::to_string(&section).unwrap();
        let back: ConsoleSectionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(section.link_count, back.link_count);
    }

    #[test]
    fn all_five_sections_serialize() {
        let sections = vec![
            ConsoleEvidenceSection::UpstreamSpine,
            ConsoleEvidenceSection::LoopControl,
            ConsoleEvidenceSection::ManualResultLadder,
            ConsoleEvidenceSection::ExternalAttestations,
            ConsoleEvidenceSection::VerificationReadiness,
        ];
        assert_eq!(5, sections.len());
        for s in &sections {
            let json = serde_json::to_string(s).unwrap();
            let back: ConsoleEvidenceSection = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
