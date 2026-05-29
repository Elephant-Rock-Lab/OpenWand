//! Memory UI service — bridges coordinator output to Dioxus UI.
//!
//! Pure format conversion: PromptInputResult → UiFilteredMemoryPanel.
//! No store access. No re-classification. One memory reality.

use crate::memory_coordinator::PromptInputResult;
use crate::ui::memory_dto::{
    UiFilteredMemoryPanel, UiMemoryPanelConflict, UiMemoryPanelRow, UiMemoryPanelSummary, UiTraceRelationCounts,
};
use openwand_memory::panel_view::RepoFilteredPanelView;

/// Build a filtered memory panel from coordinator output.
///
/// This is the ONLY way to build a panel view for the UI.
/// No store queries. No re-classification.
pub fn build_filtered_panel(result: &PromptInputResult) -> UiFilteredMemoryPanel {
    // Use hydrated claims constructor when available
    let view = if result.hydrated_claims.is_empty() {
        RepoFilteredPanelView::from_coordinator_output(
            result.source_working_directory.clone(),
            &result.report,
            &result.inputs,
        )
    } else {
        RepoFilteredPanelView::from_hydrated_claims(
            result.source_working_directory.clone(),
            &result.hydrated_claims,
            &result.report,
        )
    };

    let summary = UiMemoryPanelSummary {
        prompt_included: view.summary.prompt_included_count,
        stale: view.summary.stale_count,
        missing_in_repo: view.summary.missing_in_repo_count,
        missing_in_memory: view.summary.missing_in_memory_count,
        conflicts: view.summary.conflict_count,
        unverifiable: view.summary.unverifiable_count,
        superseded_ignored: view.summary.superseded_ignored_count,
    };

    let convert_claim = |c: &openwand_memory::panel_view::MemoryPanelClaim| UiMemoryPanelRow {
        claim: c.claim_text.clone(),
        finding_kind: format!("{:?}", c.finding_kind),
        evidence_kind: c
            .evidence_kind
            .map(|e| format!("{:?}", e))
            .unwrap_or_default(),
        repo_evidence_key: c.repo_evidence_key.clone(),
        inclusion_reason: c.inclusion_reason.as_ref().map(|r| format!("{:?}", r)),
        severity: format!("{:?}", c.severity),
        has_provenance: c.source_provenance.is_some() || c.record_id.is_some(),
        record_id: c.record_id.clone(),
        provenance_label: c.provenance_label.clone(),
        source_traces: c.source_trace_ids.clone(),
        confidence: c.confidence,
        conflict_group_id: c.conflict_group_id.clone(),
        superseded_by: c.superseded_by.clone(),
        hydration_status: format!("{:?}", c.hydration_status),
        trace_lineage_summary: c.trace_lineage_summary.clone(),
        trace_relation_counts: UiTraceRelationCounts {
            derived_from: c.trace_relation_counts.as_ref().map(|t| t.derived_from).unwrap_or(0),
            verifies: c.trace_relation_counts.as_ref().map(|t| t.verifies).unwrap_or(0),
            supersedes: c.trace_relation_counts.as_ref().map(|t| t.supersedes).unwrap_or(0),
            invalidates: c.trace_relation_counts.as_ref().map(|t| t.invalidates).unwrap_or(0),
            refines: c.trace_relation_counts.as_ref().map(|t| t.refines).unwrap_or(0),
            conflicts_with: c.trace_relation_counts.as_ref().map(|t| t.conflicts_with).unwrap_or(0),
            other: c.trace_relation_counts.as_ref().map(|t| t.other).unwrap_or(0),
        },
        trace_lineage_status: c.trace_lineage_status.clone(),
    };

    UiFilteredMemoryPanel {
        working_directory: view.working_directory.display().to_string(),
        generated_at: view.generated_at.timestamp(),
        summary,
        prompt_included: view.prompt_included.iter().map(convert_claim).collect(),
        stale: view.stale.iter().map(convert_claim).collect(),
        missing_in_repo: view.missing_in_repo.iter().map(convert_claim).collect(),
        missing_in_memory: view
            .missing_in_memory
            .iter()
            .map(|m| UiMemoryPanelRow {
                claim: m.repo_evidence_key.clone(),
                finding_kind: "MissingInMemory".to_string(),
                evidence_kind: String::new(),
                repo_evidence_key: vec![m.repo_evidence_key.clone()],
                inclusion_reason: None,
                severity: format!("{:?}", m.severity),
                has_provenance: false,
                record_id: None,
                provenance_label: String::new(),
                source_traces: vec![],
                confidence: None,
                conflict_group_id: None,
                superseded_by: None,
                hydration_status: "Missing".to_string(),
                trace_lineage_summary: None,
                trace_relation_counts: UiTraceRelationCounts::default(),
                trace_lineage_status: None,
            })
            .collect(),
        conflicts: view
            .conflicts
            .iter()
            .map(|g| UiMemoryPanelConflict {
                group_id: g.group_id.clone(),
                claims: g.claims.iter().map(convert_claim).collect(),
                detail: g.detail.clone(),
            })
            .collect(),
        unverifiable: view.unverifiable.iter().map(convert_claim).collect(),
        superseded_ignored: view.superseded_ignored.iter().map(convert_claim).collect(),
    }
}
