//! Tests for workflow run initiation authority boundary (Wave 88A).
//!
//! Proves:
//! 1. The UI request DTO module imports no backend execution authority.
//! 2. The service method delegates through the execution gate.
//! 3. The request lifecycle states are honest.
//! 4. A full chain (task plan → proposal → readiness → run) works end-to-end.

#[cfg(test)]
mod authority_tests {
    /// Guard: workflow_run_request.rs does not import backend authority types.
    #[test]
    fn request_dto_does_not_import_execution_authority() {
        let src = include_str!("../src/ui/workflow_run_request.rs");
        assert!(!src.contains("evaluate_workflow_execution"), "must not import execution gate");
        assert!(!src.contains("save_workflow_run"), "must not save workflow runs");
        assert!(!src.contains("SessionRunner"), "must not import SessionRunner");
        assert!(!src.contains("ToolExecutor"), "must not import ToolExecutor");
        assert!(!src.contains("PolicyEngine"), "must not import PolicyEngine");
        assert!(!src.contains("TraceStore"), "must not import TraceStore");
        assert!(!src.contains("MemoryStore"), "must not import MemoryStore");
        assert!(!src.contains("resolve_approval"), "must not resolve approvals");
    }

    /// Guard: the render function in ui_main.rs does not directly call gates.
    #[test]
    #[cfg(feature = "desktop")]
    fn render_function_does_not_bypass_authority() {
        let src = include_str!("../src/ui_main.rs");
        assert!(src.contains("request_workflow_run"), "must use service delegation");
        let render_section = src.split("render_workflow_run_initiation").nth(1).unwrap_or("");
        assert!(!render_section.contains("evaluate_workflow_execution"), "render must not call execution gate directly");
        assert!(!render_section.contains("save_workflow_run"), "render must not save runs directly");
    }

    /// Guard: service.rs uses the execution gate (authority is in the service, not UI).
    #[test]
    fn service_delegates_through_execution_gate() {
        let src = include_str!("../src/ui/service.rs");
        assert!(src.contains("evaluate_workflow_execution"), "service must use execution gate");
        assert!(src.contains("save_workflow_run"), "service must save workflow runs");
    }
}

#[cfg(test)]
mod service_tests {
    use openwand_app::ui::workflow_run_request::*;
    use openwand_app::ui::service::UiSessionService;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use openwand_workflow::workflow_readiness::{WorkflowReadinessRequest, WorkflowEnvironmentSnapshot};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use chrono::Utc;

    fn test_store_root() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn build_full_chain(store: &std::path::Path) -> (String, String, String) {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "88A test".into(),
            skill_context: vec![], goal_context: vec![],
            memory_summaries: vec!["mem".into()], trace_summaries: vec![],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell execution".into(), "Workspace sandbox enforced".into()],
        }).unwrap();

        // Save task plan so load_task_plan finds it
        openwand_app::task_planning::save_task_plan(store, &plan).unwrap();

        let plan_review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let plan_review = TaskPlanReview {
            review_id: plan_review_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "tester".into(), rationale: "OK".into(),
            feedback: None, creates_execution_grant: false, execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };

        let proposal = build_workflow_proposal(WorkflowProposalInput {
            task_plan: plan.clone(),
            latest_task_plan_review: Some(plan_review.clone()),
            task_plan_hash: plan.plan_hash.clone(),
        }).unwrap();

        let proposal_review_id = workflow_review_id_for(
            &proposal.proposal_id, &WorkflowProposalReviewDecision::Approved, "Good",
        );
        let proposal_review = WorkflowProposalReview {
            review_id: proposal_review_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: WorkflowProposalReviewDecision::Approved,
            reviewer: "tester".into(), rationale: "Good".into(),
            feedback: None, creates_execution_grant: false, execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };

        let readiness_request = WorkflowReadinessRequest {
            proposal_id: proposal.proposal_id.clone(),
            review_id: proposal_review.review_id.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "tester".into(), requested_at: Utc::now(),
            idempotency_key: "key88a".into(),
        };

        let context = WorkflowReadinessContext {
            proposal: Some(proposal.clone()),
            review: Some(proposal_review.clone()),
            latest_review_for_proposal: Some(proposal_review.clone()),
            source_task_plan: Some(plan.clone()),
            source_task_plan_review: Some(plan_review.clone()),
            latest_source_task_plan_review: Some(plan_review),
            environment: WorkflowEnvironmentSnapshot {
                workspace_observed: true, provider_config_available: true,
                session_runtime_available: true, tool_manifest_available: true,
                policy_context_available: true, notes: vec![],
            },
            existing_readiness_records: vec![],
        };

        let readiness = evaluate_workflow_readiness(&readiness_request, &context);

        openwand_app::workflow_proposal::save_workflow_proposal(store, &proposal).unwrap();
        openwand_app::workflow_proposal::save_proposal_review(store, &proposal_review).unwrap();
        openwand_app::workflow_readiness::save_workflow_readiness(store, &readiness).unwrap();

        (
            readiness.readiness_id.0.clone(),
            proposal.proposal_id.0.clone(),
            proposal_review.review_id.0.clone(),
        )
    }

    #[test]
    fn request_workflow_run_creates_run_from_valid_chain() {
        let store = test_store_root();
        let (readiness_id, proposal_id, review_id) = build_full_chain(&store);

        let req = WorkflowRunRequest {
            readiness_id, proposal_id, proposal_review_id: review_id,
            idempotency_key: "test_88a".into(), requested_by: "desktop_test".into(),
        };

        let result = UiSessionService::evaluate_workflow_run_request(&req, &store);

        assert!(result.is_terminal());
        match &result {
            WorkflowRunRequestState::Created { execution_id, status, stage_count, predicates_passed, predicates_total } => {
                assert!(!execution_id.is_empty());
                assert!(*stage_count > 0, "should have stages from proposal");
                assert_eq!(predicates_passed, predicates_total, "all predicates should pass");
            }
            WorkflowRunRequestState::Blocked { reason } => {
                panic!("Expected Created, got Blocked: {}", reason);
            }
            WorkflowRunRequestState::Failed { error } => {
                panic!("Expected Created, got Failed: {}", error);
            }
            _ => panic!("Unexpected state: {:?}", result),
        }
    }

    #[test]
    fn request_workflow_run_fails_on_nonexistent_readiness() {
        let store = test_store_root();

        let req = WorkflowRunRequest {
            readiness_id: "nonexistent".into(),
            proposal_id: "nonexistent".into(),
            proposal_review_id: "nonexistent".into(),
            idempotency_key: "test_bad".into(), requested_by: "desktop_test".into(),
        };

        let result = UiSessionService::evaluate_workflow_run_request(&req, &store);

        match result {
            WorkflowRunRequestState::Failed { error } => {
                assert!(error.contains("Failed to load"));
            }
            _ => panic!("Expected Failed, got {:?}", result),
        }
    }

    #[test]
    fn request_workflow_run_same_key_returns_same_run() {
        let store = test_store_root();
        let (readiness_id, proposal_id, review_id) = build_full_chain(&store);

        let req = WorkflowRunRequest {
            readiness_id, proposal_id, proposal_review_id: review_id,
            idempotency_key: "test_88a_same".into(), requested_by: "desktop_test".into(),
        };

        let result1 = UiSessionService::evaluate_workflow_run_request(&req, &store);
        let result2 = UiSessionService::evaluate_workflow_run_request(&req, &store);

        // Same request should produce the same run (idempotent)
        if let (WorkflowRunRequestState::Created { execution_id: id1, .. },
                WorkflowRunRequestState::Created { execution_id: id2, .. }) = (&result1, &result2) {
            assert_eq!(id1, id2, "same request should produce same run");
        } else {
            panic!("Expected both Created, got {:?} and {:?}", result1, result2);
        }
    }

    #[test]
    fn created_run_is_persisted_to_disk() {
        let store = test_store_root();
        let (readiness_id, proposal_id, review_id) = build_full_chain(&store);

        let req = WorkflowRunRequest {
            readiness_id, proposal_id, proposal_review_id: review_id,
            idempotency_key: "test_88a_persist".into(), requested_by: "desktop_test".into(),
        };

        let result = UiSessionService::evaluate_workflow_run_request(&req, &store);

        if let WorkflowRunRequestState::Created { execution_id, .. } = &result {
            // Verify the run was actually saved to disk
            let run = openwand_app::workflow_execution::load_workflow_run(
                &store,
                &openwand_workflow::workflow_run::WorkflowExecutionId(execution_id.clone()),
            );
            assert!(run.is_ok(), "Run should be persisted: {:?}", run);
        } else {
            panic!("Expected Created, got {:?}", result);
        }
    }

    #[test]
    fn request_state_transitions_are_honest() {
        let idle = WorkflowRunRequestState::Idle;
        assert!(!idle.is_terminal());
        assert!(!idle.is_pending());

        let pending = WorkflowRunRequestState::Pending;
        assert!(!pending.is_terminal());
        assert!(pending.is_pending());

        let created = WorkflowRunRequestState::Created {
            execution_id: "wfx_1".into(), status: "running".into(),
            stage_count: 3, predicates_passed: 5, predicates_total: 5,
        };
        assert!(created.is_terminal());
        assert!(!created.is_pending());

        let blocked = WorkflowRunRequestState::Blocked { reason: "test".into() };
        assert!(blocked.is_terminal());

        let failed = WorkflowRunRequestState::Failed { error: "test".into() };
        assert!(failed.is_terminal());
    }
}
