//! Workflow operator console DTOs — unified read-only display of full evidence ladder.
//!
//! Patch 2: The console observes, summarizes, and links evidence.
//! It does not create evidence records or perform recommended operations.
//! Patch 3: No persistence. Console state is recomputed from existing evidence indexes.
//! Patch 4: Chain consistency checks with warnings.
//! Patch 6: No-authority flags — all false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_loop_state::WorkflowDetectedLoopState;
use crate::workflow_run::{WorkflowExecutionId, WorkflowStageRunStatus};

/// Patch 6: No-authority console DTO. All flags hardcoded false.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOperatorConsoleState {
    pub workflow_execution_id: WorkflowExecutionId,
    pub run_status: String,
    pub stages: Vec<ConsoleStageSummary>,
    pub detected_state: String,
    pub recommendation: Option<ConsoleRecommendation>,
    pub evidence_chain: Vec<ConsoleEvidenceLink>,
    // Patch 4: chain consistency
    pub chain_warnings: Vec<ConsoleChainWarning>,
    pub evidence_chain_consistent: bool,
    pub warnings: Vec<String>,
    pub computed_at: DateTime<Utc>,
    // Patch 6: no-authority flags
    pub creates_route: bool,
    pub executes_tool: bool,
    pub verifies_external_state: bool,
    pub resolves_approval: bool,
    pub reconciles_outcome: bool,
    pub mutates_workflow_state: bool,
    pub creates_run_revision: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
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

/// Patch 4: chain consistency warning.
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

/// Build a console state from loop-state observations.
/// Patch 3: This is a recomputed view, not a persisted record.
pub fn build_console_state(
    workflow_execution_id: WorkflowExecutionId,
    run_status: String,
    stages: Vec<ConsoleStageSummary>,
    detected_state: &WorkflowDetectedLoopState,
    recommendation: Option<ConsoleRecommendation>,
    evidence_chain: Vec<ConsoleEvidenceLink>,
    chain_warnings: Vec<ConsoleChainWarning>,
) -> WorkflowOperatorConsoleState {
    let evidence_chain_consistent = chain_warnings.is_empty();
    WorkflowOperatorConsoleState {
        workflow_execution_id,
        run_status,
        stages,
        detected_state: format!("{:?}", detected_state).to_lowercase(),
        recommendation,
        evidence_chain,
        chain_warnings,
        evidence_chain_consistent,
        warnings: vec![],
        computed_at: Utc::now(),
        // Patch 6: all false
        creates_route: false,
        executes_tool: false,
        verifies_external_state: false,
        resolves_approval: false,
        reconciles_outcome: false,
        mutates_workflow_state: false,
        creates_run_revision: false,
        appends_trace: false,
        writes_memory: false,
    }
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

    // If gate exists but readiness doesn't → mismatch
    if reconciliation_gate_id.is_some() && reconciliation_readiness_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "reconciliation_readiness".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Reconciliation gate exists but no readiness record".into(),
        });
    }

    // If readiness exists but review doesn't → mismatch
    if reconciliation_readiness_id.is_some() && manual_result_review_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "manual_result_review".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Reconciliation readiness exists but no manual result review".into(),
        });
    }

    // If review exists but result doesn't → mismatch
    if manual_result_review_id.is_some() && manual_result_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "manual_result".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Manual result review exists but no manual result".into(),
        });
    }

    // If result exists but command review doesn't → mismatch
    if manual_result_id.is_some() && command_review_id.is_none() {
        warnings.push(ConsoleChainWarning {
            link_kind: "command_review".into(),
            expected_id: "required".into(),
            actual_id: None,
            reason: "Manual result exists but no command review".into(),
        });
    }

    // If command review exists but command composer doesn't → mismatch
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stage(id: &str, status: &str) -> ConsoleStageSummary {
        ConsoleStageSummary { stage_id: id.into(), title: format!("Stage {}", id), status: status.into(), order: 0 }
    }

    #[test]
    fn console_state_roundtrips() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()),
            "suspended".into(),
            vec![test_stage("s1", "suspended")],
            &WorkflowDetectedLoopState::NeedsCommandDescriptor,
            None, vec![], vec![],
        );
        let json = serde_json::to_string(&state).unwrap();
        let back: WorkflowOperatorConsoleState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.workflow_execution_id, back.workflow_execution_id);
    }

    #[test]
    fn console_state_is_consistent_when_no_warnings() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::NeedsCommandDescriptor,
            None, vec![], vec![],
        );
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
        );
        assert!(!state.evidence_chain_consistent);
    }

    // Patch 6: authority flag tests
    #[test]
    fn operator_console_has_no_authority_flags() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        assert!(!state.creates_route);
        assert!(!state.executes_tool);
        assert!(!state.verifies_external_state);
        assert!(!state.resolves_approval);
        assert!(!state.reconciles_outcome);
        assert!(!state.mutates_workflow_state);
        assert!(!state.creates_run_revision);
        assert!(!state.appends_trace);
        assert!(!state.writes_memory);
    }

    #[test]
    fn operator_console_does_not_verify_external_state() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        assert!(!state.verifies_external_state);
    }

    #[test]
    fn operator_console_does_not_reconcile_or_create_revision() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        assert!(!state.reconciles_outcome);
        assert!(!state.creates_run_revision);
    }

    #[test]
    fn operator_console_does_not_route_or_execute() {
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        assert!(!state.creates_route);
        assert!(!state.executes_tool);
    }

    // Patch 2: sub-boundary tests
    #[test]
    fn operator_console_does_not_create_loop_controller_record() {
        // Console state struct has no controller_id field
        let state = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
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
            vec![], vec![],
        );
        assert!(state.recommendation.is_some());
        // The console only displays the recommendation; it doesn't perform it
        assert!(!state.executes_tool);
        assert!(!state.creates_route);
    }

    // Patch 3: no persistence tests
    #[test]
    fn operator_console_state_is_recomputed_not_persisted() {
        let s1 = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        let s2 = build_console_state(
            WorkflowExecutionId("wfx_t".into()), "suspended".into(),
            vec![], &WorkflowDetectedLoopState::Inconclusive,
            None, vec![], vec![],
        );
        // Both computed from same inputs; structurally equivalent (ignoring timestamp)
        assert_eq!(s1.workflow_execution_id, s2.workflow_execution_id);
    }

    #[test]
    fn operator_console_creates_no_console_record() {
        let src = include_str!("workflow_operator_console.rs");
        // No save/persist/write function
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
        // Check only non-test public functions (test fn names contain these words)
        let pub_fns: Vec<&str> = src.lines()
            .filter(|l| l.trim().starts_with("pub fn"))
            .collect();
        assert!(!pub_fns.iter().any(|l| l.contains("write") || l.contains("create")));
    }

    // Patch 4: chain consistency tests
    #[test]
    fn console_warns_on_manual_result_chain_mismatch() {
        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), None, None, None, None,
        );
        // command_composer + command_review exist but no manual_result — OK, not a mismatch
        assert!(warnings.is_empty());

        // Now: review exists but result doesn't
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
    fn console_warns_on_manual_reconciliation_gate_mismatch() {
        // Gate exists but readiness doesn't — this is caught by the readiness check
        let warnings = validate_manual_result_chain(
            Some("wcc_1"), Some("wcrv_1"), Some("wmr_1"), Some("wmrr_1"), None, Some("wmrrg_1"),
        );
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
}
