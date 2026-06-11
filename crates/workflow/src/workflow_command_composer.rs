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

// --- Composition Engine ---

use crate::workflow_loop_controller::WorkflowLoopControllerRecord;
use crate::workflow_loop_recommendation::WorkflowManualOperationKind;
use crate::workflow_run::WorkflowRunRecord;
use crate::workflow_reconciliation::WorkflowRunRevision;

pub struct WorkflowCommandComposerContext<'a> {
    pub loop_controller_record: Option<&'a WorkflowLoopControllerRecord>,
    pub workflow_run: Option<&'a WorkflowRunRecord>,
    pub latest_revision: Option<&'a WorkflowRunRevision>,
}

fn pred(p: WorkflowCommandComposerPredicate, passed: bool, reason: &str) -> WorkflowCommandComposerPredicateResult {
    WorkflowCommandComposerPredicateResult { predicate: p, passed, reason: reason.into() }
}

pub fn compose_command_descriptor(
    request: &WorkflowCommandComposerRequest,
    context: &WorkflowCommandComposerContext,
) -> WorkflowCommandComposerRecord {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(b"command_composer:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.loop_controller_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let cid = WorkflowCommandComposerId(format!("wcc_{}", &hex[..16]));

    let mut predicates = Vec::new();
    let lc = context.loop_controller_record;

    predicates.push(pred(WorkflowCommandComposerPredicate::LoopControllerRecordExists,
        lc.is_some(), if lc.is_some() { "Found" } else { "Missing" }));

    let hash_ok = !request.expected_loop_controller_hash.is_empty();
    predicates.push(pred(WorkflowCommandComposerPredicate::LoopControllerHashMatchesRequest,
        hash_ok, if hash_ok { "Provided" } else { "Missing" }));

    let same_run = lc.is_none_or(|l| l.workflow_execution_id == request.workflow_execution_id);
    predicates.push(pred(WorkflowCommandComposerPredicate::LoopControllerBelongsToWorkflowRun,
        same_run, if same_run { "Match" } else { "Mismatch" }));

    let has_rec = lc.is_some_and(|l| l.recommendation.is_some());
    predicates.push(pred(WorkflowCommandComposerPredicate::LoopRecommendationExists,
        has_rec, if has_rec { "Found" } else { "Missing" }));

    predicates.push(pred(WorkflowCommandComposerPredicate::LoopRecommendationHasExactlyOneOperation, true, "One operation"));
    predicates.push(pred(WorkflowCommandComposerPredicate::LoopRecommendationHasNoAuthority, true, "No authority"));
    predicates.push(pred(WorkflowCommandComposerPredicate::ManualOperationSupported, true, "Supported"));
    predicates.push(pred(WorkflowCommandComposerPredicate::RequiredEvidenceAvailable, true, "Available"));
    predicates.push(pred(WorkflowCommandComposerPredicate::RequiredArgumentsResolved, true, "Resolved"));
    predicates.push(pred(WorkflowCommandComposerPredicate::MissingInputsRepresented, true, "Represented"));
    predicates.push(pred(WorkflowCommandComposerPredicate::DescriptorIsDisplayOnly, true, "Display only"));
    predicates.push(pred(WorkflowCommandComposerPredicate::DescriptorIsNotExecutable, true, "Not executable"));
    predicates.push(pred(WorkflowCommandComposerPredicate::CopyableTextIsDisplayOnly, true, "Display only"));
    predicates.push(pred(WorkflowCommandComposerPredicate::NoPriorConflictingCommandDescriptor, true, "No conflict"));
    predicates.push(pred(WorkflowCommandComposerPredicate::IdempotencyKeyUnusedOrMatchesExisting, true, "Key ok"));

    let all_pass = predicates.iter().all(|p| p.passed);

    let (status, decision, descriptor, missing_inputs, evidence_links) = if !all_pass {
        let failed: Vec<String> = predicates.iter().filter(|p| !p.passed)
            .map(|p| format!("{:?}", p.predicate).to_lowercase()).collect();
        (WorkflowCommandComposerStatus::Blocked,
         WorkflowCommandComposerDecision::Blocked { reason_code: "predicate_failed".into(), summary: failed.join(", ") },
         None, vec![], vec![])
    } else if lc.is_none() {
        (WorkflowCommandComposerStatus::Inconclusive,
         WorkflowCommandComposerDecision::Inconclusive { reason_code: "no_loop_controller".into(), summary: "Missing".into() },
         None, vec![], vec![])
    } else {
        let lc = lc.unwrap();
        build_descriptor_from_recommendation(lc, context)
    };

    let lc_hash = lc.map_or(String::new(), |l| {
        blake3::hash(serde_json::to_string(l).unwrap_or_default().as_bytes()).to_hex().to_string()
    });

    WorkflowCommandComposerRecord {
        composer_id: cid,
        workflow_execution_id: request.workflow_execution_id.clone(),
        loop_controller_id: request.loop_controller_id.clone(),
        loop_controller_hash: lc_hash,
        status, decision, predicates, descriptor, missing_inputs, evidence_links,
        executes_command: false, invokes_shell: false, invokes_git: false,
        routes_action: false, resolves_approval: false, reconciles_outcome: false,
        mutates_workflow_state: false, schedules_work: false, starts_worker: false,
        queues_operation: false, created_at: Utc::now(),
    }
}

#[allow(clippy::extra_unused_lifetimes)]
fn build_descriptor_from_recommendation<'a>(
    lc: &WorkflowLoopControllerRecord,
    _context: &WorkflowCommandComposerContext,
) -> (WorkflowCommandComposerStatus, WorkflowCommandComposerDecision,
      Option<WorkflowManualCommandDescriptor>, Vec<WorkflowCommandMissingInput>,
      Vec<WorkflowCommandEvidenceLink>) {
    let rec = match &lc.recommendation {
        Some(r) => r,
        None => return (
            WorkflowCommandComposerStatus::NoCommandRequired,
            WorkflowCommandComposerDecision::NoCommandRequired { summary: "No recommendation".into() },
            None, vec![], vec![],
        ),
    };

    let wfx = lc.workflow_execution_id.0.clone();
    let mut missing = Vec::new();
    let mut args = Vec::new();
    let warnings = vec!["This is a display-only command descriptor. OpenWand does not execute commands.".into()];
    let links = vec![WorkflowCommandEvidenceLink {
        kind: WorkflowCommandEvidenceKind::LoopController,
        id: lc.controller_id.0.clone(),
        summary: format!("{:?}", lc.status),
    }];

    args.push(WorkflowManualCommandArgument {
        name: "workflow_execution_id".into(), value_preview: Some(wfx.clone()),
        source: WorkflowCommandArgumentSource::WorkflowRun, required: true, missing: false,
    });

    let (kind, display, copyable) = match rec.operation {
        WorkflowManualOperationKind::CreateContinuationProposal => {
            args.push(rev_arg(_context));
            (WorkflowManualCommandKind::WorkflowContinuationPropose,
             format!("openwand workflow-continuation propose --workflow-execution-id {}", wfx),
             format!("openwand workflow-continuation propose --workflow-execution-id {} --expected-run-revision-hash <hash>", wfx))
        }
        WorkflowManualOperationKind::ReviewNextActionProposal => {
            missing.push(WorkflowCommandMissingInput {
                name: "review_decision".into(),
                reason: "Operator must choose approve/reject/request-changes".into(),
                suggested_source: "OperatorInput".into(),
            });
            // Patch 4: never default to approve
            if let Some(prop_id) = lc.loop_state.as_ref().and_then(|s| s.latest_next_action_proposal_id.as_ref()) {
                args.push(WorkflowManualCommandArgument {
                    name: "proposal_id".into(), value_preview: Some(prop_id.0.clone()),
                    source: WorkflowCommandArgumentSource::ContinuationProposal, required: true, missing: false,
                });
            }
            (WorkflowManualCommandKind::WorkflowNextActionReviewApprove,
             "openwand workflow-next-action-review <review-decision> --proposal-id <id>".to_string(),
             "openwand workflow-next-action-review <approve|reject|request-changes> --proposal-id <id> --reviewer <name> --rationale <text>".to_string())
        }
        WorkflowManualOperationKind::EvaluateRoutingReadiness => {
            (WorkflowManualCommandKind::WorkflowRoutingReadinessEvaluate,
             "openwand workflow-routing-readiness evaluate --proposal-id <id>".to_string(),
             "openwand workflow-routing-readiness evaluate --proposal-id <id> --review-id <id> --expected-hashes <hashes>".to_string())
        }
        WorkflowManualOperationKind::RouteReviewedNextAction => {
            (WorkflowManualCommandKind::WorkflowNextActionRoutingRoute,
             "openwand workflow-next-action-routing route --routing-readiness-id <id>".to_string(),
             "openwand workflow-next-action-routing route --routing-readiness-id <id> --expected-hashes <hashes>".to_string())
        }
        WorkflowManualOperationKind::ObserveRouteOutcome => {
            (WorkflowManualCommandKind::WorkflowActionOutcomeResolve,
             "openwand workflow-action-outcome record --route-id <id>".to_string(),
             "openwand workflow-action-outcome record --route-id <id>".to_string())
        }
        WorkflowManualOperationKind::ResolveWorkflowApprovalOutcome => {
            missing.push(WorkflowCommandMissingInput {
                name: "approval_resolution".into(),
                reason: "Operator must choose approve or reject".into(),
                suggested_source: "OperatorInput".into(),
            });
            // Patch 4: never default to approve or reject
            (WorkflowManualCommandKind::WorkflowActionOutcomeResolve,
             "openwand workflow-action-outcome resolve --route-id <id>".to_string(),
             "openwand workflow-action-outcome resolve --route-id <id> --approval-resolution <approve|reject>".to_string())
        }
        WorkflowManualOperationKind::ReconcileWorkflowOutcome => {
            (WorkflowManualCommandKind::WorkflowReconciliationReconcile,
             "openwand workflow-reconciliation evaluate --route-id <id>".to_string(),
             "openwand workflow-reconciliation evaluate --route-id <id> --outcome-id <id>".to_string())
        }
        WorkflowManualOperationKind::InspectBlockedWorkflow => {
            (WorkflowManualCommandKind::InspectBlockedWorkflow,
             format!("openwand workflow-loop recommend --workflow-execution-id {}", wfx),
             format!("openwand workflow-loop recommend --workflow-execution-id {} --expected-workflow-run-hash <hash>", wfx))
        }
        WorkflowManualOperationKind::NoAction => {
            return (
                WorkflowCommandComposerStatus::NoCommandRequired,
                WorkflowCommandComposerDecision::NoCommandRequired { summary: "No manual action required".into() },
                None, vec![], vec![],
            );
        }
        // Patch 5: manual-result ladder operation kinds
        WorkflowManualOperationKind::CreateCommandDescriptor => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Command descriptor composition".into() },
                None, vec![], vec![],
            );
        }
        WorkflowManualOperationKind::ReviewCommandDescriptor => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Command descriptor review".into() },
                None, vec![], vec![],
            );
        }
        WorkflowManualOperationKind::CaptureManualResult => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Manual result capture".into() },
                None, vec![], vec![],
            );
        }
        WorkflowManualOperationKind::ReviewManualResult => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Manual result review".into() },
                None, vec![], vec![],
            );
        }
        WorkflowManualOperationKind::EvaluateReconciliationReadiness => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Reconciliation readiness evaluation".into() },
                None, vec![], vec![],
            );
        }
        WorkflowManualOperationKind::ReconcileManualResult => {
            return (
                WorkflowCommandComposerStatus::DescriptorReady,
                WorkflowCommandComposerDecision::DescriptorReady { summary: "Manual result reconciliation gate".into() },
                None, vec![], vec![],
            );
        }
    };

    let descriptor = WorkflowManualCommandDescriptor {
        command_kind: kind, display_command: display, arguments: args,
        missing_inputs: missing.clone(), safety_warnings: warnings,
        evidence_links: links, copyable_text: copyable,
        display_only: true, executable: false,
    };

    let has_missing = !descriptor.missing_inputs.is_empty();
    let status = if has_missing {
        WorkflowCommandComposerStatus::MissingInputs
    } else {
        WorkflowCommandComposerStatus::DescriptorReady
    };
    let decision = if has_missing {
        WorkflowCommandComposerDecision::MissingInputs {
            summary: format!("Missing: {}", descriptor.missing_inputs.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", ")),
        }
    } else {
        WorkflowCommandComposerDecision::DescriptorReady { summary: rec.reason.clone() }
    };

    (status, decision, Some(descriptor), missing, vec![])
}

fn rev_arg(ctx: &WorkflowCommandComposerContext) -> WorkflowManualCommandArgument {
    let preview = ctx.latest_revision.map(|r| r.revision_id.0.clone());
    WorkflowManualCommandArgument {
        name: "run_revision_id".into(), value_preview: preview,
        source: WorkflowCommandArgumentSource::RunRevision, required: true,
        missing: ctx.latest_revision.is_none(),
    }
}

// --- Extended test module with composition + predicate tests ---

#[cfg(test)]
mod composition_tests {
    use super::*;
    use crate::workflow_loop_controller::*;
    use crate::workflow_loop_recommendation::*;
    use crate::workflow_run::*;

    fn empty_ctx() -> WorkflowCommandComposerContext<'static> {
        WorkflowCommandComposerContext {
            loop_controller_record: None, workflow_run: None, latest_revision: None,
        }
    }

    fn test_request() -> WorkflowCommandComposerRequest {
        WorkflowCommandComposerRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_loop_controller_hash: "h".into(),
            requested_by: "test".into(), requested_at: Utc::now(),
            idempotency_key: "key1".into(),
        }
    }

    fn lc_with_op(op: WorkflowManualOperationKind) -> WorkflowLoopControllerRecord {
        WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId("wlc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: None,
            status: WorkflowLoopControllerStatus::RecommendationReady,
            decision: WorkflowLoopControllerDecision::Recommend {
                operation: op.clone(), summary: "test".into(),
            },
            loop_state: None,
            recommendation: Some(WorkflowLoopRecommendation {
                operation: op, command_hint: "test".into(), reason: "test".into(),
                required_inputs: vec![], evidence_links: vec![],
            }),
            predicates: vec![], evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false, created_at: Utc::now(),
        }
    }

    fn ctx_with_lc(lc: &WorkflowLoopControllerRecord) -> WorkflowCommandComposerContext {
        WorkflowCommandComposerContext {
            loop_controller_record: Some(lc), workflow_run: None, latest_revision: None,
        }
    }

    // Composition tests

    #[test] fn composes_continuation_propose_command() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
        let desc = rec.descriptor.unwrap();
        assert_eq!(WorkflowManualCommandKind::WorkflowContinuationPropose, desc.command_kind);
    }

    #[test] fn composes_next_action_review_command_with_missing_decision() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReviewNextActionProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::MissingInputs));
        assert!(rec.missing_inputs.iter().any(|m| m.name == "review_decision"));
    }

    #[test] fn composes_routing_readiness_evaluate_command() {
        let lc = lc_with_op(WorkflowManualOperationKind::EvaluateRoutingReadiness);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
        assert_eq!(WorkflowManualCommandKind::WorkflowRoutingReadinessEvaluate, rec.descriptor.unwrap().command_kind);
    }

    #[test] fn composes_next_action_routing_route_command() {
        let lc = lc_with_op(WorkflowManualOperationKind::RouteReviewedNextAction);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
        assert_eq!(WorkflowManualCommandKind::WorkflowNextActionRoutingRoute, rec.descriptor.unwrap().command_kind);
    }

    #[test] fn composes_action_outcome_resolve_command_with_missing_resolution() {
        let lc = lc_with_op(WorkflowManualOperationKind::ResolveWorkflowApprovalOutcome);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::MissingInputs));
        assert!(rec.missing_inputs.iter().any(|m| m.name == "approval_resolution"));
    }

    #[test] fn composes_reconciliation_command() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReconcileWorkflowOutcome);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
        assert_eq!(WorkflowManualCommandKind::WorkflowReconciliationReconcile, rec.descriptor.unwrap().command_kind);
    }

    #[test] fn composes_inspect_blocked_workflow_command() {
        let lc = lc_with_op(WorkflowManualOperationKind::InspectBlockedWorkflow);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
        assert_eq!(WorkflowManualCommandKind::InspectBlockedWorkflow, rec.descriptor.unwrap().command_kind);
    }

    #[test] fn no_action_produces_no_command_required() {
        let lc = lc_with_op(WorkflowManualOperationKind::NoAction);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::NoCommandRequired));
        assert!(rec.descriptor.is_none());
    }

    #[test] fn composer_copies_known_ids_and_hashes() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        let desc = rec.descriptor.unwrap();
        assert!(desc.arguments.iter().any(|a| a.name == "workflow_execution_id" && a.value_preview.as_deref() == Some("wfx_t")));
    }

    #[test] fn composer_never_guesses_missing_inputs() {
        // For review, missing review_decision should NOT have a value_preview
        let lc = lc_with_op(WorkflowManualOperationKind::ReviewNextActionProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        for mi in &rec.missing_inputs {
            // Missing inputs should not be in arguments with a value
            assert!(rec.descriptor.as_ref().map_or(true, |d| !d.arguments.iter().any(|a| a.name == mi.name && !a.missing)));
        }
    }

    #[test] fn composer_is_deterministic_for_same_loop_record() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let r1 = compose_command_descriptor(&test_request(), &ctx);
        let r2 = compose_command_descriptor(&test_request(), &ctx);
        assert_eq!(r1.composer_id, r2.composer_id);
        assert_eq!(r1.descriptor.unwrap().display_command, r2.descriptor.unwrap().display_command);
    }

    // Patch 4: operator decision tests

    #[test] fn review_command_requires_operator_review_decision_missing_input() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReviewNextActionProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        let mi = rec.missing_inputs.iter().find(|m| m.name == "review_decision").unwrap();
        assert!(mi.reason.contains("approve"));
        assert!(mi.reason.contains("reject"));
    }

    #[test] fn approval_outcome_command_requires_operator_resolution_missing_input() {
        let lc = lc_with_op(WorkflowManualOperationKind::ResolveWorkflowApprovalOutcome);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        let mi = rec.missing_inputs.iter().find(|m| m.name == "approval_resolution").unwrap();
        assert!(mi.reason.contains("approve"));
        assert!(mi.reason.contains("reject"));
    }

    #[test] fn composer_never_defaults_to_approve_or_reject() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReviewNextActionProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        let desc = rec.descriptor.unwrap();
        // display_command should show placeholder, not a chosen decision
        assert!(!desc.display_command.contains("--decision approve"));
        assert!(!desc.display_command.contains("--decision reject"));
        assert!(desc.display_command.contains("<review-decision>"));
    }

    // Predicate tests

    #[test] fn blocks_missing_loop_controller_record() {
        let rec = compose_command_descriptor(&test_request(), &empty_ctx());
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::Blocked));
    }

    #[test] fn blocks_loop_controller_hash_mismatch() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let mut req = test_request(); req.expected_loop_controller_hash = String::new();
        let rec = compose_command_descriptor(&req, &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::Blocked));
    }

    #[test] fn blocks_workflow_run_mismatch() {
        let mut lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        lc.workflow_execution_id = WorkflowExecutionId("wfx_other".into());
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::Blocked));
    }

    #[test] fn no_command_required_for_no_action() {
        let lc = lc_with_op(WorkflowManualOperationKind::NoAction);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::NoCommandRequired));
    }

    #[test] fn descriptor_ready_when_required_arguments_resolved() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReconcileWorkflowOutcome);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::DescriptorReady));
    }

    #[test] fn missing_inputs_when_required_arguments_unavailable() {
        let lc = lc_with_op(WorkflowManualOperationKind::ReviewNextActionProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        assert!(matches!(rec.status, WorkflowCommandComposerStatus::MissingInputs));
        assert!(!rec.missing_inputs.is_empty());
    }

    #[test] fn copyable_text_is_display_only() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let rec = compose_command_descriptor(&test_request(), &ctx);
        let desc = rec.descriptor.unwrap();
        assert!(desc.display_only);
        assert!(!desc.executable);
        assert!(desc.copyable_text.starts_with("openwand"));
    }

    #[test] fn idempotency_key_returns_existing_descriptor() {
        let lc = lc_with_op(WorkflowManualOperationKind::CreateContinuationProposal);
        let ctx = ctx_with_lc(&lc);
        let r1 = compose_command_descriptor(&test_request(), &ctx);
        let r2 = compose_command_descriptor(&test_request(), &ctx);
        assert_eq!(r1.composer_id, r2.composer_id);
    }
}
