//! Manual operation DTOs — structured display data, not executable commands.
//!
//! Command descriptors are display-only. They are not parsed, executed,
//! passed to shell/git/process APIs, or used as command sources.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualCommandKind {
    WorkflowContinuationPropose,
    WorkflowNextActionReviewApprove,
    WorkflowRoutingReadinessEvaluate,
    WorkflowNextActionRoutingRoute,
    WorkflowActionOutcomeResolve,
    WorkflowReconciliationReconcile,
    WorkflowLoopRecommend,
    InspectBlockedWorkflow,
    NoCommand,
    // Patch 5: manual-result ladder command kinds
    WorkflowCommandCompose,
    WorkflowCommandReviewAcknowledge,
    WorkflowManualResultCapture,
    WorkflowManualResultReviewAccept,
    WorkflowManualResultReconciliationReadinessEvaluate,
    WorkflowManualResultReconciliationGateReconcile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualCommandArgument {
    pub name: String,
    pub value_preview: Option<String>,
    pub source: WorkflowCommandArgumentSource,
    pub required: bool,
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandArgumentSource {
    LoopController,
    WorkflowRun,
    RunRevision,
    ContinuationProposal,
    NextActionReview,
    RoutingReadiness,
    NextActionRouting,
    ActionRoute,
    ActionOutcome,
    Reconciliation,
    OperatorInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandMissingInput {
    pub name: String,
    pub reason: String,
    pub suggested_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandEvidenceLink {
    pub kind: WorkflowCommandEvidenceKind,
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandEvidenceKind {
    LoopController,
    WorkflowRun,
    RunRevision,
    ContinuationProposal,
    NextActionReview,
    RoutingReadiness,
    NextActionRouting,
    ActionRoute,
    ActionOutcome,
    Reconciliation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_command_argument_roundtrips() {
        let arg = WorkflowManualCommandArgument {
            name: "workflow_execution_id".into(),
            value_preview: Some("wfx_1".into()),
            source: WorkflowCommandArgumentSource::WorkflowRun,
            required: true, missing: false,
        };
        let json = serde_json::to_string(&arg).unwrap();
        let back: WorkflowManualCommandArgument = serde_json::from_str(&json).unwrap();
        assert_eq!(arg.name, back.name);
        assert!(!back.missing);
    }

    #[test]
    fn workflow_command_missing_input_roundtrips() {
        let mi = WorkflowCommandMissingInput {
            name: "review_decision".into(),
            reason: "Operator must choose approve/reject/request-changes".into(),
            suggested_source: "OperatorInput".into(),
        };
        let json = serde_json::to_string(&mi).unwrap();
        let back: WorkflowCommandMissingInput = serde_json::from_str(&json).unwrap();
        assert_eq!("review_decision", back.name);
    }
}
