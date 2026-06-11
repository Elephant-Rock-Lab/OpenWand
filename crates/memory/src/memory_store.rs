//! Memory write store — accepts, persists, and retrieves memory records.
//!
//! The write store is the authority for accepted memories.
//! Acceptance requires deterministic rules, not just LLM output.

use crate::retrieval::RankedRetrievalContext;
use crate::supersession::RetrievalMode;
use crate::types::{CandidateMemory, MemoryEpisode, MemoryRecord};
use crate::{MemoryError, MemoryQuery, RetrievalContext};
use async_trait::async_trait;

/// Full memory store interface — both read and write.
/// The projection layer writes episodes; the acceptance layer writes records.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    // ── Episode projection ──

    /// Project an episode from a trace event. Idempotent by source_trace_id.
    async fn project_episode(&self, episode: MemoryEpisode) -> Result<(), MemoryError>;

    /// Get all episodes for a session.
    async fn get_episodes(&self, session_id: &str) -> Result<Vec<MemoryEpisode>, MemoryError>;

    // ── Memory acceptance ──

    /// Accept a candidate memory with deterministic rules.
    /// Returns the accepted record, or None if rejected.
    ///
    /// Acceptance rules:
    /// - confidence must be >= threshold (0.7 default)
    /// - source_episode_ids must not be empty
    /// - claim must not be empty
    /// - duplicate claims attach new source episode instead of creating a new record
    async fn accept_candidate(
        &self,
        candidate: CandidateMemory,
    ) -> Result<Option<MemoryRecord>, MemoryError>;

    /// Supersede an existing fact with a new one.
    /// The old record keeps its row but gets `superseded_by` set.
    async fn supersede_record(
        &self,
        old_record_id: &str,
        new_claim: String,
    ) -> Result<MemoryRecord, MemoryError>;

    // ── Retrieval ──

    /// Search active memory records by keyword.
    async fn search_records(&self, query: MemoryQuery) -> Result<RetrievalContext, MemoryError>;

    /// Search with ranked retrieval and evidence-aware mode.
    async fn search_ranked(
        &self,
        query: MemoryQuery,
        mode: RetrievalMode,
    ) -> Result<RankedRetrievalContext, MemoryError>;

    /// Get all active records (for debugging/testing).
    async fn list_active_records(&self) -> Result<Vec<MemoryRecord>, MemoryError>;
}
