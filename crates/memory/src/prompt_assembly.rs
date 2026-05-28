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

/// The full assembly input — the only structure used for prompt formatting.
/// Produced from RepoConsistencyReport, never from raw ranked hits.
#[derive(Debug, Clone)]
pub struct MemoryPromptAssemblyInputs {
    pub supported_claims: Vec<SupportedMemoryClaim>,
    pub relevant_superseded_history: Vec<SupersededMemoryClaim>,
    pub conflicts_for_user_or_model: Vec<MemoryConflictGroup>,
    pub missing_memory_gaps: Vec<MissingMemoryObservation>,
    pub unverifiable_claims_excluded: usize,
}

impl MemoryPromptAssemblyInputs {
    pub fn empty() -> Self {
        Self {
            supported_claims: vec![],
            relevant_superseded_history: vec![],
            conflicts_for_user_or_model: vec![],
            missing_memory_gaps: vec![],
            unverifiable_claims_excluded: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.supported_claims.is_empty()
            && self.relevant_superseded_history.is_empty()
            && self.conflicts_for_user_or_model.is_empty()
            && self.missing_memory_gaps.is_empty()
            && self.unverifiable_claims_excluded == 0
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
            unverifiable_claims_excluded: 3,
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
}
