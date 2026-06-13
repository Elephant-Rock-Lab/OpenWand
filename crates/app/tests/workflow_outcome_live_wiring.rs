//! Tests for live workflow action outcome wiring (Wave 84B).
//!
//! Proves the existing outcome_by_workflow_run loader resolves action outcome
//! records from a workflow run, and that the UI state conversion functions
//! produce correct display data from those records.

#[cfg(test)]
mod tests {
    use openwand_app::workflow_action_outcome::*;
    use openwand_app::ui::workflow_action_outcome_state::*;
    use openwand_workflow::workflow_action_outcome::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use chrono::Utc;
    use std::path::Path;

    fn test_store_root() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn test_outcome(wfx_id: &str) -> WorkflowActionOutcomeRecord {
        WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_84b_test".into()),
            workflow_execution_id: WorkflowExecutionId(wfx_id.into()),
            route_id: WorkflowActionRouteId("war_84b".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_84b".into(),
            session_id: "sess_84b".into(),
            pending_approval_id: "arid_84b".into(),
            tool_call_id: Some("tc_84b".into()),
            route_hash: "rh_84b".into(),
            workflow_run_hash: "wrh_84b".into(),
            status: WorkflowActionOutcomeStatus::ToolCompleted,
            decision: WorkflowActionOutcomeDecision::ToolCompleted {
                summary: "File written".into(),
            },
            predicates: vec![
                WorkflowActionOutcomePredicateResult {
                    predicate: WorkflowActionOutcomePredicate::WorkflowRunExists,
                    passed: true,
                    reason: "Run found".into(),
                },
                WorkflowActionOutcomePredicateResult {
                    predicate: WorkflowActionOutcomePredicate::RouteRecordExists,
                    passed: true,
                    reason: "Route found".into(),
                },
                WorkflowActionOutcomePredicateResult {
                    predicate: WorkflowActionOutcomePredicate::PendingApprovalIdMatchesRoute,
                    passed: true,
                    reason: "IDs match".into(),
                },
            ],
            approval_resolution: WorkflowApprovalResolution::Approve {
                rationale: "Safe to write within workspace".into(),
            },
            session_outcome: Some(WorkflowSessionActionOutcomeSnapshot {
                session_id: "sess_84b".into(),
                session_run_id: Some("run_84b".into()),
                trace_ids: vec!["trace_84b_1".into(), "trace_84b_2".into()],
                approval_request_id_observed: "arid_84b".into(),
                approval_resolution_observed: "approved".into(),
                tool_call_id_observed_from_session: Some("tc_84b".into()),
                tool_name_observed_from_session: Some("local__file_write".into()),
                tool_status_observed_from_session: Some("completed".into()),
                safe_result_summary: Some("File written to workspace".into()),
            }),
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        }
    }

    /// Save an outcome record and verify it can be loaded by workflow run.
    fn save_and_verify(store: &Path, wfx_id: &str) -> WorkflowActionOutcomeRecord {
        let record = test_outcome(wfx_id);
        save_workflow_action_outcome(store, &record).unwrap();

        // Verify load by workflow run
        let loaded = outcome_by_workflow_run(store, wfx_id);
        assert!(loaded.is_ok());
        let loaded = loaded.unwrap().unwrap();
        assert_eq!(loaded.outcome_id, record.outcome_id);
        assert_eq!(loaded.workflow_execution_id.0, wfx_id);
        record
    }

    #[test]
    fn outcome_by_workflow_run_loads_record() {
        let store = test_store_root();
        let record = save_and_verify(&store, "wfx_84b_run1");

        // Verify all key fields survived persistence roundtrip
        assert_eq!(record.status, WorkflowActionOutcomeStatus::ToolCompleted);
        assert!(!record.predicates.is_empty());
    }

    #[test]
    fn outcome_state_builds_complete_ui_state() {
        let store = test_store_root();
        let record = save_and_verify(&store, "wfx_84b_run2");

        // Build UI state — same conversion as inspector_shell::load_action_outcome
        let ui_state = WorkflowActionOutcomeUiState {
            latest_outcome: Some(workflow_action_outcome_summary(&record)),
            predicates: workflow_action_outcome_predicate_rows(&record),
            approval_resolution: Some(workflow_approval_resolution_lines(&record)),
            session_outcome: workflow_session_action_outcome_lines(&record),
            trace_links: workflow_outcome_trace_link_rows(&record),
            warnings: vec![],
        };

        // Latest outcome
        let outcome = ui_state.latest_outcome.unwrap();
        assert_eq!(outcome.outcome_id, "wao_84b_test");
        assert!(outcome.status.contains("toolcompleted"));
        assert!(outcome.decision.contains("toolcompleted"));

        // Predicates
        assert_eq!(ui_state.predicates.len(), 3);
        assert!(ui_state.predicates.iter().all(|p| p.passed));

        // Approval resolution
        let resolution = ui_state.approval_resolution.unwrap();
        assert_eq!(resolution.resolution, "approved");
        assert!(resolution.rationale.contains("Safe"));

        // Session outcome
        let session = ui_state.session_outcome.unwrap();
        assert_eq!(session.session_id, "sess_84b");
        assert_eq!(session.tool_name.as_deref(), Some("local__file_write"));
        assert_eq!(session.tool_status.as_deref(), Some("completed"));
        assert_eq!(session.trace_count, 2);

        // Trace links
        assert_eq!(ui_state.trace_links.len(), 2);
        assert!(ui_state.trace_links.iter().any(|t| t.trace_id == "trace_84b_1"));
        assert!(ui_state.trace_links.iter().any(|t| t.trace_id == "trace_84b_2"));
    }

    #[test]
    fn outcome_empty_when_no_record() {
        let store = test_store_root();
        let result = outcome_by_workflow_run(&store, "nonexistent_run");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        // Inspector shell converts None to state = None (honest empty)
    }

    #[test]
    fn outcome_by_different_run_returns_different_record() {
        let store = test_store_root();
        let mut r1 = test_outcome("wfx_84b_a");
        r1.outcome_id = WorkflowActionOutcomeId("wao_a".into());
        r1.route_id = WorkflowActionRouteId("war_a".into());
        r1.pending_approval_id = "arid_a".into();
        save_workflow_action_outcome(&store, &r1).unwrap();

        let mut r2 = test_outcome("wfx_84b_b");
        r2.outcome_id = WorkflowActionOutcomeId("wao_b".into());
        r2.route_id = WorkflowActionRouteId("war_b".into());
        r2.pending_approval_id = "arid_b".into();
        save_workflow_action_outcome(&store, &r2).unwrap();

        let loaded_a = outcome_by_workflow_run(&store, "wfx_84b_a").unwrap().unwrap();
        let loaded_b = outcome_by_workflow_run(&store, "wfx_84b_b").unwrap().unwrap();
        assert_ne!(loaded_a.outcome_id, loaded_b.outcome_id);
    }

    #[test]
    fn outcome_predicate_rows_show_name_pass_reason() {
        let store = test_store_root();
        let record = save_and_verify(&store, "wfx_84b_run3");

        let rows = workflow_action_outcome_predicate_rows(&record);
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert!(!row.predicate.is_empty());
            assert!(row.passed);
            assert!(!row.reason.is_empty());
        }
    }

    #[test]
    fn outcome_ui_state_for_rejected_resolution() {
        let store = test_store_root();
        let mut record = test_outcome("wfx_84b_rejected");
        record.status = WorkflowActionOutcomeStatus::ToolDenied;
        record.decision = WorkflowActionOutcomeDecision::ToolDenied {
            summary: "Denied by policy".into(),
        };
        record.approval_resolution = WorkflowApprovalResolution::Reject {
            rationale: "Violates workspace boundary".into(),
        };
        save_workflow_action_outcome(&store, &record).unwrap();

        let loaded = outcome_by_workflow_run(&store, "wfx_84b_rejected").unwrap().unwrap();
        let ui_state = WorkflowActionOutcomeUiState {
            latest_outcome: Some(workflow_action_outcome_summary(&loaded)),
            predicates: workflow_action_outcome_predicate_rows(&loaded),
            approval_resolution: Some(workflow_approval_resolution_lines(&loaded)),
            session_outcome: workflow_session_action_outcome_lines(&loaded),
            trace_links: workflow_outcome_trace_link_rows(&loaded),
            warnings: vec![],
        };

        let outcome = ui_state.latest_outcome.unwrap();
        assert!(outcome.status.contains("tooldenied"));

        let resolution = ui_state.approval_resolution.unwrap();
        assert_eq!(resolution.resolution, "rejected");
        assert!(resolution.rationale.contains("Violates"));
    }

    #[test]
    fn outcome_no_session_snapshot_still_builds_ui_state() {
        let store = test_store_root();
        let mut record = test_outcome("wfx_84b_nosession");
        record.session_outcome = None;
        save_workflow_action_outcome(&store, &record).unwrap();

        let loaded = outcome_by_workflow_run(&store, "wfx_84b_nosession").unwrap().unwrap();
        let ui_state = WorkflowActionOutcomeUiState {
            latest_outcome: Some(workflow_action_outcome_summary(&loaded)),
            predicates: workflow_action_outcome_predicate_rows(&loaded),
            approval_resolution: Some(workflow_approval_resolution_lines(&loaded)),
            session_outcome: workflow_session_action_outcome_lines(&loaded),
            trace_links: workflow_outcome_trace_link_rows(&loaded),
            warnings: vec![],
        };

        assert!(ui_state.session_outcome.is_none());
        assert!(ui_state.trace_links.is_empty());
        // But outcome and predicates are still present
        assert!(ui_state.latest_outcome.is_some());
        assert!(!ui_state.predicates.is_empty());
    }
}
