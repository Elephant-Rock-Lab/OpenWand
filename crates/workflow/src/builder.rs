//! Deterministic plan builder.
//!
//! Creates structured non-executing plans from supplied inputs.
//! No LLM involvement. No tool calls. No execution.

use chrono::Utc;

use crate::context::*;
use crate::plan::*;
use crate::validation::{compute_plan_hash, task_plan_id_for};

/// Build a deterministic task plan from input.
///
/// Returns Err if user_intent is empty.
/// Produces Reviewable status when steps are generated, Draft otherwise.
pub fn build_task_plan(input: &TaskPlanInput) -> Result<TaskPlan, String> {
    if input.user_intent.trim().is_empty() {
        return Err("user_intent must not be empty".into());
    }

    let mut evidence_links = Vec::new();
    let mut skill_ids = Vec::new();
    let mut goal_ids = Vec::new();
    let mut assumptions = Vec::new();

    // Goal context → evidence links
    for goal_summary in &input.goal_context {
        let parts: Vec<&str> = goal_summary.splitn(2, ':').collect();
        let (id, desc) = if parts.len() == 2 {
            (parts[0].trim(), parts[1].trim())
        } else {
            ("unknown", goal_summary.as_str())
        };
        goal_ids.push(id.to_string());
        evidence_links.push(goal_evidence(id, desc));
    }

    // Skill context → evidence links
    for skill_summary in &input.skill_context {
        let parts: Vec<&str> = skill_summary.splitn(2, ':').collect();
        let (id, desc) = if parts.len() == 2 {
            (parts[0].trim(), parts[1].trim())
        } else {
            ("unknown", skill_summary.as_str())
        };
        skill_ids.push(id.to_string());
        evidence_links.push(skill_evidence(id, desc));
    }

    // User intent → evidence
    evidence_links.push(user_intent_evidence(&input.user_intent));

    // Missing context → assumptions
    if input.memory_summaries.is_empty() {
        assumptions.push(TaskPlanAssumption {
            text: "No memory context available for this plan.".into(),
            evidence_links: vec![],
        });
    }
    if input.trace_summaries.is_empty() {
        assumptions.push(TaskPlanAssumption {
            text: "No trace context available for this plan.".into(),
            evidence_links: vec![],
        });
    }

    // Build steps deterministically
    let mut steps = Vec::new();

    // Step 1: Observe
    steps.push(TaskPlanStep {
        step_id: "step_1".into(),
        title: "Observe current state".into(),
        description: format!("Gather relevant context for: {}", input.user_intent),
        kind: TaskPlanStepKind::Observe,
        depends_on: vec![],
        expected_output: "Current state summary".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    });

    // Step 2: Analyze
    steps.push(TaskPlanStep {
        step_id: "step_2".into(),
        title: "Analyze findings".into(),
        description: "Analyze observed context against intent and goals.".into(),
        kind: TaskPlanStepKind::Analyze,
        depends_on: vec!["step_1".into()],
        expected_output: "Analysis of current state vs intent".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: evidence_links.clone(),
    });

    // Step 3: Propose changes (requires approval if any policy constraints exist)
    let requires_approval = !input.policy_constraints.is_empty();
    steps.push(TaskPlanStep {
        step_id: "step_3".into(),
        title: "Propose changes".into(),
        description: "Propose specific changes to achieve the stated intent.".into(),
        kind: TaskPlanStepKind::ProposeChange,
        depends_on: vec!["step_2".into()],
        expected_output: "Change proposal".into(),
        risk_level: if requires_approval { "medium" } else { "low" }.into(),
        requires_approval,
        evidence_links: vec![],
    });

    // Step 4: Request approval (if needed)
    if requires_approval {
        steps.push(TaskPlanStep {
            step_id: "step_4".into(),
            title: "Request approval".into(),
            description: "Present proposed changes for human review.".into(),
            kind: TaskPlanStepKind::RequestApproval,
            depends_on: vec!["step_3".into()],
            expected_output: "Human approval or feedback".into(),
            risk_level: "low".into(),
            requires_approval: false,
            evidence_links: vec![],
        });
    }

    // Step 5: Verify
    steps.push(TaskPlanStep {
        step_id: "step_5".into(),
        title: "Verify results".into(),
        description: "Verify that changes achieved the intended outcome.".into(),
        kind: TaskPlanStepKind::Verify,
        depends_on: if requires_approval {
            vec!["step_4".into()]
        } else {
            vec!["step_3".into()]
        },
        expected_output: "Verification of outcome".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    });

    // Step 6: Report
    steps.push(TaskPlanStep {
        step_id: "step_6".into(),
        title: "Report outcome".into(),
        description: "Report what was done and the outcome.".into(),
        kind: TaskPlanStepKind::Report,
        depends_on: vec!["step_5".into()],
        expected_output: "Outcome report".into(),
        risk_level: "low".into(),
        requires_approval: false,
        evidence_links: vec![],
    });

    // Required approvals
    let required_approvals: Vec<TaskPlanApprovalRequirement> = steps
        .iter()
        .filter(|s| s.requires_approval)
        .map(|s| TaskPlanApprovalRequirement {
            step_id: s.step_id.clone(),
            reason: "Policy constraints require human review.".into(),
            required_before: s.step_id.clone(),
        })
        .collect();

    // Risks
    let mut risks = Vec::new();
    if !input.policy_constraints.is_empty() {
        risks.push(TaskPlanRisk {
            risk_level: "medium".into(),
            summary: "Policy constraints apply to this intent.".into(),
            mitigation: "Human review required before changes.".into(),
        });
    }

    let plan_hash = compute_plan_hash(&steps, &input.policy_constraints);
    let plan_id = task_plan_id_for(
        &input.user_intent,
        &input.user_intent,
        steps.len(),
        &goal_ids,
        &skill_ids,
    );

    let title = if input.user_intent.len() > 80 {
        format!("{}...", &input.user_intent[..77])
    } else {
        input.user_intent.clone()
    };

    Ok(TaskPlan {
        plan_id,
        title,
        user_intent: input.user_intent.clone(),
        status: TaskPlanStatus::Reviewable,
        steps,
        assumptions,
        risks,
        required_approvals,
        evidence_links,
        skill_context_ids: skill_ids,
        goal_context_ids: goal_ids,
        policy_constraints: input.policy_constraints.clone(),
        plan_hash,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_input() -> TaskPlanInput {
        TaskPlanInput {
            user_intent: "Refactor the session module".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["some memory".into()],
            trace_summaries: vec!["some trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec![],
        }
    }

    #[test]
    fn builder_creates_plan_from_user_intent() {
        let plan = build_task_plan(&basic_input()).unwrap();
        assert_eq!("Refactor the session module", plan.title);
        assert_eq!(TaskPlanStatus::Reviewable, plan.status);
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn builder_rejects_empty_intent() {
        let input = TaskPlanInput {
            user_intent: "   ".into(),
            ..basic_input()
        };
        assert!(build_task_plan(&input).is_err());
    }

    #[test]
    fn builder_includes_active_goals_as_evidence() {
        let input = TaskPlanInput {
            goal_context: vec!["ship-product: Ship OpenWand".into()],
            ..basic_input()
        };
        let plan = build_task_plan(&input).unwrap();
        assert!(plan.evidence_links.iter().any(|e| matches!(e.kind, TaskPlanEvidenceKind::Goal)));
        assert!(plan.goal_context_ids.contains(&"ship-product".to_string()));
    }

    #[test]
    fn builder_includes_enabled_skills_as_evidence() {
        let input = TaskPlanInput {
            skill_context: vec!["rust-test-triage: Triage tests".into()],
            ..basic_input()
        };
        let plan = build_task_plan(&input).unwrap();
        assert!(plan.evidence_links.iter().any(|e| matches!(e.kind, TaskPlanEvidenceKind::Skill)));
        assert!(plan.skill_context_ids.contains(&"rust-test-triage".to_string()));
    }

    #[test]
    fn builder_copies_policy_constraints() {
        let input = TaskPlanInput {
            policy_constraints: vec!["No direct shell execution".into()],
            ..basic_input()
        };
        let plan = build_task_plan(&input).unwrap();
        assert!(plan.policy_constraints.contains(&"No direct shell execution".to_string()));
    }

    #[test]
    fn builder_outputs_non_executing_step_kinds_only() {
        let plan = build_task_plan(&basic_input()).unwrap();
        for step in &plan.steps {
            match step.kind {
                TaskPlanStepKind::Observe
                | TaskPlanStepKind::Analyze
                | TaskPlanStepKind::ProposeChange
                | TaskPlanStepKind::RequestApproval
                | TaskPlanStepKind::Verify
                | TaskPlanStepKind::Report => {}
            }
        }
    }

    #[test]
    fn builder_adds_assumption_for_missing_context() {
        let input = TaskPlanInput {
            memory_summaries: vec![],
            trace_summaries: vec![],
            ..basic_input()
        };
        let plan = build_task_plan(&input).unwrap();
        assert!(plan.assumptions.iter().any(|a| a.text.contains("No memory context")));
        assert!(plan.assumptions.iter().any(|a| a.text.contains("No trace context")));
    }

    #[test]
    fn builder_orders_steps_deterministically() {
        let plan1 = build_task_plan(&basic_input()).unwrap();
        let plan2 = build_task_plan(&basic_input()).unwrap();
        let ids1: Vec<_> = plan1.steps.iter().map(|s| s.step_id.clone()).collect();
        let ids2: Vec<_> = plan2.steps.iter().map(|s| s.step_id.clone()).collect();
        assert_eq!(ids1, ids2);
    }
}
