//! Workflow loop state DTOs.
//!
//! Loop state is a read-only observation, not a command.
//! Detected state does not advance, route, approve, reconcile, or mutate.

use serde::{Deserialize, Serialize};
use crate::workflow_run::WorkflowExecutionId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_action_route::WorkflowActionRouteId;
use crate::workflow_action_outcome::WorkflowActionOutcomeId;
use crate::workflow_reconciliation::WorkflowReconciliationId;
use crate::workflow_continuation::WorkflowContinuationReadinessId;
use crate::workflow_continuation::WorkflowNextActionProposalId;
use crate::workflow_next_action_review::WorkflowNextActionReviewId;
use crate::workflow_routing_readiness::WorkflowRoutingReadinessId;
use crate::workflow_next_action_routing_gate::WorkflowNextActionRoutingId;
use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_command_review::WorkflowCommandReviewId;
use crate::workflow_manual_result::WorkflowManualResultId;
use crate::workflow_manual_result_review::WorkflowManualResultReviewId;
use crate::workflow_manual_result_reconciliation_readiness::WorkflowManualResultReconciliationReadinessId;
use crate::workflow_manual_result_reconciliation_gate::WorkflowManualResultReconciliationGateId;

/// Observed state of a workflow run and its linked evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopState {
    pub workflow_execution_id: WorkflowExecutionId,
    pub latest_run_revision_id: Option<WorkflowRunRevisionId>,
    pub run_status: String,
    pub stage_summary: Vec<WorkflowLoopStageSummary>,
    pub latest_route_id: Option<WorkflowActionRouteId>,
    pub latest_outcome_id: Option<WorkflowActionOutcomeId>,
    pub latest_reconciliation_id: Option<WorkflowReconciliationId>,
    pub latest_continuation_readiness_id: Option<WorkflowContinuationReadinessId>,
    pub latest_next_action_proposal_id: Option<WorkflowNextActionProposalId>,
    pub latest_next_action_review_id: Option<WorkflowNextActionReviewId>,
    pub latest_routing_readiness_id: Option<WorkflowRoutingReadinessId>,
    pub latest_next_action_routing_id: Option<WorkflowNextActionRoutingId>,
    // Wave 44: manual-result ladder fields
    pub latest_command_composer_id: Option<WorkflowCommandComposerId>,
    pub latest_command_review_id: Option<WorkflowCommandReviewId>,
    pub latest_manual_result_id: Option<WorkflowManualResultId>,
    pub latest_manual_result_review_id: Option<WorkflowManualResultReviewId>,
    pub latest_reconciliation_readiness_id: Option<WorkflowManualResultReconciliationReadinessId>,
    pub latest_manual_reconciliation_gate_id: Option<WorkflowManualResultReconciliationGateId>,
    pub detected_state: WorkflowDetectedLoopState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopStageSummary {
    pub stage_id: String,
    pub title: String,
    pub status: String,
    pub order: u32,
    pub has_action_request: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDetectedLoopState {
    NeedsInitialContinuationProposal,
    NeedsNextActionReview,
    NeedsRoutingReadiness,
    NeedsNextActionRouting,
    NeedsSessionRoutingObservation,
    NeedsApprovalOutcomeResolution,
    NeedsOutcomeReconciliation,
    NeedsContinuationAfterReconciliation,
    // Wave 44: manual-result ladder detected states (Patch 1: 6 additions)
    NeedsCommandDescriptor,
    NeedsCommandReview,
    NeedsManualResultCapture,
    NeedsManualResultReview,
    NeedsReconciliationReadiness,
    NeedsManualReconciliation,
    WorkflowComplete,
    WorkflowBlocked,
    Inconclusive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_loop_state_roundtrips() {
        let state = WorkflowLoopState {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: Some(WorkflowRunRevisionId("wrr_t".into())),
            run_status: "suspended".into(),
            stage_summary: vec![WorkflowLoopStageSummary {
                stage_id: "s1".into(), title: "Stage 1".into(),
                status: "pending".into(), order: 0, has_action_request: true,
            }],
            latest_route_id: None, latest_outcome_id: None,
            latest_reconciliation_id: None, latest_continuation_readiness_id: None,
            latest_next_action_proposal_id: None, latest_next_action_review_id: None,
            latest_routing_readiness_id: None, latest_next_action_routing_id: None,
            latest_command_composer_id: None, latest_command_review_id: None,
            latest_manual_result_id: None, latest_manual_result_review_id: None,
            latest_reconciliation_readiness_id: None, latest_manual_reconciliation_gate_id: None,
            detected_state: WorkflowDetectedLoopState::NeedsInitialContinuationProposal,
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: WorkflowLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.workflow_execution_id, back.workflow_execution_id);
        assert_eq!(state.detected_state, back.detected_state);
    }

    // Patch 1: verify 15 detected states covering manual-result ladder
    #[test]
    fn workflow_detected_loop_state_covers_manual_result_ladder_states() {
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
        assert_eq!(17, states.len(), "Should have 17 detected states (9 original + 6 manual-result + 2 terminal)");
        // Verify all serialize
        for s in &states {
            let json = serde_json::to_string(s).unwrap();
            let back: WorkflowDetectedLoopState = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
