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
            detected_state: WorkflowDetectedLoopState::NeedsInitialContinuationProposal,
        };
        let json = serde_json::to_string(&state).unwrap();
        let back: WorkflowLoopState = serde_json::from_str(&json).unwrap();
        assert_eq!(state.workflow_execution_id, back.workflow_execution_id);
        assert_eq!(state.detected_state, back.detected_state);
    }
}
