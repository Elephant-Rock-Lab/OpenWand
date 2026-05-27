use serde::{Deserialize, Serialize};

use crate::ids::{ClaimId, EntityId, EpisodeId};
use crate::snapshots::GateResultSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEvent {
    EpisodeRecorded {
        episode_id: EpisodeId,
        episode_kind: String,
        text_hash: String,
    },
    EntityCreated {
        entity_id: EntityId,
        kind: String,
        name: String,
        canonical_key: String,
    },
    EntityMerged {
        survivor_id: EntityId,
        absorbed_id: EntityId,
        reason: String,
    },
    EntitySummaryUpdated {
        entity_id: EntityId,
        summary_hash: String,
    },
    FactExtracted {
        claim_id: ClaimId,
        statement: String,
        confidence: f64,
        predicate: String,
    },
    FactAccepted {
        claim_id: ClaimId,
        gate_summary: Vec<GateResultSnapshot>,
    },
    FactRejected {
        claim_id: ClaimId,
        reason: String,
    },
    FactInvalidated {
        claim_id: ClaimId,
        replaced_by: Option<ClaimId>,
        reason: Option<String>,
    },
    FactRefined {
        claim_id: ClaimId,
        new_confidence: f64,
        new_statement_hash: String,
    },
    DecisionExtracted {
        claim_id: ClaimId,
        title: String,
        chosen_option: String,
        rejected_count: u8,
    },
    DecisionAccepted {
        claim_id: ClaimId,
    },
    DecisionSuperseded {
        old_claim_id: ClaimId,
        new_claim_id: ClaimId,
    },
    ChunkCreated {
        chunk_id: String,
        source_kind: String,
        source_id: String,
    },
    ChunkUpdated {
        chunk_id: String,
        embedding_model: Option<String>,
    },
}

impl MemoryEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::EpisodeRecorded { .. } => "memory.episode_recorded",
            Self::EntityCreated { .. } => "memory.entity_created",
            Self::EntityMerged { .. } => "memory.entity_merged",
            Self::EntitySummaryUpdated { .. } => "memory.entity_summary_updated",
            Self::FactExtracted { .. } => "memory.fact_extracted",
            Self::FactAccepted { .. } => "memory.fact_accepted",
            Self::FactRejected { .. } => "memory.fact_rejected",
            Self::FactInvalidated { .. } => "memory.fact_invalidated",
            Self::FactRefined { .. } => "memory.fact_refined",
            Self::DecisionExtracted { .. } => "memory.decision_extracted",
            Self::DecisionAccepted { .. } => "memory.decision_accepted",
            Self::DecisionSuperseded { .. } => "memory.decision_superseded",
            Self::ChunkCreated { .. } => "memory.chunk_created",
            Self::ChunkUpdated { .. } => "memory.chunk_updated",
        }
    }
}
