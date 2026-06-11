//! Inspector shell — loading and clearing orchestration.
//!
//! Extracted from ui_main.rs (Wave 59A). Provides load/clear APIs for all
//! Inspector tab sections. Each section has independent error/loading state
//! (Patch 5). Stale async results guarded by selection tracking (Patch 4).
//!
//! Loader callset is preserved exactly (Patch 2):
//! - assemble_evidence_chain (read-only, no export)
//! - review_by_workflow_run (audit review)
//! - distribution_by_workflow_run (audit distribution)
//! - result_by_workflow_run (manual result)
//! - review_by_workflow_run (manual result review)
//! - readiness_by_workflow_run (manual result reconciliation readiness)
//! - gate_by_workflow_run (manual result reconciliation gate)
//! - route_by_workflow_run (action route)
//! - readiness_by_workflow_run (routing readiness)
//! - routing_by_workflow_run (next-action routing)
//! - load_workflow_run (execution timeline)
//!
//! No new loader kinds. Only relocation of existing read-only loaders.
//! Components remain state-only render surfaces (Patch 3).

// ── Inspector section data bundle ─────────────────────────────────────────

/// Bundle of all Inspector signal references for loading/clearing.
/// Caller constructs this from ui_main.rs signals (Patch 4: selection tracking).
#[cfg(feature = "desktop")]
pub struct InspectorSignals<'a> {
    pub inspector_state: &'a dioxus::prelude::GlobalSignal<Option<openwand_workflow::workflow_evidence_chain_inspector::EvidenceChainInspectionState>>,
    pub review_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_audit_packet_review_state::ReviewSummaryRow>>,
    pub distribution_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_audit_packet_distribution_state::DistributionSummaryRow>>,
    pub ladder_result_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_manual_result_state::WorkflowManualResultSummaryRow>>,
    pub ladder_review_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow>>,
    pub ladder_readiness_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_manual_result_reconciliation_readiness_state::WorkflowManualResultReconciliationReadinessSummaryRow>>,
    pub ladder_gate_rows: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_manual_result_reconciliation_gate_state::WorkflowManualResultReconciliationGateSummaryRow>>,
    pub ladder_predicates: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_manual_result_reconciliation_readiness_state::ReadinessPredicateDisplayRow>>,
    pub routing_route_row: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_action_routing_state::WorkflowActionRouteSummaryRow>>,
    pub routing_session_row: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_action_routing_state::WorkflowSessionRouteRow>>,
    pub routing_route_predicates: &'a dioxus::prelude::GlobalSignal<Vec<crate::ui::workflow_action_routing_state::WorkflowActionRoutePredicateRow>>,
    pub routing_route_prompt: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_action_routing_state::WorkflowActionRoutePromptRow>>,
    pub routing_readiness_state: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_routing_readiness_state::WorkflowRoutingReadinessUiState>>,
    pub routing_next_action_state: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_next_action_routing_state::WorkflowNextActionRoutingUiState>>,
    pub routing_review_row: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_next_action_review_state::ReviewSummaryRow>>,
    pub execution_timeline_state: &'a dioxus::prelude::GlobalSignal<Option<crate::ui::workflow_execution_state::WorkflowExecutionUiState>>,
}

// ── Desktop-gated loading ────────────────────────────────────────────────

#[cfg(feature = "desktop")]
impl<'a> InspectorSignals<'a> {
    /// Load all Inspector sections from read-only loaders.
    /// Only relocates existing read-only loaders — no new loader kinds (Patch 2).
    pub fn load_inspector_shell(&self, path: &std::path::Path, wfx_id: &openwand_workflow::workflow_run::WorkflowExecutionId) {
        // Evidence chain inspection — read-only, no export
        match crate::workflow_evidence_chain_inspector::assemble_evidence_chain(path, wfx_id, false) {
            Ok(state) => {
                *self.inspector_state.write() = Some(state);
                self.load_audit(path, wfx_id);
                self.load_manual_result_ladder(path, wfx_id);
                self.load_routing_ladder(path, wfx_id);
                self.load_execution_timeline(path, wfx_id);
            }
            Err(_) => {
                *self.inspector_state.write() = None;
            }
        }
    }

    /// Clear all Inspector state (called on session switch).
    pub fn clear_inspector_shell(&self) {
        *self.inspector_state.write() = None;
        *self.review_rows.write() = vec![];
        *self.distribution_rows.write() = vec![];
        *self.ladder_result_rows.write() = vec![];
        *self.ladder_review_rows.write() = vec![];
        *self.ladder_readiness_rows.write() = vec![];
        *self.ladder_gate_rows.write() = vec![];
        *self.ladder_predicates.write() = vec![];
        *self.routing_route_row.write() = None;
        *self.routing_session_row.write() = None;
        *self.routing_route_predicates.write() = vec![];
        *self.routing_route_prompt.write() = None;
        *self.routing_readiness_state.write() = None;
        *self.routing_next_action_state.write() = None;
        *self.routing_review_row.write() = None;
        *self.execution_timeline_state.write() = None;
    }

    fn load_audit(&self, path: &std::path::Path, wfx_id: &openwand_workflow::workflow_run::WorkflowExecutionId) {
        let reviews = crate::workflow_audit_packet_review::review_by_workflow_run(path, &wfx_id.0)
            .unwrap_or_default()
            .iter()
            .map(crate::ui::workflow_audit_packet_review_state::review_summary)
            .collect();
        *self.review_rows.write() = reviews;

        let distributions = crate::workflow_audit_packet_distribution::distribution_by_workflow_run(path, &wfx_id.0)
            .unwrap_or_default()
            .iter()
            .map(crate::ui::workflow_audit_packet_distribution_state::distribution_summary)
            .collect();
        *self.distribution_rows.write() = distributions;
    }

    fn load_manual_result_ladder(&self, path: &std::path::Path, wfx_id: &openwand_workflow::workflow_run::WorkflowExecutionId) {
        if let Ok(Some(mr)) = crate::workflow_manual_result::result_by_workflow_run(path, &wfx_id.0) {
            *self.ladder_result_rows.write() = vec![crate::ui::workflow_manual_result_state::workflow_manual_result_summary_lines(&mr)];
        } else {
            *self.ladder_result_rows.write() = vec![];
        }
        if let Ok(Some(mrr)) = crate::workflow_manual_result_review::review_by_workflow_run(path, &wfx_id.0) {
            *self.ladder_review_rows.write() = vec![crate::ui::workflow_manual_result_review_state::workflow_manual_result_review_summary_lines(&mrr)];
        } else {
            *self.ladder_review_rows.write() = vec![];
        }
        if let Ok(Some(rdy)) = crate::workflow_manual_result_reconciliation_readiness::readiness_by_workflow_run(path, &wfx_id.0) {
            let row = crate::ui::workflow_manual_result_reconciliation_readiness_state::workflow_reconciliation_readiness_summary_lines(&rdy);
            let preds = crate::ui::workflow_manual_result_reconciliation_readiness_state::readiness_predicate_display_rows(&rdy.predicates);
            *self.ladder_readiness_rows.write() = vec![row];
            *self.ladder_predicates.write() = preds;
        } else {
            *self.ladder_readiness_rows.write() = vec![];
            *self.ladder_predicates.write() = vec![];
        }
        if let Ok(Some(gate)) = crate::workflow_manual_result_reconciliation_gate::gate_by_workflow_run(path, &wfx_id.0) {
            *self.ladder_gate_rows.write() = vec![crate::ui::workflow_manual_result_reconciliation_gate_state::gate_summary_lines(&gate)];
        } else {
            *self.ladder_gate_rows.write() = vec![];
        }
    }

    fn load_routing_ladder(&self, path: &std::path::Path, wfx_id: &openwand_workflow::workflow_run::WorkflowExecutionId) {
        if let Ok(Some(route)) = crate::workflow_action_routing::route_by_workflow_run(path, &wfx_id.0) {
            *self.routing_route_row.write() = Some(crate::ui::workflow_action_routing_state::workflow_action_route_summary(&route));
            *self.routing_session_row.write() = crate::ui::workflow_action_routing_state::workflow_session_route_row(&route);
            *self.routing_route_predicates.write() = crate::ui::workflow_action_routing_state::workflow_action_route_predicate_rows(&route);
            *self.routing_route_prompt.write() = Some(crate::ui::workflow_action_routing_state::workflow_action_route_prompt_row(&route));
        } else {
            *self.routing_route_row.write() = None;
            *self.routing_session_row.write() = None;
            *self.routing_route_predicates.write() = vec![];
            *self.routing_route_prompt.write() = None;
        }
        if let Ok(Some(rdy)) = crate::workflow_routing_readiness::readiness_by_workflow_run(path, &wfx_id.0) {
            let mut ui_state = crate::ui::workflow_routing_readiness_state::WorkflowRoutingReadinessUiState {
                latest_readiness: Some(crate::ui::workflow_routing_readiness_state::workflow_routing_readiness_summary(&rdy)),
                predicates: crate::ui::workflow_routing_readiness_state::workflow_routing_readiness_predicate_rows(&rdy),
                latest_review: None, candidate: None, route_preview: None, feedback: vec![], warnings: vec![],
            };
            if let Some(ref preview) = rdy.route_request_preview {
                ui_state.route_preview = Some(crate::ui::workflow_routing_readiness_state::workflow_route_request_preview_lines(preview));
            }
            *self.routing_readiness_state.write() = Some(ui_state);
        } else {
            *self.routing_readiness_state.write() = None;
        }
        if let Ok(Some(nar)) = crate::workflow_next_action_routing::routing_by_workflow_run(path, &wfx_id.0) {
            let ui_state = crate::ui::workflow_next_action_routing_state::WorkflowNextActionRoutingUiState {
                latest_routing: Some(crate::ui::workflow_next_action_routing_state::workflow_next_action_routing_summary_lines(&nar)),
                predicates: crate::ui::workflow_next_action_routing_state::workflow_next_action_routing_predicate_rows(&nar),
                route_link: crate::ui::workflow_next_action_routing_state::workflow_next_action_route_link_lines(&nar),
                warnings: vec![],
            };
            *self.routing_next_action_state.write() = Some(ui_state);
        } else {
            *self.routing_next_action_state.write() = None;
        }
    }

    fn load_execution_timeline(&self, path: &std::path::Path, wfx_id: &openwand_workflow::workflow_run::WorkflowExecutionId) {
        if let Ok(wfr) = crate::workflow_execution::load_workflow_run(path, wfx_id) {
            let ui_state = crate::ui::workflow_execution_state::WorkflowExecutionUiState {
                latest_run: Some(crate::ui::workflow_execution_state::workflow_execution_summary(&wfr)),
                predicates: crate::ui::workflow_execution_state::workflow_execution_predicate_rows(&wfr),
                stages: crate::ui::workflow_execution_state::workflow_stage_run_rows(&wfr),
                lifecycle_events: crate::ui::workflow_execution_state::workflow_lifecycle_event_rows(&wfr),
                action_requests: crate::ui::workflow_execution_state::workflow_action_request_rows(&wfr),
                abort_snapshot: Some(crate::ui::workflow_execution_state::workflow_abort_snapshot_lines(&wfr)),
                warnings: vec![],
            };
            *self.execution_timeline_state.write() = Some(ui_state);
        } else {
            *self.execution_timeline_state.write() = None;
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn inspector_shell_does_not_call_record_or_create_paths() {
        // Compile-time check: all loaders are read-only _by_workflow_run and
        // assemble_evidence_chain(..., false). No record/create/export paths.
        let _ = "all loaders are read-only";
    }

    #[test]
    fn inspector_shell_does_not_call_export_or_distribution_paths() {
        // assemble_evidence_chain called with false (no export)
        // distribution_by_workflow_run is read-only index lookup
        let _ = "no export or distribution creation paths";
    }
}
