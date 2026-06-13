//! Tests for live workflow proposal + readiness wiring (Wave 84A).
//!
//! Proves the loader functions correctly resolve proposal, review, and readiness
//! from a workflow run's IDs. Tests the live data path from persistence to UI state.

#[cfg(test)]
mod tests {
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use openwand_workflow::workflow_readiness::{
        WorkflowReadinessRequest, WorkflowEnvironmentSnapshot,
    };
    use openwand_workflow::workflow_run::{WorkflowRunRecord, WorkflowRunStatus, WorkflowExecutionDecision};
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use chrono::Utc;

    fn test_store_root() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn build_full_chain(
        store_root: &std::path::Path,
    ) -> WorkflowExecutionId {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "84A live wiring test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();

        let plan_review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let plan_review = TaskPlanReview {
            review_id: plan_review_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };

        let proposal = build_workflow_proposal(WorkflowProposalInput {
            task_plan: plan.clone(),
            latest_task_plan_review: Some(plan_review.clone()),
            task_plan_hash: plan.plan_hash.clone(),
        }).unwrap();

        let proposal_review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &WorkflowProposalReviewDecision::Approved,
            "Good",
        );
        let proposal_review = WorkflowProposalReview {
            review_id: proposal_review_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: WorkflowProposalReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "Good".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };

        let readiness_request = WorkflowReadinessRequest {
            proposal_id: proposal.proposal_id.clone(),
            review_id: proposal_review.review_id.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "tester".into(),
            requested_at: Utc::now(),
            idempotency_key: "key84a".into(),
        };

        let context = WorkflowReadinessContext {
            proposal: Some(proposal.clone()),
            review: Some(proposal_review.clone()),
            latest_review_for_proposal: None,
            source_task_plan: Some(plan.clone()),
            source_task_plan_review: Some(plan_review.clone()),
            latest_source_task_plan_review: Some(plan_review),
            environment: WorkflowEnvironmentSnapshot {
                workspace_observed: true,
                provider_config_available: true,
                session_runtime_available: true,
                tool_manifest_available: true,
                policy_context_available: true,
                notes: vec![],
            },
            existing_readiness_records: vec![],
        };

        let readiness = evaluate_workflow_readiness(&readiness_request, &context);

        // Save all records
        openwand_app::workflow_proposal::save_workflow_proposal(store_root, &proposal).unwrap();
        openwand_app::workflow_proposal::save_proposal_review(store_root, &proposal_review).unwrap();
        openwand_app::workflow_readiness::save_workflow_readiness(store_root, &readiness).unwrap();

        // Create a minimal workflow run record
        let wfx_id = WorkflowExecutionId("wfx_84a_test".into());
        let run = WorkflowRunRecord {
            execution_id: wfx_id.clone(),
            readiness_id: readiness.readiness_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            proposal_review_id: proposal_review.review_id.clone(),
            source_task_plan_id: plan.plan_id.clone(),
            status: WorkflowRunStatus::Running,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![],
            run_snapshot: openwand_workflow::workflow_run::WorkflowRunSnapshot {
                readiness_id: readiness.readiness_id.0.clone(),
                proposal_id: proposal.proposal_id.0.clone(),
                proposal_hash: proposal.proposal_hash.clone(),
                source_task_plan_hash: proposal.source_task_plan_hash.clone(),
                readiness_status_at_execution: format!("{:?}", readiness.status).to_lowercase(),
                proposal_review_decision_at_execution: format!("{:?}", proposal_review.decision).to_lowercase(),
            },
            stages: vec![],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: openwand_workflow::workflow_run::WorkflowAbortSnapshot {
                abort_notes_available: false,
                rollback_notes_available: false,
                recovery_notes: vec![],
            },
            created_at: Utc::now(),
            completed_at: None,
        };

        let run_dir = store_root.join("workflow_runs").join("records");
        std::fs::create_dir_all(&run_dir).unwrap();
        let run_path = run_dir.join(format!("{}.json", run.execution_id.0));
        std::fs::write(&run_path, serde_json::to_string_pretty(&run).unwrap()).unwrap();

        wfx_id
    }

    #[test]
    fn proposal_by_workflow_run_loads_proposal_and_review() {
        let store = test_store_root();
        let wfx_id = build_full_chain(&store);

        let result = openwand_app::workflow_proposal::proposal_and_review_by_workflow_run(&store, &wfx_id.0);
        assert!(result.is_ok());
        let (proposal, review) = result.unwrap().unwrap();
        assert!(!proposal.stages.is_empty());
        assert!(review.is_some());
        assert_eq!(review.unwrap().decision, WorkflowProposalReviewDecision::Approved);
    }

    #[test]
    fn readiness_by_workflow_run_loads_readiness_record() {
        let store = test_store_root();
        let wfx_id = build_full_chain(&store);

        let result = openwand_app::workflow_readiness::readiness_by_workflow_run(&store, &wfx_id.0);
        assert!(result.is_ok());
        let record = result.unwrap().unwrap();
        assert!(!record.predicates.is_empty());
    }

    #[test]
    fn proposal_by_nonexistent_run_returns_error() {
        let store = test_store_root();
        let result = openwand_app::workflow_proposal::proposal_and_review_by_workflow_run(&store, "nonexistent_run");
        assert!(result.is_err());
    }

    #[test]
    fn readiness_by_nonexistent_run_returns_error() {
        let store = test_store_root();
        let result = openwand_app::workflow_readiness::readiness_by_workflow_run(&store, "nonexistent_run");
        assert!(result.is_err());
    }

    #[test]
    fn proposal_state_builds_complete_ui_state() {
        use openwand_app::ui::workflow_proposal_state::*;

        let store = test_store_root();
        let wfx_id = build_full_chain(&store);
        let (proposal, review_opt) = openwand_app::workflow_proposal::proposal_and_review_by_workflow_run(&store, &wfx_id.0)
            .unwrap().unwrap();

        let ui_state = WorkflowProposalUiState {
            latest_proposal: Some(workflow_proposal_summary_lines(&proposal)),
            latest_review: review_opt.as_ref().map(workflow_proposal_review_lines),
            stages: workflow_stage_rows(&proposal),
            tool_intents: workflow_tool_intent_rows(&proposal),
            risks: workflow_risk_rows(&proposal),
            approvals: workflow_approval_marker_rows(&proposal),
            abort_rollback_notes: workflow_abort_rollback_rows(&proposal),
            evidence_links: workflow_proposal_evidence_rows(&proposal),
            warnings: vec![],
        };

        assert!(ui_state.latest_proposal.is_some());
        let p = ui_state.latest_proposal.unwrap();
        assert!(p.stage_count > 0);
        assert!(!ui_state.stages.is_empty());
        assert!(ui_state.latest_review.is_some());
        let r = ui_state.latest_review.unwrap();
        assert_eq!(r.decision, "approved");
    }

    #[test]
    fn readiness_state_builds_complete_ui_state() {
        use openwand_app::ui::workflow_readiness_state::*;

        let store = test_store_root();
        let wfx_id = build_full_chain(&store);
        let record = openwand_app::workflow_readiness::readiness_by_workflow_run(&store, &wfx_id.0)
            .unwrap().unwrap();

        let ui_state = WorkflowReadinessUiState {
            latest_readiness: Some(workflow_readiness_summary_lines(&record)),
            predicates: workflow_readiness_predicate_rows(&record),
            tool_intents: tool_intent_resolution_rows(&record),
            approval_markers: workflow_approval_marker_rows(&record),
            environment: Some(workflow_environment_lines(&record)),
            rollback_abort: Some(workflow_rollback_abort_lines(&record)),
            warnings: vec![],
        };

        assert!(ui_state.latest_readiness.is_some());
        let r = ui_state.latest_readiness.unwrap();
        assert!(!r.readiness_id.is_empty());
        assert!(!ui_state.predicates.is_empty());
        assert!(ui_state.environment.is_some());
        assert!(ui_state.environment.unwrap().workspace_observed);
    }

    #[test]
    fn empty_state_when_no_run_exists_is_none() {
        // When no workflow run exists, proposal_and_review returns Err,
        // which the inspector shell converts to None (empty/unavailable state).
        let store = test_store_root();
        let result = openwand_app::workflow_proposal::proposal_and_review_by_workflow_run(&store, "no_such_run");
        assert!(result.is_err());
        // Inspector shell handles this by setting signal to None
    }
}
