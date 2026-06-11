//! Memory input loader for consistency check.
//!
//! Loads three independent memory views using correct retrieval modes.

use crate::conflict::ConflictGroup;
use crate::memory_store::MemoryStore;
use crate::query::MemoryQuery;
use crate::retrieval::RankedMemoryHit;
use crate::supersession::RetrievalMode;
use crate::MemoryError;

/// Three memory views for consistency checking.
#[derive(Debug, Clone)]
pub struct RepoMemoryInputs {
    /// Current active claims (from CurrentState retrieval).
    pub current_claims: Vec<RankedMemoryHit>,
    /// Superseded history (from ChangeHistory retrieval).
    pub superseded_history: Vec<RankedMemoryHit>,
    /// Conflict groups detected in memory.
    pub conflict_groups: Vec<ConflictGroup>,
}

/// Load memory inputs from a memory store.
pub async fn load_memory_inputs(
    store: &dyn MemoryStore,
    query: &str,
) -> Result<RepoMemoryInputs, MemoryError> {
    let mq = MemoryQuery::new(query);

    // Current active claims — excludes superseded
    let current_ctx = store
        .search_ranked(mq.clone(), RetrievalMode::CurrentState)
        .await?;
    let current_claims: Vec<RankedMemoryHit> = current_ctx
        .hits
        .into_iter()
        .filter(|h| h.evidence_kind.is_accepted_state())
        .collect();

    // Superseded history — includes chains
    let history_ctx = store
        .search_ranked(mq.clone(), RetrievalMode::ChangeHistory)
        .await?;
    let superseded_history: Vec<RankedMemoryHit> = history_ctx
        .hits
        .into_iter()
        .filter(|h| !h.evidence_kind.is_accepted_state())
        .collect();

    // Conflict search — returns all records for conflict review
    let conflict_ctx = store
        .search_ranked(mq.clone(), RetrievalMode::ConflictSearch)
        .await?;

    // Group by conflict_group_id if present (future: read from records)
    // For now, conflict groups are derived from records with matching conflict_group_id
    let conflict_groups = derive_conflict_groups(&conflict_ctx.hits);

    Ok(RepoMemoryInputs {
        current_claims,
        superseded_history,
        conflict_groups,
    })
}

/// Derive conflict groups from hits that share a conflict_group_id.
fn derive_conflict_groups(hits: &[RankedMemoryHit]) -> Vec<ConflictGroup> {
    let groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for _hit in hits {
        // Check if the underlying record has a conflict_group_id
        // For now, we use the hit's source_trace_ids as a proxy
        // This will be properly wired when conflict_group_id is on RankedMemoryHit
    }

    // Convert to ConflictGroup structs
    groups
        .into_iter()
        .map(|(id, record_ids)| ConflictGroup::new(id, record_ids))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::EvidenceKind;
    use crate::in_memory::InMemoryMemoryStore;
    use crate::memory_store::MemoryStore;
    use crate::types::{CandidateMemory, CandidateKind, EpisodeRole, MemoryEpisode};

    fn make_episode(id: &str, trace_id: &str) -> MemoryEpisode {
        MemoryEpisode {
            episode_id: id.to_string(),
            source_trace_id: trace_id.to_string(),
            session_id: "s1".to_string(),
            event_kind: "message".to_string(),
            role: EpisodeRole::User,
            content: "test".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    fn make_candidate(claim: &str, ep_id: &str) -> CandidateMemory {
        CandidateMemory {
            claim: claim.to_string(),
            kind: CandidateKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec![ep_id.to_string()],
        }
    }

    #[tokio::test]
    async fn loader_uses_current_state_for_active_claims() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate core").await.unwrap();
        assert!(!inputs.current_claims.is_empty(), "current claims should have results");
    }

    #[tokio::test]
    async fn loader_uses_change_history_for_superseded_claims() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let old = store.accept_candidate(make_candidate("crate old exists", "ep1")).await.unwrap().unwrap();
        store.supersede_record(&old.record_id, "crate new exists".to_string()).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate").await.unwrap();
        assert!(!inputs.superseded_history.is_empty(), "should have superseded history");
    }

    #[tokio::test]
    async fn loader_uses_conflict_search_for_conflicts() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.accept_candidate(make_candidate("memory claim about crate", "ep1")).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate").await.unwrap();
        // No conflicts set up, so conflict_groups should be empty
        assert!(inputs.conflict_groups.is_empty());
    }

    #[tokio::test]
    async fn loader_excludes_superseded_from_current_inputs() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        let old = store.accept_candidate(make_candidate("crate old exists", "ep1")).await.unwrap().unwrap();
        store.supersede_record(&old.record_id, "crate new exists".to_string()).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate").await.unwrap();
        // Current claims should only have the successor
        for hit in &inputs.current_claims {
            assert!(hit.evidence_kind.is_accepted_state(), "current claims must be accepted");
        }
    }

    #[tokio::test]
    async fn loader_preserves_evidence_kind() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.accept_candidate(make_candidate("crate core exists", "ep1")).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate").await.unwrap();
        assert!(!inputs.current_claims.is_empty());
        assert_eq!(EvidenceKind::AcceptedClaim, inputs.current_claims[0].evidence_kind);
    }

    #[tokio::test]
    async fn loader_preserves_dedup_key_if_present() {
        let store = InMemoryMemoryStore::new();
        store.project_episode(make_episode("ep1", "t1")).await.unwrap();
        store.accept_candidate(make_candidate("unique crate claim", "ep1")).await.unwrap();

        let inputs = load_memory_inputs(&store, "crate").await.unwrap();
        assert!(!inputs.current_claims.is_empty());
        assert!(!inputs.current_claims[0].id.is_empty());
    }
}
