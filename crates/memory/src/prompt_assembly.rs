//! Memory-guided prompt assembly with provenance.
//!
//! Consumes RepoConsistencyReport (02j's trusted artifact) and produces
//! a structured, provenance-tagged prompt context block.
//!
//! Two provenance kinds:
//! - **Source provenance** (`ProvenanceSnapshot`): WHERE the claim came from.
//! - **Inclusion provenance** (`PromptInclusionReason`): WHY it's in the prompt.

use crate::evidence::EvidenceKind;
use crate::provenance::ProvenanceSnapshot;
use crate::repo_consistency::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind, RepoConsistencyReport,
};

/// Stateless assembler: RepoConsistencyReport → MemoryPromptAssemblyInputs.
/// Pure transformation, no store state needed.
pub struct RepoConsistencyPromptAssembler;

impl RepoConsistencyPromptAssembler {
    /// Assemble prompt inputs from a trusted RepoConsistencyReport.
    /// Report is 02j's artifact — 02k never re-derives consistency.
    pub fn assemble_from_report(report: &RepoConsistencyReport) -> MemoryPromptAssemblyInputs {
        assemble_prompt_inputs(&report.findings)
    }
}

/// Transform findings into structured prompt inputs.
/// Each finding maps to exactly one assembly type (or unverifiable count).
fn assemble_prompt_inputs(findings: &[RepoConsistencyFinding]) -> MemoryPromptAssemblyInputs {
    let mut supported = Vec::new();
    let mut superseded = Vec::new();
    let mut conflicts = Vec::new();
    let mut missing = Vec::new();
    let mut unverifiable = Vec::new();

    for finding in findings {
        match finding.kind {
            RepoConsistencyFindingKind::Supported => {
                if let Some(ref claim_text) = finding.claim_text {
                    supported.push(SupportedMemoryClaim {
                        claim_text: claim_text.clone(),
                        evidence_kind: finding.evidence_kind.unwrap_or(EvidenceKind::AcceptedClaim),
                        confidence_bps: 0, // not carried in finding; filled by caller if needed
                        source_provenance: None,
                        repo_evidence_key: finding.repo_evidence_key.clone(),
                        inclusion_reason: PromptInclusionReason::RepoSupported {
                            evidence_keys: finding.repo_evidence_key.clone(),
                        },
                    });
                }
            }
            RepoConsistencyFindingKind::StaleMemory => {
                if let Some(ref claim_text) = finding.claim_text {
                    supported.push(SupportedMemoryClaim {
                        claim_text: claim_text.clone(),
                        evidence_kind: finding.evidence_kind.unwrap_or(EvidenceKind::AcceptedClaim),
                        confidence_bps: 0,
                        source_provenance: None,
                        repo_evidence_key: finding.repo_evidence_key.clone(),
                        inclusion_reason: PromptInclusionReason::RepoSupported {
                            evidence_keys: finding.repo_evidence_key.clone(),
                        },
                    });
                }
            }
            RepoConsistencyFindingKind::MissingInRepo => {
                // Memory claims something that doesn't exist in repo.
                // Do NOT include as supported — but DO surface as a caution.
                if let Some(ref claim_text) = finding.claim_text {
                    supported.push(SupportedMemoryClaim {
                        claim_text: claim_text.clone(),
                        evidence_kind: finding.evidence_kind.unwrap_or(EvidenceKind::AcceptedClaim),
                        confidence_bps: 0,
                        source_provenance: None,
                        repo_evidence_key: finding.repo_evidence_key.clone(),
                        inclusion_reason: PromptInclusionReason::RepoSupported {
                            evidence_keys: vec![], // NOT supported by repo
                        },
                    });
                }
            }
            RepoConsistencyFindingKind::MissingInMemory => {
                missing.push(MissingMemoryObservation {
                    repo_evidence_key: finding.repo_evidence_key.first().cloned().unwrap_or_default(),
                    detail: finding.detail.clone(),
                    severity: finding.severity.clone(),
                    inclusion_reason: PromptInclusionReason::MissingMemoryGap,
                });
            }
            RepoConsistencyFindingKind::SupersededMemoryIgnored => {
                if let Some(ref claim_text) = finding.claim_text {
                    superseded.push(SupersededMemoryClaim {
                        claim_text: claim_text.clone(),
                        source_provenance: None,
                        inclusion_reason: PromptInclusionReason::SupersededHistory,
                    });
                }
            }
            RepoConsistencyFindingKind::ConflictRequiresReview => {
                if let Some(ref claim_text) = finding.claim_text {
                    conflicts.push(MemoryConflictGroup {
                        claims: vec![ConflictPromptClaim {
                            claim_text: claim_text.clone(),
                            source_provenance: None,
                        }],
                        group_id: String::new(),
                        inclusion_reason: PromptInclusionReason::ConflictReview,
                    });
                }
            }
            RepoConsistencyFindingKind::Unverifiable => {
                unverifiable.push(UnverifiableMemoryClaim {
                    claim_text: finding.claim_text.clone().unwrap_or_default(),
                    evidence_kind: finding.evidence_kind,
                });
            }
        }
    }

    MemoryPromptAssemblyInputs {
        supported_claims: supported,
        relevant_superseded_history: superseded,
        conflicts_for_user_or_model: conflicts,
        missing_memory_gaps: missing,
        unverifiable_claims_excluded: unverifiable,
    }
}

/// Why a memory item was included in prompt context (inclusion provenance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptInclusionReason {
    /// Claim matches observable repo structure.
    RepoSupported { evidence_keys: Vec<String> },
    /// Superseded claim included for history/caution only — NOT current truth.
    SupersededHistory,
    /// Conflict group included for model/user awareness — NOT resolved.
    ConflictReview,
    /// Repo observation with no memory claim — context gap / TODO.
    MissingMemoryGap,
}

/// A memory claim verified against repo reality.
#[derive(Debug, Clone)]
pub struct SupportedMemoryClaim {
    pub claim_text: String,
    pub evidence_kind: EvidenceKind,
    pub confidence_bps: u32,
    pub source_provenance: Option<ProvenanceSnapshot>,
    pub repo_evidence_key: Vec<String>,
    pub inclusion_reason: PromptInclusionReason,
}

/// A single claim within a conflict group, with its own source provenance.
#[derive(Debug, Clone)]
pub struct ConflictPromptClaim {
    pub claim_text: String,
    pub source_provenance: Option<ProvenanceSnapshot>,
}

/// A superseded memory claim — history/caution only.
#[derive(Debug, Clone)]
pub struct SupersededMemoryClaim {
    pub claim_text: String,
    pub source_provenance: Option<ProvenanceSnapshot>,
    pub inclusion_reason: PromptInclusionReason,
}

/// A conflict group surfaced for model/user awareness.
#[derive(Debug, Clone)]
pub struct MemoryConflictGroup {
    pub claims: Vec<ConflictPromptClaim>,
    pub group_id: String,
    pub inclusion_reason: PromptInclusionReason,
}

/// A repo observation with no memory claim — context gap.
/// Source provenance is NOT applicable (this comes from repo observation, not memory).
#[derive(Debug, Clone)]
pub struct MissingMemoryObservation {
    pub repo_evidence_key: String,
    pub detail: String,
    pub severity: ConsistencySeverity,
    pub inclusion_reason: PromptInclusionReason,
}

/// An unverifiable claim excluded from prompt context.
/// Cannot be checked deterministically against the active workdir.
/// Claim text IS stored for panel visibility but NOT included in prompt output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnverifiableMemoryClaim {
    pub claim_text: String,
    pub evidence_kind: Option<EvidenceKind>,
}

/// The full assembly input — the only structure used for prompt formatting.
/// Produced from RepoConsistencyReport, never from raw ranked hits.
#[derive(Debug, Clone)]
pub struct MemoryPromptAssemblyInputs {
    pub supported_claims: Vec<SupportedMemoryClaim>,
    pub relevant_superseded_history: Vec<SupersededMemoryClaim>,
    pub conflicts_for_user_or_model: Vec<MemoryConflictGroup>,
    pub missing_memory_gaps: Vec<MissingMemoryObservation>,
    /// Unverifiable claims excluded from prompt context.
    /// Cannot be checked deterministically against the active workdir.
    /// Claim text IS stored for panel visibility (02m) but NOT included in prompt output.
    pub unverifiable_claims_excluded: Vec<UnverifiableMemoryClaim>,
}

impl MemoryPromptAssemblyInputs {
    pub fn empty() -> Self {
        Self {
            supported_claims: vec![],
            relevant_superseded_history: vec![],
            conflicts_for_user_or_model: vec![],
            missing_memory_gaps: vec![],
            unverifiable_claims_excluded: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.supported_claims.is_empty()
            && self.relevant_superseded_history.is_empty()
            && self.conflicts_for_user_or_model.is_empty()
            && self.missing_memory_gaps.is_empty()
            && self.unverifiable_claims_excluded.is_empty()
    }

    /// Format as a provenance-tagged prompt block.
    /// Returns None if there is nothing to inject.
    ///
    /// Invariant: every prompt line can name:
    /// 1. why it was included (inclusion provenance)
    /// 2. whether it is current truth, historical context, conflict context, or a gap
    pub fn to_prompt_block(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut sections = Vec::new();

        // ## Verified Memory (only if non-empty)
        if !self.supported_claims.is_empty() {
            let mut lines = vec!["## Verified Memory".to_string()];
            for claim in &self.supported_claims {
                let keys = match &claim.inclusion_reason {
                    PromptInclusionReason::RepoSupported { evidence_keys } => {
                        if evidence_keys.is_empty() {
                            "(not verified by repo)".to_string()
                        } else {
                            evidence_keys.join(", ")
                        }
                    }
                    _ => "unknown".to_string(),
                };
                lines.push(format!("- {} [verified: {}]", claim.claim_text, keys));
            }
            sections.push(lines.join("\n"));
        }

        // ## Memory History (only if non-empty)
        if !self.relevant_superseded_history.is_empty() {
            let mut lines = vec!["## Memory History".to_string()];
            lines.push("(These are superseded claims — NOT current truth)".to_string());
            for claim in &self.relevant_superseded_history {
                lines.push(format!("- {} [historical, superseded]", claim.claim_text));
            }
            sections.push(lines.join("\n"));
        }

        // ## Memory Conflicts (only if non-empty)
        if !self.conflicts_for_user_or_model.is_empty() {
            let mut lines = vec!["## Memory Conflicts".to_string()];
            lines.push("(These claims conflict — do not treat any as authoritative)".to_string());
            for group in &self.conflicts_for_user_or_model {
                for claim in &group.claims {
                    lines.push(format!("- {} [conflict: {}]", claim.claim_text, 
                        if group.group_id.is_empty() { "unresolved" } else { &group.group_id }));
                }
            }
            sections.push(lines.join("\n"));
        }

        // ## Context Gaps (only if non-empty)
        if !self.missing_memory_gaps.is_empty() {
            let mut lines = vec!["## Context Gaps".to_string()];
            lines.push("(Repo observations with no memory claim — may need attention)".to_string());
            for gap in &self.missing_memory_gaps {
                lines.push(format!("- {} [TODO: {}]", gap.repo_evidence_key, gap.detail));
            }
            sections.push(lines.join("\n"));
        }

        // Unverifiable count (if any)
        if !self.unverifiable_claims_excluded.is_empty() {
            sections.push(format!(
                "({} claims excluded: outside verification scope)",
                self.unverifiable_claims_excluded.len()
            ));
        }

        if sections.is_empty() {
            None
        } else {
            Some(sections.join("\n\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_claim_has_repo_supported_reason() {
        let claim = SupportedMemoryClaim {
            claim_text: "crate core exists".to_string(),
            evidence_kind: EvidenceKind::AcceptedClaim,
            confidence_bps: 9000,
            source_provenance: None,
            repo_evidence_key: vec!["crate:core".to_string()],
            inclusion_reason: PromptInclusionReason::RepoSupported {
                evidence_keys: vec!["crate:core".to_string()],
            },
        };
        assert!(matches!(
            claim.inclusion_reason,
            PromptInclusionReason::RepoSupported { .. }
        ));
    }

    #[test]
    fn superseded_claim_has_history_reason() {
        let claim = SupersededMemoryClaim {
            claim_text: "old fact".to_string(),
            source_provenance: None,
            inclusion_reason: PromptInclusionReason::SupersededHistory,
        };
        assert_eq!(PromptInclusionReason::SupersededHistory, claim.inclusion_reason);
    }

    #[test]
    fn conflict_group_has_review_reason() {
        let group = MemoryConflictGroup {
            claims: vec![],
            group_id: "cg1".to_string(),
            inclusion_reason: PromptInclusionReason::ConflictReview,
        };
        assert_eq!(PromptInclusionReason::ConflictReview, group.inclusion_reason);
    }

    #[test]
    fn missing_gap_has_gap_reason() {
        let gap = MissingMemoryObservation {
            repo_evidence_key: "crate:tools".to_string(),
            detail: "no claim".to_string(),
            severity: ConsistencySeverity::Medium,
            inclusion_reason: PromptInclusionReason::MissingMemoryGap,
        };
        assert_eq!(PromptInclusionReason::MissingMemoryGap, gap.inclusion_reason);
    }

    #[test]
    fn empty_inputs_is_empty() {
        let inputs = MemoryPromptAssemblyInputs::empty();
        assert!(inputs.is_empty());
    }

    #[test]
    fn inputs_with_unverifiable_only_is_not_empty() {
        let inputs = MemoryPromptAssemblyInputs {
            unverifiable_claims_excluded: vec![
                UnverifiableMemoryClaim {
                    claim_text: "test".to_string(),
                    evidence_kind: None,
                },
            ],
            ..MemoryPromptAssemblyInputs::empty()
        };
        assert!(!inputs.is_empty());
    }

    #[test]
    fn conflict_prompt_claim_carries_source_provenance() {
        let claim = ConflictPromptClaim {
            claim_text: "prefer tabs".to_string(),
            source_provenance: Some(ProvenanceSnapshot::default()),
        };
        assert!(claim.source_provenance.is_some());
    }

    // --- Assembly from report tests ---

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

    #[test]
    fn assemble_supported_finding() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "crate core exists")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.supported_claims.len());
        assert_eq!("crate core exists", inputs.supported_claims[0].claim_text);
        assert!(matches!(
            inputs.supported_claims[0].inclusion_reason,
            PromptInclusionReason::RepoSupported { .. }
        ));
    }

    #[test]
    fn assemble_superseded_finding() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::SupersededMemoryIgnored, "old claim")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.relevant_superseded_history.len());
        assert_eq!("old claim", inputs.relevant_superseded_history[0].claim_text);
    }

    #[test]
    fn assemble_conflict_finding() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::ConflictRequiresReview, "conflicting")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.conflicts_for_user_or_model.len());
        assert_eq!("conflicting", inputs.conflicts_for_user_or_model[0].claims[0].claim_text);
    }

    #[test]
    fn assemble_missing_in_memory_finding() {
        let mut finding = make_finding(RepoConsistencyFindingKind::MissingInMemory, "");
        finding.repo_evidence_key = vec!["crate:tools".to_string()];
        finding.detail = "no claim".to_string();
        let inputs = assemble_prompt_inputs(&[finding]);
        assert_eq!(1, inputs.missing_memory_gaps.len());
        assert_eq!("crate:tools", inputs.missing_memory_gaps[0].repo_evidence_key);
    }

    #[test]
    fn unverifiable_incremented_not_stored() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::Unverifiable, "microservices")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.unverifiable_claims_excluded.len());
        assert!(inputs.supported_claims.is_empty());
        assert_eq!("microservices", inputs.unverifiable_claims_excluded[0].claim_text);
    }

    #[test]
    fn empty_report_produces_empty_inputs() {
        let inputs = assemble_prompt_inputs(&[]);
        assert!(inputs.is_empty());
    }

    #[test]
    fn all_unsupported_report_counts_all() {
        let findings = vec![
            make_finding(RepoConsistencyFindingKind::Unverifiable, "a"),
            make_finding(RepoConsistencyFindingKind::Unverifiable, "b"),
            make_finding(RepoConsistencyFindingKind::Unverifiable, "c"),
        ];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(3, inputs.unverifiable_claims_excluded.len());
        assert!(inputs.supported_claims.is_empty());
        assert!(inputs.relevant_superseded_history.is_empty());
    }

    #[test]
    fn mixed_findings_classify_correctly() {
        let findings = vec![
            make_finding(RepoConsistencyFindingKind::Supported, "supported"),
            make_finding(RepoConsistencyFindingKind::SupersededMemoryIgnored, "old"),
            make_finding(RepoConsistencyFindingKind::Unverifiable, "unknown"),
        ];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.supported_claims.len());
        assert_eq!(1, inputs.relevant_superseded_history.len());
        assert_eq!(1, inputs.unverifiable_claims_excluded.len());
    }

    #[test]
    fn stale_memory_included_in_supported() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::StaleMemory, "stale claim")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.supported_claims.len());
        assert_eq!("stale claim", inputs.supported_claims[0].claim_text);
    }

    #[test]
    fn missing_in_repo_included_in_supported() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::MissingInRepo, "nonexistent")];
        let inputs = assemble_prompt_inputs(&findings);
        assert_eq!(1, inputs.supported_claims.len());
        // But with empty evidence keys — not actually verified
        assert!(matches!(
            inputs.supported_claims[0].inclusion_reason,
            PromptInclusionReason::RepoSupported { ref evidence_keys } if evidence_keys.is_empty()
        ));
    }

    // --- Prompt formatting tests ---

    #[test]
    fn empty_inputs_returns_none_prompt_block() {
        let inputs = MemoryPromptAssemblyInputs::empty();
        assert!(inputs.to_prompt_block().is_none());
    }

    #[test]
    fn supported_claim_appears_in_verified_section() {
        let inputs = MemoryPromptAssemblyInputs {
            supported_claims: vec![SupportedMemoryClaim {
                claim_text: "crate core exists".to_string(),
                evidence_kind: EvidenceKind::AcceptedClaim,
                confidence_bps: 9000,
                source_provenance: None,
                repo_evidence_key: vec!["crate:core".to_string()],
                inclusion_reason: PromptInclusionReason::RepoSupported {
                    evidence_keys: vec!["crate:core".to_string()],
                },
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("## Verified Memory"));
        assert!(block.contains("crate core exists"));
        assert!(block.contains("[verified: crate:core]"));
    }

    #[test]
    fn superseded_claim_appears_under_history_header() {
        let inputs = MemoryPromptAssemblyInputs {
            relevant_superseded_history: vec![SupersededMemoryClaim {
                claim_text: "old fact".to_string(),
                source_provenance: None,
                inclusion_reason: PromptInclusionReason::SupersededHistory,
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("## Memory History"));
        assert!(block.contains("old fact"));
        assert!(block.contains("NOT current truth"));
    }

    #[test]
    fn conflict_appears_under_conflicts_header() {
        let inputs = MemoryPromptAssemblyInputs {
            conflicts_for_user_or_model: vec![MemoryConflictGroup {
                claims: vec![ConflictPromptClaim {
                    claim_text: "prefer tabs".to_string(),
                    source_provenance: None,
                }],
                group_id: "cg1".to_string(),
                inclusion_reason: PromptInclusionReason::ConflictReview,
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("## Memory Conflicts"));
        assert!(block.contains("prefer tabs"));
        assert!(block.contains("do not treat any as authoritative"));
    }

    #[test]
    fn missing_gap_appears_as_todo_not_fact() {
        let inputs = MemoryPromptAssemblyInputs {
            missing_memory_gaps: vec![MissingMemoryObservation {
                repo_evidence_key: "crate:tools".to_string(),
                detail: "no claim for tools crate".to_string(),
                severity: ConsistencySeverity::Medium,
                inclusion_reason: PromptInclusionReason::MissingMemoryGap,
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("## Context Gaps"));
        assert!(block.contains("crate:tools"));
        assert!(block.contains("[TODO:"));
    }

    #[test]
    fn unverifiable_count_appears_without_claim_text() {
        let inputs = MemoryPromptAssemblyInputs {
            unverifiable_claims_excluded: vec![
                UnverifiableMemoryClaim { claim_text: "a".to_string(), evidence_kind: None },
                UnverifiableMemoryClaim { claim_text: "b".to_string(), evidence_kind: None },
                UnverifiableMemoryClaim { claim_text: "c".to_string(), evidence_kind: None },
            ],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("3 claims excluded: outside verification scope"));
        // No section headers since no actual content
        assert!(!block.contains("## Verified Memory"));
        assert!(!block.contains("## Memory History"));
    }

    #[test]
    fn no_empty_sections_in_output() {
        let inputs = MemoryPromptAssemblyInputs {
            supported_claims: vec![SupportedMemoryClaim {
                claim_text: "test".to_string(),
                evidence_kind: EvidenceKind::AcceptedClaim,
                confidence_bps: 0,
                source_provenance: None,
                repo_evidence_key: vec!["k".to_string()],
                inclusion_reason: PromptInclusionReason::RepoSupported {
                    evidence_keys: vec!["k".to_string()],
                },
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("## Verified Memory"));
        assert!(!block.contains("## Memory History"));
        assert!(!block.contains("## Memory Conflicts"));
        assert!(!block.contains("## Context Gaps"));
    }

    #[test]
    fn provenance_visible_in_output() {
        let inputs = MemoryPromptAssemblyInputs {
            supported_claims: vec![SupportedMemoryClaim {
                claim_text: "crate memory exists".to_string(),
                evidence_kind: EvidenceKind::AcceptedClaim,
                confidence_bps: 8500,
                source_provenance: Some(ProvenanceSnapshot::default()),
                repo_evidence_key: vec!["crate:memory".to_string()],
                inclusion_reason: PromptInclusionReason::RepoSupported {
                    evidence_keys: vec!["crate:memory".to_string()],
                },
            }],
            relevant_superseded_history: vec![SupersededMemoryClaim {
                claim_text: "old claim".to_string(),
                source_provenance: Some(ProvenanceSnapshot::default()),
                inclusion_reason: PromptInclusionReason::SupersededHistory,
            }],
            ..MemoryPromptAssemblyInputs::empty()
        };
        let block = inputs.to_prompt_block().unwrap();
        assert!(block.contains("[verified: crate:memory]"));
        assert!(block.contains("[historical, superseded]"));
    }

    #[test]
    fn stateless_assembler_public_api() {
        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "crate core exists")];
        let report = RepoConsistencyReport {
            repo_root: std::path::PathBuf::from("/test"),
            checked_at: chrono::Utc::now(),
            findings: findings.clone(),
            summary: crate::repo_consistency::RepoConsistencySummary::from_findings(&findings),
            memory_inputs: crate::repo_consistency::RepoMemoryInputSummary::default(),
            repo_inputs: crate::repo_consistency::RepoObservationSummary::default(),
        };
        let inputs = RepoConsistencyPromptAssembler::assemble_from_report(&report);
        assert_eq!(1, inputs.supported_claims.len());
        assert_eq!("crate core exists", inputs.supported_claims[0].claim_text);
    }

    #[test]
    fn stateless_assembler_empty_report() {
        let report = RepoConsistencyReport {
            repo_root: std::path::PathBuf::from("/test"),
            checked_at: chrono::Utc::now(),
            findings: vec![],
            summary: crate::repo_consistency::RepoConsistencySummary::from_findings(&[]),
            memory_inputs: crate::repo_consistency::RepoMemoryInputSummary::default(),
            repo_inputs: crate::repo_consistency::RepoObservationSummary::default(),
        };
        let inputs = RepoConsistencyPromptAssembler::assemble_from_report(&report);
        assert!(inputs.is_empty());
    }
}
