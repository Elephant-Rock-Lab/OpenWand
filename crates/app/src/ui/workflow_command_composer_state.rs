//! UI command composer state — display-only helpers.

use openwand_workflow::workflow_command_composer::*;
use openwand_workflow::workflow_command_descriptor::*;
use openwand_workflow::workflow_manual_operation::*;

#[derive(Debug, Clone)]
pub struct WorkflowCommandComposerSummaryRow { pub composer_id: String, pub status: String }
#[derive(Debug, Clone)]
pub struct WorkflowManualCommandDescriptorRow { pub command_kind: String, pub display_command: String, pub has_missing_inputs: bool }
#[derive(Debug, Clone)]
pub struct WorkflowCommandArgumentRow { pub name: String, pub value_preview: Option<String>, pub required: bool, pub missing: bool }
#[derive(Debug, Clone)]
pub struct WorkflowCommandMissingInputRow { pub name: String, pub reason: String, pub suggested_source: String }
#[derive(Debug, Clone)]
pub struct WorkflowCommandEvidenceRow { pub kind: String, pub id: String, pub summary: String }
#[derive(Debug, Clone)]
pub struct WorkflowCommandComposerPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }

#[derive(Debug, Clone)]
pub struct WorkflowCommandComposerUiState {
    pub latest_record: Option<WorkflowCommandComposerSummaryRow>,
    pub descriptor: Option<WorkflowManualCommandDescriptorRow>,
    pub arguments: Vec<WorkflowCommandArgumentRow>,
    pub missing_inputs: Vec<WorkflowCommandMissingInputRow>,
    pub evidence_links: Vec<WorkflowCommandEvidenceRow>,
    pub predicates: Vec<WorkflowCommandComposerPredicateRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_command_summary_lines(record: &WorkflowCommandComposerRecord) -> WorkflowCommandComposerSummaryRow {
    WorkflowCommandComposerSummaryRow {
        composer_id: record.composer_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
    }
}

pub fn workflow_manual_command_descriptor_lines(desc: &WorkflowManualCommandDescriptor) -> WorkflowManualCommandDescriptorRow {
    WorkflowManualCommandDescriptorRow {
        command_kind: format!("{:?}", desc.command_kind).to_lowercase(),
        display_command: desc.display_command.clone(),
        has_missing_inputs: !desc.missing_inputs.is_empty(),
    }
}

pub fn workflow_command_argument_rows(desc: &WorkflowManualCommandDescriptor) -> Vec<WorkflowCommandArgumentRow> {
    desc.arguments.iter().map(|a| WorkflowCommandArgumentRow {
        name: a.name.clone(), value_preview: a.value_preview.clone(),
        required: a.required, missing: a.missing,
    }).collect()
}

pub fn workflow_command_missing_input_rows(record: &WorkflowCommandComposerRecord) -> Vec<WorkflowCommandMissingInputRow> {
    record.missing_inputs.iter().map(|m| WorkflowCommandMissingInputRow {
        name: m.name.clone(), reason: m.reason.clone(), suggested_source: m.suggested_source.clone(),
    }).collect()
}

pub fn workflow_command_evidence_rows(record: &WorkflowCommandComposerRecord) -> Vec<WorkflowCommandEvidenceRow> {
    record.evidence_links.iter().map(|e| WorkflowCommandEvidenceRow {
        kind: format!("{:?}", e.kind).to_lowercase(), id: e.id.clone(), summary: e.summary.clone(),
    }).collect()
}

pub fn workflow_command_predicate_rows(record: &WorkflowCommandComposerRecord) -> Vec<WorkflowCommandComposerPredicateRow> {
    record.predicates.iter().map(|p| WorkflowCommandComposerPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_command_composer_safety_warning() -> String {
    "Manual command descriptors are display-only. OpenWand does not execute, \
     route, approve, reconcile, schedule, or mutate workflow state from this screen.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_manual_operation::*;
    use openwand_workflow::workflow_command_descriptor::*;
    use chrono::Utc;

    fn test_record() -> WorkflowCommandComposerRecord {
        let desc = WorkflowManualCommandDescriptor {
            command_kind: WorkflowManualCommandKind::WorkflowContinuationPropose,
            display_command: "openwand workflow-continuation propose".into(),
            arguments: vec![WorkflowManualCommandArgument {
                name: "workflow_execution_id".into(), value_preview: Some("wfx_t".into()),
                source: WorkflowCommandArgumentSource::WorkflowRun, required: true, missing: false,
            }],
            missing_inputs: vec![WorkflowCommandMissingInput {
                name: "review_decision".into(), reason: "Operator must choose".into(),
                suggested_source: "OperatorInput".into(),
            }],
            safety_warnings: vec!["display only".into()],
            evidence_links: vec![WorkflowCommandEvidenceLink {
                kind: WorkflowCommandEvidenceKind::LoopController, id: "wlc_t".into(), summary: "ready".into(),
            }],
            copyable_text: "openwand workflow-continuation propose --workflow-execution-id wfx_t".into(),
            display_only: true, executable: false,
        };
        WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId("wcc_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            loop_controller_hash: "h".into(),
            status: WorkflowCommandComposerStatus::MissingInputs,
            decision: WorkflowCommandComposerDecision::MissingInputs { summary: "missing review_decision".into() },
            predicates: vec![WorkflowCommandComposerPredicateResult {
                predicate: WorkflowCommandComposerPredicate::LoopControllerRecordExists, passed: true, reason: "ok".into(),
            }],
            descriptor: Some(desc), missing_inputs: vec![],
            evidence_links: vec![WorkflowCommandEvidenceLink {
                kind: WorkflowCommandEvidenceKind::LoopController, id: "wlc_t".into(), summary: "ready".into(),
            }],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        }
    }

    #[test] fn ui_state_loads_latest_command_descriptor() {
        let state = WorkflowCommandComposerUiState {
            latest_record: Some(workflow_command_summary_lines(&test_record())),
            descriptor: None, arguments: vec![], missing_inputs: vec![],
            evidence_links: vec![], predicates: vec![], warnings: vec![],
        };
        assert!(state.latest_record.is_some());
    }
    #[test] fn descriptor_lines_show_display_command_and_warning() {
        let rec = test_record();
        let row = workflow_manual_command_descriptor_lines(rec.descriptor.as_ref().unwrap());
        assert!(row.display_command.contains("workflow-continuation"));
        assert!(row.has_missing_inputs);
    }
    #[test] fn argument_rows_show_required_and_missing() {
        let rec = test_record();
        let rows = workflow_command_argument_rows(rec.descriptor.as_ref().unwrap());
        assert!(!rows.is_empty());
        assert!(rows[0].required);
        assert!(!rows[0].missing);
    }
    #[test] fn missing_input_rows_show_reason_and_suggested_source() {
        let mut rec = test_record();
        rec.missing_inputs.push(WorkflowCommandMissingInput {
            name: "decision".into(), reason: "Must choose".into(), suggested_source: "Operator".into(),
        });
        let rows = workflow_command_missing_input_rows(&rec);
        assert_eq!(1, rows.len());
        assert_eq!("Must choose", rows[0].reason);
    }
    #[test] fn evidence_rows_show_link_kind_and_summary() {
        let rec = test_record();
        let rows = workflow_command_evidence_rows(&rec);
        assert!(!rows.is_empty());
        assert!(rows[0].kind.contains("loopcontroller"));
    }
    #[test] fn predicate_rows_show_pass_fail_reason() {
        let rec = test_record();
        let rows = workflow_command_predicate_rows(&rec);
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn safety_warning_mentions_display_only() {
        let w = workflow_command_composer_safety_warning();
        assert!(w.contains("display-only"));
        assert!(w.contains("does not execute"));
        assert!(w.contains("route"));
    }
}
