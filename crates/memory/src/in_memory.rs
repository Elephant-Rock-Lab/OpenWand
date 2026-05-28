//! In-memory memory store for testing and simple use cases.

use crate::dedup::compute_normalized_hash;
use crate::evidence::EvidenceKind;
use crate::extractor::MemoryExtractor;
use crate::memory_store::MemoryStore;
use crate::types::{CandidateMemory, MemoryEpisode, MemoryKind, MemoryRecord};
use crate::{MemoryError, MemoryQuery, RetrievalContext};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;

/// Deterministic acceptance threshold. Candidates below this are rejected.
const CONFIDENCE_THRESHOLD: f64 = 0.7;

/// In-memory implementation of MemoryStore.
pub struct InMemoryMemoryStore {
    episodes: Mutex<HashMap<String, MemoryEpisode>>,
    records: Mutex<HashMap<String, MemoryRecord>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self {
            episodes: Mutex::new(HashMap::new()),
            records: Mutex::new(HashMap::new()),
        }
    }

    /// Run extraction and acceptance on all unprocessed episodes.
    pub async fn extract_and_accept(
        &self,
        extractor: &dyn MemoryExtractor,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let episodes = {
            let eps = self.episodes.lock().unwrap();
            eps.values().cloned().collect::<Vec<_>>()
        };

        let candidates = extractor.extract(&episodes).await;

        let mut accepted = Vec::new();
        for candidate in candidates {
            if let Some(record) = self.accept_candidate(candidate).await? {
                accepted.push(record);
            }
        }

        Ok(accepted)
    }
}

impl Default for InMemoryMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn project_episode(&self, episode: MemoryEpisode) -> Result<(), MemoryError> {
        let mut episodes = self.episodes.lock().unwrap();
        // Idempotent: if episode with same source_trace_id exists, skip
        let exists = episodes
            .values()
            .any(|e| e.source_trace_id == episode.source_trace_id);
        if !exists {
            episodes.insert(episode.episode_id.clone(), episode);
        }
        Ok(())
    }

    async fn get_episodes(&self, session_id: &str) -> Result<Vec<MemoryEpisode>, MemoryError> {
        let episodes = self.episodes.lock().unwrap();
        Ok(episodes
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect())
    }

    async fn accept_candidate(
        &self,
        candidate: CandidateMemory,
    ) -> Result<Option<MemoryRecord>, MemoryError> {
        // Deterministic acceptance rules
        if candidate.confidence < CONFIDENCE_THRESHOLD {
            return Ok(None);
        }
        if candidate.source_episode_ids.is_empty() {
            return Ok(None);
        }
        if candidate.claim.trim().is_empty() {
            return Ok(None);
        }

        let mut records = self.records.lock().unwrap();

        // Check for duplicate using normalized_text_hash — attach source instead of creating new
        let claim_hash = compute_normalized_hash(&candidate.claim);
        for record in records.values_mut() {
            if record.normalized_text_hash == claim_hash && record.is_active() {
                // Attach new source episodes and their trace IDs
                let episodes = self.episodes.lock().unwrap();
                for ep_id in &candidate.source_episode_ids {
                    if !record.source_episode_ids.contains(ep_id) {
                        record.source_episode_ids.push(ep_id.clone());
                        if let Some(ep) = episodes.get(ep_id) {
                            if !record.source_trace_ids.contains(&ep.source_trace_id) {
                                record.source_trace_ids.push(ep.source_trace_id.clone());
                            }
                        }
                    }
                }
                return Ok(Some(record.clone()));
            }
        }

        // Create new record
        let record_id = format!("mem_{}", ulid::Ulid::new());
        let kind = match candidate.kind {
            crate::types::CandidateKind::Fact => MemoryKind::Fact,
            crate::types::CandidateKind::Decision => MemoryKind::Decision,
            crate::types::CandidateKind::Preference => MemoryKind::Preference,
        };

        let record = MemoryRecord {
            record_id: record_id.clone(),
            claim: candidate.claim,
            kind,
            confidence: candidate.confidence,
            source_episode_ids: candidate.source_episode_ids.clone(),
            source_trace_ids: {
                // Look up trace IDs from episodes
                let episodes = self.episodes.lock().unwrap();
                candidate
                    .source_episode_ids
                    .iter()
                    .filter_map(|ep_id| episodes.get(ep_id).map(|e| e.source_trace_id.clone()))
                    .collect()
            },
            created_at: Utc::now(),
            valid_until: None,
            superseded_by: None,
            evidence_kind: EvidenceKind::AcceptedClaim,
            normalized_text_hash: claim_hash,
            supersedes_record_id: None,
        };

        records.insert(record_id, record.clone());
        Ok(Some(record))
    }

    async fn supersede_record(
        &self,
        old_record_id: &str,
        new_claim: String,
    ) -> Result<MemoryRecord, MemoryError> {
        let mut records = self.records.lock().unwrap();

        let old_record = records
            .get_mut(old_record_id)
            .ok_or_else(|| MemoryError::Internal(format!("Record not found: {old_record_id}")))?;

        let new_record_id = format!("mem_{}", ulid::Ulid::new());
        let claim_hash = compute_normalized_hash(&new_claim);
        let new_record = MemoryRecord {
            record_id: new_record_id.clone(),
            claim: new_claim,
            kind: old_record.kind,
            confidence: old_record.confidence,
            source_episode_ids: old_record.source_episode_ids.clone(),
            source_trace_ids: old_record.source_trace_ids.clone(),
            created_at: Utc::now(),
            valid_until: None,
            superseded_by: None,
            evidence_kind: old_record.evidence_kind,
            normalized_text_hash: claim_hash,
            supersedes_record_id: Some(old_record_id.to_string()),
        };

        old_record.superseded_by = Some(new_record_id.clone());
        old_record.valid_until = Some(Utc::now());

        records.insert(new_record_id, new_record.clone());
        Ok(new_record)
    }

    async fn search_records(&self, query: MemoryQuery) -> Result<RetrievalContext, MemoryError> {
        let records = self.records.lock().unwrap();
        let query_tokens = crate::query::tokenize(&query.text);
        let max = query.max_results.unwrap_or(10);

        let mut scored: Vec<(f64, MemoryRecord)> = Vec::new();

        for record in records.values() {
            if !record.is_active() {
                continue;
            }

            let claim_tokens = crate::query::tokenize(&record.claim);
            let match_count = query_tokens
                .iter()
                .filter(|qt| claim_tokens.iter().any(|ct| ct == *qt))
                .count();

            if match_count == 0 {
                continue;
            }

            let coverage = match_count as f64 / query_tokens.len().max(1) as f64;
            let score = coverage * record.confidence;
            scored.push((score, record.clone()));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max);

        let mut facts = Vec::new();
        let mut decisions = Vec::new();
        let mut episodes = Vec::new();

        for (_, record) in scored {
            match record.kind {
                MemoryKind::Fact => facts.push(record.claim),
                MemoryKind::Decision => decisions.push(record.claim),
                MemoryKind::Preference => episodes.push(record.claim),
            }
        }

        let total_hits = facts.len() + decisions.len() + episodes.len();
        Ok(RetrievalContext {
            facts,
            decisions,
            episodes,
            query_text: query.text,
            total_hits,
        })
    }

    async fn list_active_records(&self) -> Result<Vec<MemoryRecord>, MemoryError> {
        let records = self.records.lock().unwrap();
        Ok(records.values().filter(|r| r.is_active()).cloned().collect())
    }
}

#[async_trait]
impl crate::store::MemoryReadStore for InMemoryMemoryStore {
    async fn search(
        &self,
        query: MemoryQuery,
    ) -> Result<RetrievalContext, MemoryError> {
        self.search_records(query).await
    }
}
