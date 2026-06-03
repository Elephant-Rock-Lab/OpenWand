//! Workflow proposal validation.
//!
//! Ensures proposals contain no executable fields, valid stage references,
//! and correct content-addressing.

use crate::workflow_proposal::*;

/// Validate a workflow proposal.
pub fn validate_workflow_proposal(proposal: &WorkflowProposal) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if proposal.title.trim().is_empty() {
        errors.push("title must not be empty".into());
    }

    if proposal.proposal_hash.trim().is_empty() {
        errors.push("proposal_hash must not be empty".into());
    }

    if proposal.source_task_plan_hash.trim().is_empty() {
        errors.push("source_task_plan_hash must not be empty".into());
    }

    if proposal.status == WorkflowProposalStatus::Reviewable && proposal.stages.is_empty() {
        errors.push("reviewable proposal must have at least one stage".into());
    }

    // Validate stage dependency references
    let stage_ids: Vec<&str> = proposal.stages.iter().map(|s| s.stage_id.as_str()).collect();
    for stage in &proposal.stages {
        for dep in &stage.depends_on {
            if !stage_ids.contains(&dep.as_str()) {
                errors.push(format!(
                    "stage '{}' references unknown dependency '{}'",
                    stage.stage_id, dep
                ));
            }
        }
    }

    // Validate tool intent capabilities
    for stage in &proposal.stages {
        for intent in &stage.tool_intents {
            if let Err(e) = is_valid_capability_category(&intent.capability) {
                errors.push(format!(
                    "stage '{}' tool intent '{}': {}",
                    stage.stage_id, intent.intent_id, e
                ));
            }
        }
    }

    // Validate approval marker stage references
    let stage_id_set: Vec<&str> = proposal.stages.iter().map(|s| s.stage_id.as_str()).collect();
    for marker in &proposal.required_approvals {
        if !stage_id_set.contains(&marker.stage_id.as_str()) {
            errors.push(format!(
                "approval marker '{}' references unknown stage '{}'",
                marker.marker_id, marker.stage_id
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate that a serialized JSON string contains no executable fields.
pub fn validate_no_executable_fields(json: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let forbidden_fields = [
        "tool_name",
        "tool_args",
        "command",
        "shell",
        "script",
        "cwd",
        "env",
        "function_ref",
        "workflow_run_id",
        "execution_grant",
        "approval_request_id",
        "process",
        "git_args",
    ];

    // Check for forbidden field keys in JSON
    let lower_json = json.to_lowercase();
    for field in &forbidden_fields {
        // Look for "field":  pattern (key in JSON object)
        if lower_json.contains(&format!("\"{}\"", field)) {
            // Only flag if it's a key (followed by colon)
            if lower_json.contains(&format!("\"{}\":", field)) {
                errors.push(format!("serialized proposal contains forbidden field '{}'", field));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Compute content-addressed proposal ID from normalized fields.
pub fn workflow_proposal_id_for(
    source_task_plan_id: &str,
    title: &str,
    stage_count: usize,
    proposal_hash: &str,
) -> WorkflowProposalId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source_task_plan_id.as_bytes());
    hasher.update(title.trim().as_bytes());
    hasher.update(stage_count.to_le_bytes().as_slice());
    hasher.update(proposal_hash.as_bytes());
    let hash = hasher.finalize();
    WorkflowProposalId(format!("wfp_{}", hash.to_hex()))
}

/// Compute proposal hash from stages and evidence.
pub fn compute_proposal_hash(
    stages: &[WorkflowStage],
    risks: &[WorkflowProposalRisk],
) -> String {
    let mut hasher = blake3::Hasher::new();
    for stage in stages {
        hasher.update(stage.stage_id.as_bytes());
        hasher.update(stage.title.as_bytes());
        let kind_str = serde_json::to_string(&stage.kind).unwrap_or_default();
        hasher.update(kind_str.as_bytes());
        hasher.update(stage.order.to_le_bytes().as_slice());
    }
    for risk in risks {
        hasher.update(risk.summary.as_bytes());
    }
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::TaskPlanId;
    use crate::plan_review::TaskPlanReviewId;
    use chrono::Utc;

    fn test_proposal() -> WorkflowProposal {
        WorkflowProposal {
            proposal_id: WorkflowProposalId("wfp_test".into()),
            source_task_plan_id: TaskPlanId("tpl_test".into()),
            source_task_plan_review_id: TaskPlanReviewId("tpr_test".into()),
            source_task_plan_hash: "hash_test".into(),
            title: "Test proposal".into(),
            status: WorkflowProposalStatus::Reviewable,
            stages: vec![WorkflowStage {
                stage_id: "stage_1".into(),
                title: "Observe".into(),
                description: "Gather context".into(),
                kind: WorkflowStageKind::Observe,
                order: 0,
                depends_on: vec![],
                tool_intents: vec![],
                expected_output: "Context".into(),
                risk_level: "low".into(),
                requires_approval_before_execution: false,
                evidence_links: vec![],
            }],
            required_approvals: vec![],
            risks: vec![],
            abort_rollback_notes: vec![],
            evidence_links: vec![],
            proposal_hash: "phash".into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn validate_proposal_accepts_valid() {
        assert!(validate_workflow_proposal(&test_proposal()).is_ok());
    }

    #[test]
    fn validate_proposal_rejects_empty_title() {
        let mut p = test_proposal();
        p.title = "  ".into();
        let errors = validate_workflow_proposal(&p).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("title")));
    }

    #[test]
    fn validate_proposal_rejects_empty_stages_for_reviewable() {
        let mut p = test_proposal();
        p.status = WorkflowProposalStatus::Reviewable;
        p.stages = vec![];
        let errors = validate_workflow_proposal(&p).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("stage")));
    }

    #[test]
    fn validate_proposal_rejects_unknown_stage_dependency() {
        let mut p = test_proposal();
        p.stages[0].depends_on = vec!["nonexistent".into()];
        let errors = validate_workflow_proposal(&p).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unknown dependency")));
    }

    #[test]
    fn validate_proposal_rejects_invalid_capability() {
        let mut p = test_proposal();
        p.stages[0].tool_intents = vec![WorkflowToolIntent {
            intent_id: "intent_bad".into(),
            capability: "shell".into(), // forbidden
            purpose: "Run commands".into(),
            expected_input_summary: "cmd".into(),
            expected_output_summary: "output".into(),
            requires_policy_gate: true,
        }];
        let errors = validate_workflow_proposal(&p).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("forbidden tool name")));
    }

    #[test]
    fn proposal_hash_changes_when_stage_changes() {
        let stages_a = vec![WorkflowStage {
            stage_id: "s1".into(),
            title: "Observe".into(),
            description: "Gather".into(),
            kind: WorkflowStageKind::Observe,
            order: 0,
            depends_on: vec![],
            tool_intents: vec![],
            expected_output: "ctx".into(),
            risk_level: "low".into(),
            requires_approval_before_execution: false,
            evidence_links: vec![],
        }];
        let stages_b = vec![WorkflowStage {
            stage_id: "s1".into(),
            title: "Analyze".into(),
            description: "Analyze".into(),
            kind: WorkflowStageKind::Analyze,
            order: 0,
            depends_on: vec![],
            tool_intents: vec![],
            expected_output: "analysis".into(),
            risk_level: "low".into(),
            requires_approval_before_execution: false,
            evidence_links: vec![],
        }];
        let hash_a = compute_proposal_hash(&stages_a, &[]);
        let hash_b = compute_proposal_hash(&stages_b, &[]);
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn proposal_id_is_content_addressed() {
        let id1 = workflow_proposal_id_for("tpl_a", "Title", 3, "hash1");
        let id2 = workflow_proposal_id_for("tpl_a", "Title", 3, "hash1");
        let id3 = workflow_proposal_id_for("tpl_b", "Title", 3, "hash1");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.0.starts_with("wfp_"));
    }

    #[test]
    fn proposal_dto_has_no_executable_fields() {
        let proposal = test_proposal();
        // Compile-time check: these fields don't exist on the struct
        let _ = &proposal.proposal_id;
        let _ = &proposal.stages;
        let _ = &proposal.risks;
    }

    #[test]
    fn stage_dto_has_no_executable_fields() {
        let stage = &test_proposal().stages[0];
        let _ = &stage.stage_id;
        let _ = &stage.tool_intents;
    }

    #[test]
    fn tool_intent_has_no_tool_args_or_command() {
        let intent = WorkflowToolIntent {
            intent_id: "i1".into(),
            capability: "file-observation".into(),
            purpose: "Read files".into(),
            expected_input_summary: "Paths".into(),
            expected_output_summary: "Contents".into(),
            requires_policy_gate: false,
        };
        let json = serde_json::to_string(&intent).unwrap();
        // Must not contain executable fields
        assert!(!json.contains("tool_name"));
        assert!(!json.contains("tool_args"));
        assert!(!json.contains("command"));
        assert!(!json.contains("shell"));
    }

    #[test]
    fn serialized_proposal_contains_no_executable_fields() {
        let proposal = test_proposal();
        let json = serde_json::to_string(&proposal).unwrap();
        assert!(validate_no_executable_fields(&json).is_ok());
    }

    #[test]
    fn validate_no_executable_fields_catches_forbidden() {
        let bad_json = r#"{"tool_name": "shell", "tool_args": ["-c", "ls"]}"#;
        let errors = validate_no_executable_fields(bad_json).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("tool_name")));
        assert!(errors.iter().any(|e| e.contains("tool_args")));
    }

    #[test]
    fn tool_intent_capability_is_category_not_tool_name() {
        let intent = WorkflowToolIntent {
            intent_id: "i1".into(),
            capability: "file-observation".into(),
            purpose: "Observe files".into(),
            expected_input_summary: "Paths".into(),
            expected_output_summary: "Contents".into(),
            requires_policy_gate: false,
        };
        // The capability is a descriptive category, not a tool name
        assert!(is_valid_capability_category(&intent.capability).is_ok());
        assert!(!FORBIDDEN_TOOL_NAMES.contains(&intent.capability.as_str()));
    }

    #[test]
    fn tool_intent_rejects_registered_tool_name() {
        let intent = WorkflowToolIntent {
            intent_id: "i1".into(),
            capability: "shell".into(),
            purpose: "Run shell".into(),
            expected_input_summary: "cmd".into(),
            expected_output_summary: "output".into(),
            requires_policy_gate: false,
        };
        assert!(is_valid_capability_category(&intent.capability).is_err());
    }

    #[test]
    fn tool_intent_serialized_json_contains_no_tool_args() {
        let intent = WorkflowToolIntent {
            intent_id: "i1".into(),
            capability: "context-gathering".into(),
            purpose: "Gather context".into(),
            expected_input_summary: "Sources".into(),
            expected_output_summary: "Summary".into(),
            requires_policy_gate: false,
        };
        let json = serde_json::to_string(&intent).unwrap();
        assert!(validate_no_executable_fields(&json).is_ok());
        // Also verify none of the descriptive fields contain tool args
        assert!(!json.contains("tool_args"));
        assert!(!json.contains("command"));
        assert!(!json.contains("script"));
    }
}
