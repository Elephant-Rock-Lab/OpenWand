//! Evidence kind semantics for memory records.
//!
//! Every memory record carries an evidence kind that determines whether it
//! can be treated as accepted project state, whether it is observation-only,
//! and how it ranks in retrieval.

use serde::{Deserialize, Serialize};

/// What kind of evidence a memory record represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// Existing normal memory claim — passed deterministic acceptance rules.
    AcceptedClaim,
    /// Explicit user instruction or constraint — highest authority.
    UserStatedClaim,
    /// Tool/test/git output from deterministic source — evidence, not claim.
    DeterministicEvidence,
    /// Shell/tool output with no semantic validation.
    RawObservation,
    /// Non-deterministic extracted candidate — not yet accepted.
    LlmExtractedCandidate,
    /// Previously valid but replaced by a successor.
    SupersededClaim,
    /// Active contradiction exists — not resolved.
    ConflictingClaim,
}

impl Default for EvidenceKind {
    fn default() -> Self {
        Self::AcceptedClaim
    }
}

impl EvidenceKind {
    /// Whether this evidence kind can be treated as accepted project state.
    pub fn is_accepted_state(&self) -> bool {
        matches!(self, Self::AcceptedClaim | Self::UserStatedClaim)
    }

    /// Whether this evidence kind can support a claim (directly or as evidence).
    pub fn can_support_claim(&self) -> bool {
        matches!(
            self,
            Self::AcceptedClaim | Self::UserStatedClaim | Self::DeterministicEvidence
        )
    }

    /// Whether this evidence kind is an observation (never promoted to claim automatically).
    pub fn is_observation(&self) -> bool {
        matches!(self, Self::RawObservation | Self::DeterministicEvidence)
    }

    /// Authority ranking for retrieval scoring (basis points, 0-10000).
    pub fn authority_bps(&self) -> u16 {
        match self {
            Self::UserStatedClaim => 10000,
            Self::DeterministicEvidence => 9000,
            Self::AcceptedClaim => 8000,
            Self::LlmExtractedCandidate => 5000,
            Self::RawObservation => 3000,
            Self::ConflictingClaim => 4000,
            Self::SupersededClaim => 1000,
        }
    }
}
