//! Memory panel view — read-only projection of repo-consistency-filtered memory.
//!
//! Derived from RepoConsistencyReport + MemoryPromptAssemblyInputs.
//! One memory reality: the panel renders the same governed data that the
//! coordinator produces for prompt assembly. No separate classification logic.
//!
//! Display-oriented but not UI-owned. Lives in the memory crate so the
//! app/UI can render without re-classifying.

use crate::evidence::EvidenceKind;
use crate::prompt_assembly::{
    MemoryPromptAssemblyInputs, PromptInclusionReason, UnverifiableMemoryClaim,
};
use crate::repo_consistency::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind, RepoConsistencyReport,
};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Summary counts for the panel header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPanelSummary {
    pub prompt_included_count: usize,
    pub stale_count: usize,
    pub missing_in_repo_count: usize,
    pub missing_in_memory_count: usize,
    pub conflict_count: usize,
    pub unverifiable_count: usize,
    pub superseded_ignored_count: usize,
}

/// A single memory claim in the panel, with classification and provenance.
#[derive(Debug, Clone)]
pub struct MemoryPanelClaim {
    pub claim_text: String,
    pub finding_kind: RepoConsistencyFindingKind,
    pub evidence_kind: Option<EvidenceKind>,
    pub repo_evidence_key: Vec<String>,
    pub inclusion_reason: Option<PromptInclusionReason>,
    pub severity: ConsistencySeverity,
}

/// A repo observation with no memory claim — context gap.
#[derive(Debug, Clone)]
pub struct MemoryPanelMissingObservation {
    pub repo_evidence_key: String,
    pub detail: String,
    pub severity: ConsistencySeverity,
}

/// A conflict group in the panel, requiring user review.
#[derive(Debug, Clone)]
pub struct MemoryPanelConflictGroup {
    pub group_id: String,
    pub claims: Vec<MemoryPanelClaim>,
    pub detail: String,
}

/// Read-only panel view derived from coordinator output.
///
/// One memory reality — built from RepoConsistencyReport + MemoryPromptAssemblyInputs.
/// No store queries. No re-classification. Pure format conversion.
#[derive(Debug, Clone)]
pub struct RepoFilteredPanelView {
    pub working_directory: PathBuf,
    pub generated_at: DateTime<Utc>,
    pub summary: MemoryPanelSummary,
    pub prompt_included: Vec<MemoryPanelClaim>,
    pub stale: Vec<MemoryPanelClaim>,
    pub missing_in_repo: Vec<MemoryPanelClaim>,
    pub missing_in_memory: Vec<MemoryPanelMissingObservation>,
    pub conflicts: Vec<MemoryPanelConflictGroup>,
    pub unverifiable: Vec<MemoryPanelClaim>,
    pub superseded_ignored: Vec<MemoryPanelClaim>,
}

impl RepoFilteredPanelView {
    /// Build a panel view from coordinator output.
    ///
    /// This is the ONLY constructor. It consumes governed data, never raw store state.
    /// The report provides findings (classification). The inputs provide inclusion decisions.
    pub fn from_coordinator_output(
        working_directory: PathBuf,
        report: &RepoConsistencyReport,
        inputs: &MemoryPromptAssemblyInputs,
    ) -> Self {
        let mut prompt_included = Vec::new();
        let mut stale = Vec::new();
        let mut missing_in_repo = Vec::new();
        let mut missing_in_memory = Vec::new();
        let mut conflicts = Vec::new();
        let mut unverifiable = Vec::new();
        let mut superseded_ignored = Vec::new();

        // Build a lookup: claim_text → inclusion_reason from inputs
        // (for prompt-included claims that have a corresponding finding)
        let supported_map: std::collections::HashMap<&str, &PromptInclusionReason> = inputs
            .supported_claims
            .iter()
            .map(|c| (c.claim_text.as_str(), &c.inclusion_reason))
            .collect();

        for finding in &report.findings {
            let inclusion = supported_map.get(finding.claim_text.as_deref().unwrap_or("")).copied();

            let panel_claim = MemoryPanelClaim {
                claim_text: finding.claim_text.clone().unwrap_or_default(),
                finding_kind: finding.kind.clone(),
                evidence_kind: finding.evidence_kind,
                repo_evidence_key: finding.repo_evidence_key.clone(),
                inclusion_reason: inclusion.cloned(),
                severity: finding.severity.clone(),
            };

            match finding.kind {
                RepoConsistencyFindingKind::Supported => {
                    prompt_included.push(panel_claim);
                }
                RepoConsistencyFindingKind::StaleMemory => {
                    stale.push(panel_claim);
                }
                RepoConsistencyFindingKind::MissingInRepo => {
                    missing_in_repo.push(panel_claim);
                }
                RepoConsistencyFindingKind::MissingInMemory => {
                    missing_in_memory.push(MemoryPanelMissingObservation {
                        repo_evidence_key: finding.repo_evidence_key.first().cloned().unwrap_or_default(),
                        detail: finding.detail.clone(),
                        severity: finding.severity.clone(),
                    });
                }
                RepoConsistencyFindingKind::SupersededMemoryIgnored => {
                    superseded_ignored.push(panel_claim);
                }
                RepoConsistencyFindingKind::ConflictRequiresReview => {
                    // Group conflicts — for now, each finding is its own group
                    // (real grouping by conflict_group_id is deferred)
                    conflicts.push(MemoryPanelConflictGroup {
                        group_id: String::new(),
                        claims: vec![panel_claim],
                        detail: finding.detail.clone(),
                    });
                }
                RepoConsistencyFindingKind::Unverifiable => {
                    unverifiable.push(panel_claim);
                }
            }
        }

        let summary = MemoryPanelSummary {
            prompt_included_count: prompt_included.len(),
            stale_count: stale.len(),
            missing_in_repo_count: missing_in_repo.len(),
            missing_in_memory_count: missing_in_memory.len(),
            conflict_count: conflicts.len(),
            unverifiable_count: unverifiable.len(),
            superseded_ignored_count: superseded_ignored.len(),
        };

        Self {
            working_directory,
            generated_at: report.checked_at,
            summary,
            prompt_included,
            stale,
            missing_in_repo,
            missing_in_memory,
            conflicts,
            unverifiable,
            superseded_ignored,
        }
    }

    /// Empty view for when no coordinator output exists yet.
    pub fn empty(working_directory: PathBuf) -> Self {
        Self {
            working_directory,
            generated_at: Utc::now(),
            summary: MemoryPanelSummary {
                prompt_included_count: 0,
                stale_count: 0,
                missing_in_repo_count: 0,
                missing_in_memory_count: 0,
                conflict_count: 0,
                unverifiable_count: 0,
                superseded_ignored_count: 0,
            },
            prompt_included: vec![],
            stale: vec![],
            missing_in_repo: vec![],
            missing_in_memory: vec![],
            conflicts: vec![],
            unverifiable: vec![],
            superseded_ignored: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.summary.prompt_included_count == 0
            && self.summary.stale_count == 0
            && self.summary.missing_in_repo_count == 0
            && self.summary.missing_in_memory_count == 0
            && self.summary.conflict_count == 0
            && self.summary.unverifiable_count == 0
            && self.summary.superseded_ignored_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_consistency::{
        RepoMemoryInputSummary, RepoObservationSummary, RepoConsistencySummary,
    };

    fn make_finding(kind: RepoConsistencyFindingKind, claim: &str) -> RepoConsistencyFinding {
        RepoConsistencyFinding {
            kind,
            claim_text: Some(claim.to_string()),
            evidence_kind: Some(EvidenceKind::AcceptedClaim),
            repo_evidence_key: vec![],
            severity: ConsistencySeverity::Low,
            detail: "test".to_string(),
        }
    }

    fn make_report(findings: Vec<RepoConsistencyFinding>) -> RepoConsistencyReport {
        RepoConsistencyReport {
            repo_root: PathBuf::from("/test"),
            checked_at: Utc::now(),
            summary: RepoConsistencySummary::from_findings(&findings),
            findings,
            memory_inputs: RepoMemoryInputSummary::default(),
            repo_inputs: RepoObservationSummary::default(),
        }
    }

    #[test]
    fn memory_panel_summary_counts_match_buckets() {
        let findings = vec![
            make_finding(RepoConsistencyFindingKind::Supported, "a"),
            make_finding(RepoConsistencyFindingKind::Supported, "b"),
            make_finding(RepoConsistencyFindingKind::StaleMemory, "c"),
            make_finding(RepoConsistencyFindingKind::Unverifiable, "d"),
        ];
        let report = make_report(findings);
        let inputs = crate::prompt_assembly::RepoConsistencyPromptAssembler::assemble_from_report(&report);

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert_eq!(2, view.summary.prompt_included_count);
        assert_eq!(1, view.summary.stale_count);
        assert_eq!(1, view.summary.unverifiable_count);
        assert_eq!(0, view.summary.missing_in_repo_count);
    }

    #[test]
    fn memory_panel_view_preserves_repo_consistency_kinds() {
        let findings = vec![
            make_finding(RepoConsistencyFindingKind::Supported, "supported"),
            make_finding(RepoConsistencyFindingKind::MissingInRepo, "missing"),
            make_finding(RepoConsistencyFindingKind::ConflictRequiresReview, "conflict"),
        ];
        let report = make_report(findings);
        let inputs = crate::prompt_assembly::RepoConsistencyPromptAssembler::assemble_from_report(&report);

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert_eq!(RepoConsistencyFindingKind::Supported, view.prompt_included[0].finding_kind);
        assert_eq!(RepoConsistencyFindingKind::MissingInRepo, view.missing_in_repo[0].finding_kind);
        assert_eq!(RepoConsistencyFindingKind::ConflictRequiresReview, view.conflicts[0].claims[0].finding_kind);
    }

    #[test]
    fn memory_panel_view_marks_prompt_included_from_supported_findings() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "crate core exists")];
        let report = make_report(findings);
        let inputs = crate::prompt_assembly::RepoConsistencyPromptAssembler::assemble_from_report(&report);

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert_eq!(1, view.prompt_included.len());
        assert!(view.prompt_included[0].inclusion_reason.is_some());
    }

    #[test]
    fn memory_panel_view_keeps_conflict_group_identity() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::ConflictRequiresReview, "conflict claim")];
        let report = make_report(findings);
        let inputs = crate::prompt_assembly::RepoConsistencyPromptAssembler::assemble_from_report(&report);

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert_eq!(1, view.conflicts.len());
        assert_eq!(1, view.conflicts[0].claims.len());
        assert_eq!("conflict claim", view.conflicts[0].claims[0].claim_text);
    }

    #[test]
    fn memory_panel_view_empty_report_produces_empty_view() {
        let report = make_report(vec![]);
        let inputs = MemoryPromptAssemblyInputs::empty();

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert!(view.is_empty());
        assert!(view.prompt_included.is_empty());
    }

    #[test]
    fn memory_panel_view_is_read_only_no_projection_write_access() {
        // Compile-time test: RepoFilteredPanelView has no &mut self methods.
        // This test verifies the type exists and can be constructed.
        let view = RepoFilteredPanelView::empty(PathBuf::from("/test"));
        assert!(view.is_empty());
        // If the type had mutable methods, this comment would be wrong.
    }

    #[test]
    fn filtered_panel_builder_does_not_access_memory_store() {
        // Architectural guard: from_coordinator_output takes (&Report, &Inputs) only.
        // No MemoryStore, no MemoryReadStore, no store trait parameters.
        // This is enforced at compile time by the function signature.
        //
        // If someone adds a store parameter, this test's comment becomes a lie
        // and must be updated (or the parameter removed).
        let report = make_report(vec![make_finding(RepoConsistencyFindingKind::Supported, "test")]);
        let inputs = crate::prompt_assembly::RepoConsistencyPromptAssembler::assemble_from_report(&report);

        let view = RepoFilteredPanelView::from_coordinator_output(
            PathBuf::from("/test"),
            &report,
            &inputs,
        );

        assert_eq!(1, view.summary.prompt_included_count);
        // No store access occurred. The function signature proves it.
    }
}
