//! Deterministic workflow proposal builder from an approved task plan.
//!
//! Converts a task plan with an approved review into a workflow proposal.
//! No LLM involvement. No tool calls. No execution.

use chrono::Utc;

use crate::plan::*;
use crate::plan_review::{TaskPlanReview, TaskPlanReviewDecision};
use crate::workflow_proposal::*;
use crate::workflow_proposal_validation::{
    compute_proposal_hash, validate_workflow_proposal, workflow_proposal_id_for,
};

#[cfg(test)]
use crate::builder::build_task_plan;
#[cfg(test)]
use crate::context::TaskPlanInput;

/// Input for building a workflow proposal from an approved task plan.
pub struct WorkflowProposalInput {
    pub task_plan: TaskPlan,
    pub latest_task_plan_review: Option<TaskPlanReview>,
    pub task_plan_hash: String,
}

/// Build a deterministic workflow proposal from an approved task plan.
///
/// Returns Err if:
/// - No review is provided
/// - Review decision is not Approved
/// - Plan hash does not match review hash
/// - Task plan status is not Reviewable
pub fn build_workflow_proposal(input: WorkflowProposalInput) -> Result<WorkflowProposal, String> {
    // Rule 1: Review must exist
    let review = input
        .latest_task_plan_review
        .as_ref()
        .ok_or("latest task plan review is required")?;

    // Rule 2: Review must be Approved
    if review.decision != TaskPlanReviewDecision::Approved {
        return Err(format!(
            "task plan review decision must be Approved, got {:?}",
            review.decision
        ));
    }

    // Rule 3: Plan hash must match review hash
    if input.task_plan_hash != review.plan_hash {
        return Err(format!(
            "task plan hash mismatch: plan has '{}', review has '{}'",
            input.task_plan_hash, review.plan_hash
        ));
    }

    // Rule 4: Plan must be Reviewable
    if input.task_plan.status != TaskPlanStatus::Reviewable {
        return Err(format!(
            "task plan status must be Reviewable, got {:?}",
            input.task_plan.status
        ));
    }

    // Rule 5: Map task plan steps to workflow stages deterministically
    let mut stages = Vec::new();
    for (idx, step) in input.task_plan.steps.iter().enumerate() {
        let stage_kind = match step.kind {
            TaskPlanStepKind::Observe => WorkflowStageKind::Observe,
            TaskPlanStepKind::Analyze => WorkflowStageKind::Analyze,
            TaskPlanStepKind::ProposeChange => WorkflowStageKind::PrepareChange,
            TaskPlanStepKind::RequestApproval => WorkflowStageKind::RequestApproval,
            TaskPlanStepKind::Verify => WorkflowStageKind::Verify,
            TaskPlanStepKind::Report => WorkflowStageKind::Report,
        };

        // Generate descriptive tool intents for each stage
        let tool_intents = generate_tool_intents_for_stage(&stage_kind, idx);

        // Copy evidence links from step to stage
        let evidence_links: Vec<WorkflowProposalEvidenceLink> = step
            .evidence_links
            .iter()
            .map(|el| WorkflowProposalEvidenceLink {
                kind: map_plan_evidence_kind(&el.kind),
                id: el.id.clone(),
                summary: el.summary.clone(),
            })
            .collect();

        stages.push(WorkflowStage {
            stage_id: format!("stage_{}", idx + 1),
            title: step.title.clone(),
            description: step.description.clone(),
            kind: stage_kind,
            order: idx as u32,
            depends_on: step.depends_on.iter().map(|d| {
                // Map step_N dependency to stage_N
                if d.starts_with("step_") {
                    d.replace("step_", "stage_")
                } else {
                    d.clone()
                }
            }).collect(),
            tool_intents,
            expected_output: step.expected_output.clone(),
            risk_level: step.risk_level.clone(),
            requires_approval_before_execution: step.requires_approval,
            evidence_links,
        });
    }

    // Rule 6: Copy required approvals as approval markers
    let approval_markers: Vec<WorkflowApprovalMarker> = input
        .task_plan
        .required_approvals
        .iter()
        .enumerate()
        .map(|(i, req)| {
            let stage_id = if req.step_id.starts_with("step_") {
                req.step_id.replace("step_", "stage_")
            } else {
                req.step_id.clone()
            };
            let required_before = if req.required_before.starts_with("step_") {
                req.required_before.replace("step_", "stage_")
            } else {
                req.required_before.clone()
            };
            WorkflowApprovalMarker {
                marker_id: format!("marker_{}", i + 1),
                stage_id,
                reason: req.reason.clone(),
                required_before,
            }
        })
        .collect();

    // Rule 7: Copy risks
    let risks: Vec<WorkflowProposalRisk> = input
        .task_plan
        .risks
        .iter()
        .map(|r| WorkflowProposalRisk {
            risk_level: r.risk_level.clone(),
            summary: r.summary.clone(),
            mitigation: r.mitigation.clone(),
        })
        .collect();

    // Rule 8: Generate abort/rollback notes
    let abort_rollback_notes = generate_abort_rollback_notes(&stages);

    // Rule 9: Copy proposal-level evidence links from plan
    let mut evidence_links: Vec<WorkflowProposalEvidenceLink> = vec![
        WorkflowProposalEvidenceLink {
            kind: WorkflowProposalEvidenceKind::TaskPlan,
            id: input.task_plan.plan_id.0.clone(),
            summary: format!("Source task plan: {}", input.task_plan.title),
        },
        WorkflowProposalEvidenceLink {
            kind: WorkflowProposalEvidenceKind::TaskPlanReview,
            id: review.review_id.0.clone(),
            summary: format!("Approved review by {}", review.reviewer),
        },
    ];

    for el in &input.task_plan.evidence_links {
        evidence_links.push(WorkflowProposalEvidenceLink {
            kind: map_plan_evidence_kind(&el.kind),
            id: el.id.clone(),
            summary: el.summary.clone(),
        });
    }

    let proposal_hash = compute_proposal_hash(&stages, &risks);
    let proposal_id = workflow_proposal_id_for(
        input.task_plan.plan_id.0.as_str(),
        &input.task_plan.title,
        stages.len(),
        &proposal_hash,
    );

    let proposal = WorkflowProposal {
        proposal_id,
        source_task_plan_id: input.task_plan.plan_id.clone(),
        source_task_plan_review_id: review.review_id.clone(),
        source_task_plan_hash: input.task_plan.plan_hash.clone(),
        title: format!("Workflow: {}", input.task_plan.title),
        status: WorkflowProposalStatus::Reviewable,
        stages,
        required_approvals: approval_markers,
        risks,
        abort_rollback_notes,
        evidence_links,
        proposal_hash,
        created_at: Utc::now(),
    };

    // Validate the built proposal
    validate_workflow_proposal(&proposal).map_err(|e| e.join(", "))?;

    Ok(proposal)
}

/// Generate descriptive tool intents for a stage kind.
fn generate_tool_intents_for_stage(kind: &WorkflowStageKind, idx: usize) -> Vec<WorkflowToolIntent> {
    match kind {
        WorkflowStageKind::Observe => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "context-observation".into(),
            purpose: "Gather relevant context for analysis".into(),
            expected_input_summary: "Project state, files, current conditions".into(),
            expected_output_summary: "Structured context summary".into(),
            requires_policy_gate: false,
        }],
        WorkflowStageKind::Analyze => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "text-analysis".into(),
            purpose: "Analyze observed context against intent".into(),
            expected_input_summary: "Context summary, goals, constraints".into(),
            expected_output_summary: "Gap analysis and recommendations".into(),
            requires_policy_gate: false,
        }],
        WorkflowStageKind::PrepareChange => vec![
            WorkflowToolIntent {
                intent_id: format!("intent_{}_1", idx + 1),
                capability: "file-observation".into(),
                purpose: "Identify files and locations for changes".into(),
                expected_input_summary: "Analysis output, target locations".into(),
                expected_output_summary: "Change plan with file locations".into(),
                requires_policy_gate: false,
            },
            WorkflowToolIntent {
                intent_id: format!("intent_{}_2", idx + 1),
                capability: "change-preparation".into(),
                purpose: "Prepare specific change descriptions".into(),
                expected_input_summary: "Change plan".into(),
                expected_output_summary: "Detailed change descriptions".into(),
                requires_policy_gate: true,
            },
        ],
        WorkflowStageKind::RequestApproval => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "review-presentation".into(),
            purpose: "Present proposed changes for human review".into(),
            expected_input_summary: "Change descriptions, risk analysis".into(),
            expected_output_summary: "Human approval or feedback".into(),
            requires_policy_gate: false,
        }],
        WorkflowStageKind::ApplyChange => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "change-application".into(),
            purpose: "Apply approved changes".into(),
            expected_input_summary: "Approved change descriptions".into(),
            expected_output_summary: "Change application result".into(),
            requires_policy_gate: true,
        }],
        WorkflowStageKind::Verify => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "outcome-verification".into(),
            purpose: "Verify changes achieved intended outcome".into(),
            expected_input_summary: "Change results, expected outcome".into(),
            expected_output_summary: "Verification result".into(),
            requires_policy_gate: false,
        }],
        WorkflowStageKind::Report => vec![WorkflowToolIntent {
            intent_id: format!("intent_{}_1", idx + 1),
            capability: "result-reporting".into(),
            purpose: "Report outcome and evidence".into(),
            expected_input_summary: "Verification result".into(),
            expected_output_summary: "Outcome report".into(),
            requires_policy_gate: false,
        }],
    }
}

/// Generate abort/rollback notes for proposal stages.
fn generate_abort_rollback_notes(stages: &[WorkflowStage]) -> Vec<WorkflowAbortRollbackNote> {
    let mut notes = Vec::new();

    // Always add a general abort note
    notes.push(WorkflowAbortRollbackNote {
        stage_id: None,
        summary: "Workflow can be aborted before any stage executes. No partial state to recover."
            .into(),
        recovery_hint: "Since this is a proposal, no execution has occurred. Simply discard."
            .into(),
    });

    // Add stage-specific notes for high-risk stages
    for stage in stages {
        if stage.risk_level == "medium" || stage.risk_level == "high" {
            notes.push(WorkflowAbortRollbackNote {
                stage_id: Some(stage.stage_id.clone()),
                summary: format!(
                    "Stage '{}' ({}) carries {} risk. Abort before execution if uncertain.",
                    stage.title, stage.stage_id, stage.risk_level
                ),
                recovery_hint: "Review stage output before proceeding to next stage.".into(),
            });
        }

        if stage.requires_approval_before_execution {
            notes.push(WorkflowAbortRollbackNote {
                stage_id: Some(stage.stage_id.clone()),
                summary: format!(
                    "Stage '{}' requires approval. Withhold approval to abort.",
                    stage.stage_id
                ),
                recovery_hint: "No recovery needed — stage has not executed.".into(),
            });
        }
    }

    notes
}

/// Map task plan evidence kind to proposal evidence kind.
fn map_plan_evidence_kind(kind: &TaskPlanEvidenceKind) -> WorkflowProposalEvidenceKind {
    match kind {
        TaskPlanEvidenceKind::Goal => WorkflowProposalEvidenceKind::Goal,
        TaskPlanEvidenceKind::Skill => WorkflowProposalEvidenceKind::Skill,
        TaskPlanEvidenceKind::TraceEvent => WorkflowProposalEvidenceKind::TraceEvent,
        TaskPlanEvidenceKind::MemoryClaim => WorkflowProposalEvidenceKind::MemoryClaim,
        TaskPlanEvidenceKind::GovernanceRecord => WorkflowProposalEvidenceKind::GovernanceRecord,
        TaskPlanEvidenceKind::UserIntent => WorkflowProposalEvidenceKind::UserIntent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TaskPlanInput;
    use crate::plan_review::task_review_id_for;
    use chrono::Utc;

    fn test_task_plan() -> TaskPlan {
        build_task_plan(&TaskPlanInput {
            user_intent: "Refactor session module".into(),
            skill_context: vec!["rust-refactor: Refactor Rust code".into()],
            goal_context: vec!["ship-product: Ship OpenWand".into()],
            memory_summaries: vec!["memory".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell execution".into()],
        })
        .unwrap()
    }

    fn approved_review(plan: &TaskPlan) -> TaskPlanReview {
        let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "test-reviewer".into(),
            rationale: "Looks good".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    fn valid_input() -> WorkflowProposalInput {
        let plan = test_task_plan();
        let review = approved_review(&plan);
        let hash = plan.plan_hash.clone();
        WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(review),
            task_plan_hash: hash,
        }
    }

    #[test]
    fn builder_requires_approved_task_plan_review() {
        let result = build_workflow_proposal(WorkflowProposalInput {
            latest_task_plan_review: None,
            ..valid_input()
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("review is required"));
    }

    #[test]
    fn builder_blocks_rejected_task_plan_review() {
        let plan = test_task_plan();
        let review_id =
            task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Rejected, "Bad");
        let review = TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Rejected,
            reviewer: "r".into(),
            rationale: "Bad".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let input = WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(review),
            task_plan_hash: test_task_plan().plan_hash.clone(),
        };
        let result = build_workflow_proposal(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Approved"));
    }

    #[test]
    fn builder_blocks_requested_changes_task_plan_review() {
        let plan = test_task_plan();
        let review_id = task_review_id_for(
            &plan.plan_id,
            &TaskPlanReviewDecision::ChangesRequested,
            "Fix",
        );
        let review = TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::ChangesRequested,
            reviewer: "r".into(),
            rationale: "Fix".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let input = WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(review),
            task_plan_hash: test_task_plan().plan_hash.clone(),
        };
        let result = build_workflow_proposal(input);
        assert!(result.is_err());
    }

    #[test]
    fn builder_blocks_plan_hash_mismatch() {
        let plan = test_task_plan();
        let review = approved_review(&plan);
        let input = WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(review),
            task_plan_hash: "wrong_hash".into(),
        };
        let result = build_workflow_proposal(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
    }

    #[test]
    fn builder_maps_task_plan_steps_to_workflow_stages() {
        let proposal = build_workflow_proposal(valid_input()).unwrap();
        let plan_steps = test_task_plan().steps;
        assert_eq!(plan_steps.len(), proposal.stages.len());

        // Check mapping
        for (step, stage) in plan_steps.iter().zip(proposal.stages.iter()) {
            match step.kind {
                TaskPlanStepKind::Observe => assert_eq!(WorkflowStageKind::Observe, stage.kind),
                TaskPlanStepKind::Analyze => assert_eq!(WorkflowStageKind::Analyze, stage.kind),
                TaskPlanStepKind::ProposeChange => {
                    assert_eq!(WorkflowStageKind::PrepareChange, stage.kind)
                }
                TaskPlanStepKind::RequestApproval => {
                    assert_eq!(WorkflowStageKind::RequestApproval, stage.kind)
                }
                TaskPlanStepKind::Verify => assert_eq!(WorkflowStageKind::Verify, stage.kind),
                TaskPlanStepKind::Report => assert_eq!(WorkflowStageKind::Report, stage.kind),
            }
        }
    }

    #[test]
    fn builder_copies_task_plan_evidence_links() {
        let proposal = build_workflow_proposal(valid_input()).unwrap();
        // Should have TaskPlan and TaskPlanReview evidence at minimum
        assert!(proposal
            .evidence_links
            .iter()
            .any(|e| matches!(e.kind, WorkflowProposalEvidenceKind::TaskPlan)));
        assert!(proposal
            .evidence_links
            .iter()
            .any(|e| matches!(e.kind, WorkflowProposalEvidenceKind::TaskPlanReview)));
        // Plus goal and skill from plan
        assert!(proposal
            .evidence_links
            .iter()
            .any(|e| matches!(e.kind, WorkflowProposalEvidenceKind::Goal)));
        assert!(proposal
            .evidence_links
            .iter()
            .any(|e| matches!(e.kind, WorkflowProposalEvidenceKind::Skill)));
    }

    #[test]
    fn builder_copies_required_approvals_as_markers() {
        let proposal = build_workflow_proposal(valid_input()).unwrap();
        // Plan has policy_constraints → step_3 requires_approval → marker
        assert!(!proposal.required_approvals.is_empty());
        assert!(proposal
            .required_approvals
            .iter()
            .any(|m| m.stage_id.contains("stage_")));
    }

    #[test]
    fn builder_generates_abort_rollback_notes() {
        let proposal = build_workflow_proposal(valid_input()).unwrap();
        assert!(!proposal.abort_rollback_notes.is_empty());
        // General note
        assert!(proposal
            .abort_rollback_notes
            .iter()
            .any(|n| n.stage_id.is_none()));
        // Stage-specific notes for high-risk or approval-required stages
        assert!(proposal
            .abort_rollback_notes
            .iter()
            .any(|n| n.stage_id.is_some()));
    }

    #[test]
    fn builder_orders_stages_deterministically() {
        let proposal1 = build_workflow_proposal(valid_input()).unwrap();
        let proposal2 = build_workflow_proposal(valid_input()).unwrap();
        let ids1: Vec<_> = proposal1.stages.iter().map(|s| s.stage_id.clone()).collect();
        let ids2: Vec<_> = proposal2.stages.iter().map(|s| s.stage_id.clone()).collect();
        assert_eq!(ids1, ids2);
    }

    #[test]
    fn builder_tool_intents_are_descriptive_only() {
        let proposal = build_workflow_proposal(valid_input()).unwrap();
        for stage in &proposal.stages {
            for intent in &stage.tool_intents {
                assert!(
                    is_valid_capability_category(&intent.capability).is_ok(),
                    "capability '{}' should be valid descriptive category",
                    intent.capability
                );
            }
        }
    }
}
