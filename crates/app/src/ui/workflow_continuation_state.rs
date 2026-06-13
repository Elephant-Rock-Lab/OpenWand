//! UI workflow continuation state — read-only display helpers.

use openwand_workflow::workflow_continuation::*;

#[derive(Debug, Clone)]
pub struct WorkflowContinuationReadinessRow { pub readiness_id: String, pub status: String, pub decision: String }
#[derive(Debug, Clone)]
pub struct WorkflowNextActionProposalRow { pub proposal_id: String, pub stage_id: String, pub candidate_kind: String }
#[derive(Debug, Clone)]
pub struct WorkflowContinuationPredicateRow { pub predicate: String, pub passed: bool, pub reason: String }
#[derive(Debug, Clone)]
pub struct WorkflowNextActionCandidateRow { pub stage_id: String, pub action_request_id: Option<String>, pub candidate_kind: String }
#[derive(Debug, Clone)]
pub struct WorkflowContinuationEvidenceRow { pub kind: String, pub id: String, pub summary: String }

#[derive(Debug, Clone)]
pub struct WorkflowContinuationUiState {
    pub latest_readiness: Option<WorkflowContinuationReadinessRow>,
    pub latest_proposal: Option<WorkflowNextActionProposalRow>,
    pub predicates: Vec<WorkflowContinuationPredicateRow>,
    pub candidate: Option<WorkflowNextActionCandidateRow>,
    pub evidence_links: Vec<WorkflowContinuationEvidenceRow>,
    pub warnings: Vec<String>,
}

pub fn workflow_continuation_safety_warning() -> String {
    "Workflow continuation proposes the next eligible action as evidence only. \
     It does not route actions, resolve approvals, execute tools, append trace, \
     or mutate workflow run state.".into()
}

pub fn workflow_continuation_readiness_summary(record: &WorkflowContinuationReadinessRecord) -> WorkflowContinuationReadinessRow {
    WorkflowContinuationReadinessRow {
        readiness_id: record.readiness_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
        decision: format!("{:?}", record.decision).to_lowercase(),
    }
}

pub fn workflow_continuation_predicate_rows(record: &WorkflowContinuationReadinessRecord) -> Vec<WorkflowContinuationPredicateRow> {
    record.predicates.iter().map(|p| WorkflowContinuationPredicateRow {
        predicate: format!("{:?}", p.predicate), passed: p.passed, reason: p.reason.clone(),
    }).collect()
}

pub fn workflow_next_action_candidate_lines(candidate: &WorkflowNextActionCandidate) -> WorkflowNextActionCandidateRow {
    WorkflowNextActionCandidateRow {
        stage_id: candidate.stage_id.clone(),
        action_request_id: candidate.action_request_id.clone(),
        candidate_kind: format!("{:?}", candidate.candidate_kind).to_lowercase(),
    }
}

pub fn workflow_continuation_evidence_rows(proposal: &WorkflowNextActionProposal) -> Vec<WorkflowContinuationEvidenceRow> {
    proposal.evidence_links.iter().map(|l| WorkflowContinuationEvidenceRow {
        kind: format!("{:?}", l.kind).to_lowercase(),
        id: l.id.clone(), summary: l.summary.clone(),
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_readiness() -> WorkflowContinuationReadinessRecord {
        WorkflowContinuationReadinessRecord {
            readiness_id: WorkflowContinuationReadinessId("wcr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            run_revision_hash: "h".into(),
            status: WorkflowContinuationStatus::ProposalReady,
            decision: WorkflowContinuationDecision::ProposalReady { summary: "ok".into() },
            predicates: vec![WorkflowContinuationPredicateResult {
                predicate: WorkflowContinuationPredicate::WorkflowRunExists, passed: true, reason: "ok".into(),
            }],
            selected_candidate: Some(WorkflowNextActionCandidate {
                stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                stage_title: "Stage 1".into(), reason: "deps met".into(), dependency_evidence: vec![],
            }),
            created_at: Utc::now(),
        }
    }

    fn test_proposal() -> WorkflowNextActionProposal {
        WorkflowNextActionProposal {
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            readiness_id: WorkflowContinuationReadinessId("wcr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h".into(),
            candidate: WorkflowNextActionCandidate {
                stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                stage_title: "Stage 1".into(), reason: "deps met".into(), dependency_evidence: vec![],
            },
            predicates: vec![], evidence_links: vec![
                WorkflowContinuationEvidenceLink { kind: WorkflowContinuationEvidenceKind::WorkflowRunRevision, id: "wrr_t".into(), summary: "Source revision".into() },
                WorkflowContinuationEvidenceLink { kind: WorkflowContinuationEvidenceKind::Stage, id: "s1".into(), summary: "Target stage".into() },
            ],
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            proposal_hash: "ph".into(), created_at: Utc::now(),
        }
    }

    #[test] fn ui_state_loads_latest_continuation() {
        let r = test_readiness();
        let state = WorkflowContinuationUiState {
            latest_readiness: Some(workflow_continuation_readiness_summary(&r)),
            latest_proposal: None,
            predicates: workflow_continuation_predicate_rows(&r),
            candidate: r.selected_candidate.as_ref().map(workflow_next_action_candidate_lines),
            evidence_links: vec![], warnings: vec![],
        };
        assert!(state.latest_readiness.is_some());
        assert!(state.candidate.is_some());
    }
    #[test] fn predicate_rows_show_pass_fail_reason() {
        let rows = workflow_continuation_predicate_rows(&test_readiness());
        assert!(!rows.is_empty()); assert!(rows[0].passed);
    }
    #[test] fn candidate_lines_show_stage_and_action_request() {
        let c = test_readiness().selected_candidate.unwrap();
        let row = workflow_next_action_candidate_lines(&c);
        assert_eq!("s1", row.stage_id);
        assert_eq!(Some("ar_1".into()), row.action_request_id);
    }
    #[test] fn evidence_rows_show_revision_and_stage_links() {
        let rows = workflow_continuation_evidence_rows(&test_proposal());
        assert!(rows.iter().any(|r| r.kind.contains("revision")));
        assert!(rows.iter().any(|r| r.kind.contains("stage")));
    }
    #[test] fn safety_warning_mentions_no_routing_or_execution() {
        let w = workflow_continuation_safety_warning();
        assert!(w.contains("route actions") && w.contains("does not"));
        assert!(w.contains("execute tools"));
        assert!(w.contains("append trace"));
    }
}
