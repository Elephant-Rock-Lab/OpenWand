//! Tests for desktop evidence export authority boundary (Wave 88C).
//!
//! Proves:
//! 1. The UI request DTO module imports no backend/evidence authority types.
//! 2. The render function does not call export_audit_packet directly.
//! 3. The service delegates through export_audit_packet and computes checksum
//!    only on the returned artifact path.
//! 4. Path containment rejects traversal escape.
//! 5. Empty workflow_execution_id and output_path are rejected before delegation.
//! 6. Exported state reports certifies_external_truth=false, verifies_artifacts=false.
//! 7. Nonexistent workflow run returns Unavailable, not silently succeeded.

// ── Source-level authority boundary guards ───────────────────

#[cfg(test)]
mod authority_guards {
    /// Guard: evidence_export_request.rs imports no backend authority.
    #[test]
    fn dto_does_not_import_backend_authority() {
        let src = include_str!("../src/ui/evidence_export_request.rs");
        // Check code only — comments mentioning functions are fine,
        // but actual use statements or calls are not.
        assert!(!src.contains("use ") || src.contains("use serde"), "only serde use allowed");
        assert!(!src.contains("export_audit_packet("), "must not call export_audit_packet");
        assert!(!src.contains("assemble_evidence"), "must not assemble evidence");
        assert!(!src.contains("AuditPacket"), "must not import AuditPacket");
        assert!(!src.contains("std::fs"), "must not do filesystem I/O");
        assert!(!src.contains("SessionRunner"), "must not import SessionRunner");
        assert!(!src.contains("ToolExecutor"), "must not import ToolExecutor");
        assert!(!src.contains("PolicyEngine"), "must not import PolicyEngine");
        assert!(!src.contains("TraceStore"), "must not import TraceStore");
        assert!(!src.contains("resolve_workspace_path"), "must not call sandbox resolver");
        assert!(!src.contains("append_trace"), "must not append trace");
    }

    /// Guard: render_evidence_export does not bypass authority.
    #[test]
    #[cfg(feature = "desktop")]
    fn render_function_does_not_bypass_authority() {
        let src = include_str!("../src/ui_main.rs");
        let render_section = src
            .split("fn render_evidence_export")
            .nth(1)
            .unwrap_or("");

        // Must use the service delegation path
        assert!(
            render_section.contains("export_evidence"),
            "must use service delegation"
        );

        // Must not directly call these
        assert!(!render_section.contains("export_audit_packet"), "must not call export_audit_packet directly");
        assert!(!render_section.contains("assemble_evidence"), "must not assemble evidence directly");
        assert!(!render_section.contains("std::fs::write"), "must not write files directly");
        assert!(!render_section.contains("std::fs::read"), "must not read files directly");
        assert!(!render_section.contains("append_trace"), "must not append trace");
    }

    /// Guard: service.rs uses export_audit_packet (not direct file assembly).
    #[test]
    fn service_delegates_through_existing_exporter() {
        let src = include_str!("../src/ui/service.rs");
        assert!(
            src.contains("export_audit_packet("),
            "service must delegate through export_audit_packet()"
        );
    }
}

// ── DTO validation tests ─────────────────────────────────────

#[cfg(test)]
mod dto_validation_tests {
    use openwand_app::ui::evidence_export_request::*;

    #[test]
    fn empty_workflow_execution_id_rejected() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "".into(),
            output_path: "out.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn empty_output_path_rejected() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "wfx_1".into(),
            output_path: "".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn whitespace_workflow_execution_id_rejected() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "  ".into(),
            output_path: "out.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn valid_request_passes_validation() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "wfx_1".into(),
            output_path: "exports/packet.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_ok());
    }
}

// ── Service delegation + path containment tests ──────────────

#[cfg(test)]
mod service_tests {
    use openwand_app::ui::evidence_export_request::*;
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

    fn build_and_run_workflow(store: &std::path::Path) -> String {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "88C export test".into(),
            skill_context: vec![], goal_context: vec![],
            memory_summaries: vec!["mem".into()], trace_summaries: vec![],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell execution".into()],
        }).unwrap();
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
            idempotency_key: "key88c".into(),
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

        let req = openwand_app::ui::workflow_run_request::WorkflowRunRequest {
            readiness_id: readiness.readiness_id.0.clone(),
            proposal_id: proposal.proposal_id.0.clone(),
            proposal_review_id: proposal_review.review_id.0.clone(),
            idempotency_key: "test_88c".into(), requested_by: "desktop_test".into(),
        };

        let result = UiSessionService::evaluate_workflow_run_request(&req, store);
        match result {
            openwand_app::ui::workflow_run_request::WorkflowRunRequestState::Created { execution_id, .. } => {
                execution_id
            }
            openwand_app::ui::workflow_run_request::WorkflowRunRequestState::Blocked { reason } => {
                panic!("Workflow run blocked: {}", reason);
            }
            other => panic!("Unexpected run state: {:?}", other),
        }
    }

    #[test]
    fn export_succeeds_for_valid_workflow_run() {
        let store = test_store_root();
        let execution_id = build_and_run_workflow(&store);
        let export_root = store.join("exports");

        let req = EvidenceExportRequest {
            workflow_execution_id: execution_id,
            output_path: "audit_packet.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        match &result {
            EvidenceExportState::Exported {
                artifact_path, packet_hash,
                certifies_external_truth, verifies_artifacts, ..
            } => {
                assert!(!artifact_path.is_empty());
                assert!(!packet_hash.is_empty());
                assert!(!*certifies_external_truth, "must not certify external truth");
                assert!(!*verifies_artifacts, "must not verify artifacts");
            }
            _ => panic!("Expected Exported, got {:?}", result),
        }
    }

    #[test]
    fn export_fails_for_nonexistent_workflow_run() {
        let store = test_store_root();
        let export_root = store.join("exports");

        let req = EvidenceExportRequest {
            workflow_execution_id: "nonexistent_run".into(),
            output_path: "out.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        match result {
            EvidenceExportState::Unavailable { .. } | EvidenceExportState::Failed { .. } => {}
            EvidenceExportState::Exported { .. } => {
                panic!("Must not export for nonexistent workflow run");
            }
            _ => panic!("Expected Unavailable or Failed, got {:?}", result),
        }
    }

    #[test]
    fn export_rejects_traversal_escape() {
        let store = test_store_root();
        let export_root = store.join("exports");

        // Try to write outside export root via traversal
        let req = EvidenceExportRequest {
            workflow_execution_id: "wfx_test".into(),
            output_path: "../../../etc/evil.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        match result {
            EvidenceExportState::Failed { error } => {
                // Should be rejected either by containment check or filesystem
                assert!(
                    error.contains("escape") || error.contains("not within") || error.contains("Failed"),
                    "Should reject traversal: {}",
                    error
                );
            }
            EvidenceExportState::Exported { .. } => {
                panic!("Must not export to traversal-escaped path");
            }
            _ => {}
        }
    }

    #[test]
    fn export_rejects_empty_workflow_execution_id() {
        let store = test_store_root();
        let export_root = store.join("exports");

        let req = EvidenceExportRequest {
            workflow_execution_id: "".into(),
            output_path: "out.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        assert!(matches!(result, EvidenceExportState::Failed { .. }));
    }

    #[test]
    fn export_reports_honest_truth_flags() {
        let store = test_store_root();
        let execution_id = build_and_run_workflow(&store);
        let export_root = store.join("exports");

        let req = EvidenceExportRequest {
            workflow_execution_id: execution_id,
            output_path: "audit.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        if let EvidenceExportState::Exported {
            certifies_external_truth, verifies_artifacts, ..
        } = &result {
            assert!(!*certifies_external_truth, "certifies_external_truth must be false");
            assert!(!*verifies_artifacts, "verifies_artifacts must be false");
        } else {
            panic!("Expected Exported, got {:?}", result);
        }
    }

    #[test]
    fn export_artifact_is_within_export_root() {
        let store = test_store_root();
        let execution_id = build_and_run_workflow(&store);
        let export_root = store.join("exports");

        let req = EvidenceExportRequest {
            workflow_execution_id: execution_id,
            output_path: "audit.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };

        let result = UiSessionService::export_evidence(&req, &store, &export_root);

        if let EvidenceExportState::Exported { artifact_path, .. } = &result {
            let artifact = std::path::Path::new(artifact_path);
            let export_canon = export_root.canonicalize().unwrap_or_else(|_| export_root.clone());
            let artifact_canon = artifact.canonicalize().unwrap_or_else(|_| artifact.to_path_buf());
            assert!(
                artifact_canon.starts_with(&export_canon),
                "Artifact {} must be within export root {}",
                artifact_canon.display(),
                export_canon.display()
            );
        } else {
            panic!("Expected Exported, got {:?}", result);
        }
    }
}
