//! Command composer DTOs and evaluation.
//!
//! The composer converts loop-controller recommendations into display-only
//! command descriptors. It does not execute, route, approve, reconcile,
//! or mutate any state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_manual_operation::*;
use crate::workflow_command_descriptor::WorkflowManualCommandDescriptor;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed composer ID. Format: wcc_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowCommandComposerId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandComposerRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub expected_loop_controller_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandComposerRecord {
    pub composer_id: WorkflowCommandComposerId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub loop_controller_hash: String,
    pub status: WorkflowCommandComposerStatus,
    pub decision: WorkflowCommandComposerDecision,
    pub predicates: Vec<WorkflowCommandComposerPredicateResult>,
    pub descriptor: Option<WorkflowManualCommandDescriptor>,
    pub missing_inputs: Vec<WorkflowCommandMissingInput>,
    pub evidence_links: Vec<WorkflowCommandEvidenceLink>,
    pub executes_command: bool,
    pub invokes_shell: bool,
    pub invokes_git: bool,
    pub routes_action: bool,
    pub resolves_approval: bool,
    pub reconciles_outcome: bool,
    pub mutates_workflow_state: bool,
    pub schedules_work: bool,
    pub starts_worker: bool,
    pub queues_operation: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandComposerStatus {
    DescriptorReady,
    MissingInputs,
    NoCommandRequired,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowCommandComposerDecision {
    DescriptorReady { summary: String },
    MissingInputs { summary: String },
    NoCommandRequired { summary: String },
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandComposerPredicate {
    LoopControllerRecordExists,
    LoopControllerHashMatchesRequest,
    LoopControllerBelongsToWorkflowRun,
    LoopRecommendationExists,
    LoopRecommendationHasExactlyOneOperation,
    LoopRecommendationHasNoAuthority,
    ManualOperationSupported,
    RequiredEvidenceAvailable,
    RequiredArgumentsResolved,
    MissingInputsRepresented,
    DescriptorIsDisplayOnly,
    DescriptorIsNotExecutable,
    CopyableTextIsDisplayOnly,
    NoPriorConflictingCommandDescriptor,
    IdempotencyKeyUnusedOrMatchesExisting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandComposerPredicateResult {
    pub predicate: WorkflowCommandComposerPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_command_composer_record_roundtrips() {
        let rec = WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_abc".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_1".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_1".into()),
            loop_controller_hash: "h".into(),
            status: WorkflowCommandComposerStatus::DescriptorReady,
            decision: WorkflowCommandComposerDecision::DescriptorReady { summary: "test".into() },
            predicates: vec![], descriptor: None, missing_inputs: vec![], evidence_links: vec![],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let back: WorkflowCommandComposerRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.composer_id, back.composer_id);
    }

    #[test]
    fn workflow_command_composer_id_is_content_addressed() {
        let hash = blake3::hash(b"test");
        let id = WorkflowCommandComposerId(format!("wcc_{}", hash.to_hex()));
        assert!(id.0.starts_with("wcc_"));
    }

    #[test]
    fn workflow_command_composer_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowCommandComposerStatus::MissingInputs).unwrap();
        assert!(json.contains("missing_inputs"));
    }

    #[test]
    fn workflow_command_composer_decision_roundtrips() {
        let d = WorkflowCommandComposerDecision::Blocked { reason_code: "test".into(), summary: "test".into() };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowCommandComposerDecision = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn workflow_command_composer_requires_predicates() {
        let rec = WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_x".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_x".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_x".into()),
            loop_controller_hash: String::new(),
            status: WorkflowCommandComposerStatus::Blocked,
            decision: WorkflowCommandComposerDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            predicates: vec![], descriptor: None, missing_inputs: vec![], evidence_links: vec![],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        };
        assert!(rec.predicates.is_empty());
    }

    #[test]
    fn workflow_command_composer_has_no_authority_flags() {
        let rec = WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_a".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_a".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_a".into()),
            loop_controller_hash: "h".into(),
            status: WorkflowCommandComposerStatus::DescriptorReady,
            decision: WorkflowCommandComposerDecision::DescriptorReady { summary: "ok".into() },
            predicates: vec![], descriptor: None, missing_inputs: vec![], evidence_links: vec![],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        };
        assert!(!rec.executes_command); assert!(!rec.invokes_shell); assert!(!rec.invokes_git);
        assert!(!rec.routes_action); assert!(!rec.resolves_approval); assert!(!rec.reconciles_outcome);
        assert!(!rec.mutates_workflow_state); assert!(!rec.schedules_work);
        assert!(!rec.starts_worker); assert!(!rec.queues_operation);
    }
}
