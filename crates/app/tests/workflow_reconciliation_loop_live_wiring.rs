//! Tests for live workflow reconciliation + loop controller wiring (Wave 84C).
//!
//! Proves existing by_workflow_run loaders resolve reconciliation and loop
//! controller records, and that UI state conversion functions produce correct
//! display data from those records.

#[cfg(test)]
mod tests {
    // ── Reconciliation ──────────────────────────────────────────────────

    mod reconciliation {
        use openwand_app::workflow_reconciliation::*;
        use openwand_app::ui::workflow_reconciliation_state::*;
        use openwand_workflow::workflow_reconciliation::*;
        use openwand_workflow::workflow_run::WorkflowExecutionId;
        use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
        use openwand_workflow::workflow_action_outcome::{WorkflowActionOutcomeId, WorkflowActionOutcomeStatus};
        use chrono::Utc;

        fn test_store() -> std::path::PathBuf {
            tempfile::tempdir().unwrap().into_path()
        }

        fn test_record(wfx: &str) -> WorkflowReconciliationRecord {
            WorkflowReconciliationRecord {
                reconciliation_id: WorkflowReconciliationId("wrc_84c".into()),
                workflow_execution_id: WorkflowExecutionId(wfx.into()),
                route_id: WorkflowActionRouteId("war_84c".into()),
                outcome_id: WorkflowActionOutcomeId("wao_84c".into()),
                stage_id: "stage_1".into(),
                action_request_id: "ar_84c".into(),
                status: WorkflowReconciliationStatus::Reconciled,
                decision: WorkflowReconciliationDecision::Reconciled {
                    summary: "Stage completed from tool outcome".into(),
                },
                predicates: vec![
                    WorkflowReconciliationPredicateResult {
                        predicate: WorkflowReconciliationPredicate::WorkflowRunExists,
                        passed: true,
                        reason: "Run found".into(),
                    },
                    WorkflowReconciliationPredicateResult {
                        predicate: WorkflowReconciliationPredicate::OutcomeRecordExists,
                        passed: true,
                        reason: "Outcome found".into(),
                    },
                    WorkflowReconciliationPredicateResult {
                        predicate: WorkflowReconciliationPredicate::OutcomeLinksSameWorkflowRun,
                        passed: true,
                        reason: "IDs match".into(),
                    },
                ],
                progression: Some(WorkflowStageProgression {
                    stage_id: "stage_1".into(),
                    previous_status: openwand_workflow::workflow_run::WorkflowStageRunStatus::Suspended,
                    new_status: openwand_workflow::workflow_run::WorkflowStageRunStatus::Completed,
                    outcome_status: WorkflowActionOutcomeStatus::ToolCompleted,
                    lifecycle_event: openwand_workflow::workflow_run::WorkflowStageLifecycleEvent {
                        event_id: "evt_84c".into(),
                        stage_id: "stage_1".into(),
                        event_kind: openwand_workflow::workflow_run::WorkflowStageLifecycleKind::StageCompleted,
                        summary: "Stage completed from session tool outcome".into(),
                        occurred_at: Utc::now(),
                    },
                    summary: "Stage advanced from tool outcome evidence".into(),
                }),
                new_run_revision_id: None,
                created_at: Utc::now(),
            }
        }

        #[test]
        fn reconciliation_by_workflow_run_loads_record() {
            let store = test_store();
            let record = test_record("wfx_84c_recon");
            save_workflow_reconciliation(&store, &record).unwrap();

            let loaded = reconciliation_by_workflow_run(&store, "wfx_84c_recon");
            assert!(loaded.is_ok());
            let loaded = loaded.unwrap().unwrap();
            assert_eq!(loaded.reconciliation_id, record.reconciliation_id);
            assert_eq!(loaded.status, WorkflowReconciliationStatus::Reconciled);
        }

        #[test]
        fn reconciliation_ui_state_builds_from_record() {
            let store = test_store();
            let record = test_record("wfx_84c_ui");
            save_workflow_reconciliation(&store, &record).unwrap();

            let loaded = reconciliation_by_workflow_run(&store, "wfx_84c_ui").unwrap().unwrap();

            let ui_state = WorkflowReconciliationUiState {
                latest_reconciliation: Some(workflow_reconciliation_summary(&loaded)),
                latest_run_revision: None,
                predicates: workflow_reconciliation_predicate_rows(&loaded),
                progression: loaded.progression.as_ref().map(workflow_stage_progression_lines),
                lifecycle_event: loaded.progression.as_ref().map(workflow_lifecycle_event_lines),
                warnings: vec![],
            };

            let summary = ui_state.latest_reconciliation.unwrap();
            assert_eq!(summary.reconciliation_id, "wrc_84c");
            assert!(summary.status.contains("reconciled"));

            assert_eq!(ui_state.predicates.len(), 3);
            assert!(ui_state.predicates.iter().all(|p| p.passed));

            let prog = ui_state.progression.unwrap();
            assert_eq!(prog.stage_id, "stage_1");
            assert_eq!(prog.previous_status, "suspended");
            assert_eq!(prog.new_status, "completed");

            let evt = ui_state.lifecycle_event.unwrap();
            assert_eq!(evt.event_id, "evt_84c");
            assert!(evt.event_kind.contains("stagecompleted"));
        }

        #[test]
        fn reconciliation_empty_when_no_record() {
            let store = test_store();
            let result = reconciliation_by_workflow_run(&store, "no_such_run");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn reconciliation_blocked_status_displays() {
            let store = test_store();
            let mut record = test_record("wfx_84c_blocked");
            record.status = WorkflowReconciliationStatus::Blocked;
            record.decision = WorkflowReconciliationDecision::Blocked {
                reason_code: "mismatch".into(),
                summary: "Hash mismatch".into(),
            };
            save_workflow_reconciliation(&store, &record).unwrap();

            let loaded = reconciliation_by_workflow_run(&store, "wfx_84c_blocked").unwrap().unwrap();
            let summary = workflow_reconciliation_summary(&loaded);
            assert!(summary.status.contains("blocked"));
        }
    }

    // ── Loop Controller ─────────────────────────────────────────────────

    mod loop_controller {
        use openwand_app::workflow_loop_controller::*;
        use openwand_app::ui::workflow_loop_controller_state::*;
        use openwand_workflow::workflow_loop_controller::*;
        use openwand_workflow::workflow_loop_recommendation::*;
        use openwand_workflow::workflow_loop_state::*;
        use openwand_workflow::workflow_run::WorkflowExecutionId;
        use chrono::Utc;

        fn test_store() -> std::path::PathBuf {
            tempfile::tempdir().unwrap().into_path()
        }

        fn test_record(wfx: &str) -> WorkflowLoopControllerRecord {
            WorkflowLoopControllerRecord {
                controller_id: WorkflowLoopControllerId("wlc_84c".into()),
                workflow_execution_id: WorkflowExecutionId(wfx.into()),
                latest_run_revision_id: None,
                status: WorkflowLoopControllerStatus::RecommendationReady,
                decision: WorkflowLoopControllerDecision::Recommend {
                    operation: WorkflowManualOperationKind::CreateContinuationProposal,
                    summary: "Needs continuation".into(),
                },
                loop_state: Some(WorkflowLoopState {
                    workflow_execution_id: WorkflowExecutionId(wfx.into()),
                    latest_run_revision_id: None,
                    run_status: "suspended".into(),
                    stage_summary: vec![],
                    latest_route_id: None,
                    latest_outcome_id: None,
                    latest_reconciliation_id: None,
                    latest_continuation_readiness_id: None,
                    latest_next_action_proposal_id: None,
                    latest_next_action_review_id: None,
                    latest_routing_readiness_id: None,
                    latest_next_action_routing_id: None,
                    latest_command_composer_id: None,
                    latest_command_review_id: None,
                    latest_manual_result_id: None,
                    latest_manual_result_review_id: None,
                    latest_reconciliation_readiness_id: None,
                    latest_manual_reconciliation_gate_id: None,
                    detected_state: WorkflowDetectedLoopState::NeedsInitialContinuationProposal,
                }),
                recommendation: Some(WorkflowLoopRecommendation {
                    operation: WorkflowManualOperationKind::CreateContinuationProposal,
                    command_hint: "display only".into(),
                    reason: "No continuation proposal exists yet".into(),
                    required_inputs: vec![],
                    evidence_links: vec![
                        WorkflowLoopEvidenceLink {
                            link_kind: "route".into(),
                            record_id: "war_84c".into(),
                            summary: "Action was routed".into(),
                        },
                    ],
                }),
                predicates: vec![
                    WorkflowLoopPredicateResult {
                        predicate: WorkflowLoopPredicate::WorkflowRunExists,
                        passed: true,
                        reason: "Run found".into(),
                    },
                    WorkflowLoopPredicateResult {
                        predicate: WorkflowLoopPredicate::NoConflictingLatestRecords,
                        passed: true,
                        reason: "No conflicts".into(),
                    },
                ],
                evidence_links: vec![],
                creates_route: false,
                resolves_approval: false,
                reconciles_outcome: false,
                executes_tool: false,
                mutates_workflow_state: false,
                schedules_work: false,
                starts_worker: false,
                queues_operation: false,
                retries_operation: false,
                resumes_workflow: false,
                created_at: Utc::now(),
            }
        }

        #[test]
        fn controller_by_workflow_run_loads_record() {
            let store = test_store();
            let record = test_record("wfx_84c_loop");
            save_loop_controller(&store, &record).unwrap();

            let loaded = controller_by_workflow_run(&store, "wfx_84c_loop");
            assert!(loaded.is_ok());
            let loaded = loaded.unwrap().unwrap();
            assert_eq!(loaded.controller_id, record.controller_id);
        }

        #[test]
        fn loop_controller_ui_state_builds_from_record() {
            let store = test_store();
            let record = test_record("wfx_84c_loop_ui");
            save_loop_controller(&store, &record).unwrap();

            let loaded = controller_by_workflow_run(&store, "wfx_84c_loop_ui").unwrap().unwrap();

            let ui_state = WorkflowLoopControllerUiState {
                latest_controller: Some(workflow_loop_controller_summary_lines(&loaded)),
                detected_state: loaded.loop_state.as_ref().map(workflow_loop_detected_state_lines),
                recommendation: loaded.recommendation.as_ref().map(workflow_loop_recommendation_lines),
                predicates: workflow_loop_predicate_rows(&loaded),
                evidence_links: workflow_loop_evidence_rows(&loaded),
                warnings: vec![],
            };

            let summary = ui_state.latest_controller.unwrap();
            assert_eq!(summary.controller_id, "wlc_84c");
            assert!(summary.status.contains("recommendation"));

            let detected = ui_state.detected_state.unwrap();
            assert!(detected.contains("needsinitial"));

            let rec = ui_state.recommendation.unwrap();
            assert!(rec.operation.contains("create"));
            assert!(rec.reason.contains("No continuation"));

            assert_eq!(ui_state.predicates.len(), 2);
            assert!(ui_state.predicates.iter().all(|p| p.passed));
        }

        #[test]
        fn loop_controller_empty_when_no_record() {
            let store = test_store();
            let result = controller_by_workflow_run(&store, "no_such_run");
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }

        #[test]
        fn loop_controller_different_runs_load_different_records() {
            let store = test_store();
            let mut r1 = test_record("wfx_84c_a");
            r1.controller_id = WorkflowLoopControllerId("wlc_a".into());
            save_loop_controller(&store, &r1).unwrap();

            let mut r2 = test_record("wfx_84c_b");
            r2.controller_id = WorkflowLoopControllerId("wlc_b".into());
            save_loop_controller(&store, &r2).unwrap();

            let a = controller_by_workflow_run(&store, "wfx_84c_a").unwrap().unwrap();
            let b = controller_by_workflow_run(&store, "wfx_84c_b").unwrap().unwrap();
            assert_ne!(a.controller_id, b.controller_id);
        }
    }
}
