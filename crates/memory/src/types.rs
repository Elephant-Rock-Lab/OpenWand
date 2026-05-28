//! Memory episode — an immutable snapshot of a trace event
//! relevant to memory extraction.
//!
//! Episodes are projections from trace, not authority.
//! Trace remains the authoritative source. Episodes are rebuildable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evidence::EvidenceKind;

/// An immutable memory episode projected from trace events.
/// Episodes are the input to memory extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEpisode {
    pub episode_id: String,
    pub source_trace_id: String,
    pub session_id: String,
    pub event_kind: String,
    pub role: EpisodeRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// The role of the episode's actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeRole {
    User,
    Assistant,
    Tool,
    System,
}

/// A candidate fact or decision proposed by the extractor.
/// Not yet accepted — must pass deterministic acceptance rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMemory {
    pub claim: String,
    pub kind: CandidateKind,
    pub confidence: f64,
    pub source_episode_ids: Vec<String>,
}

/// The kind of candidate memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateKind {
    Fact,
    Decision,
    Preference,
}

/// An accepted memory record with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub record_id: String,
    pub claim: String,
    pub kind: MemoryKind,
    pub confidence: f64,
    pub source_episode_ids: Vec<String>,
    pub source_trace_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub superseded_by: Option<String>,
    /// What kind of evidence this record represents.
    /// Defaults to AcceptedClaim for legacy records.
    #[serde(default)]
    pub evidence_kind: EvidenceKind,
    /// BLAKE3 hash of normalized claim text.
    #[serde(default)]
    pub normalized_text_hash: String,
}

/// The kind of accepted memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Fact,
    Decision,
    Preference,
}

impl MemoryRecord {
    /// Whether this record is currently active (not superseded or expired).
    pub fn is_active(&self) -> bool {
        if self.superseded_by.is_some() {
            return false;
        }
        if let Some(valid_until) = self.valid_until {
            return valid_until > Utc::now();
        }
        true
    }
}
