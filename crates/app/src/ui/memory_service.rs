//! Memory UI service — bridges coordinator output to Dioxus UI.
//!
//! Pure format conversion: PromptInputResult → UiFilteredMemoryPanel.
//! No store access. No re-classification. One memory reality.

use crate::memory_coordinator::PromptInputResult;
use crate::ui::memory_dto::{
    UiFilteredMemoryPanel, UiMemoryPanelConflict, UiMemoryPanelRow, UiMemoryPanelSummary,
};
use openwand_memory::panel_view::RepoFilteredPanelView;

/// Build a filtered memory panel from coordinator output.
///
/// This is the ONLY way to build a panel view for the UI.
/// No store queries. No re-classification.
pub fn build_filtered_panel(result: &PromptInputResult) -> UiFilteredMemoryPanel {
    let view = RepoFilteredPanelView::from_coordinator_output(
        result.source_working_directory.clone(),
        &result.report,
        &result.inputs,
    );

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
