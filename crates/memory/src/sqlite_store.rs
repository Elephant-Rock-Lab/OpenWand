//! SQLite-backed memory store.
//!
//! Uses the same SQLite file as the trace store but runs its own migrations.
//! WAL mode allows concurrent access from separate connections.

use crate::dedup::compute_normalized_hash;
use crate::evidence::EvidenceKind;
use crate::extractor::MemoryExtractor;
use crate::memory_store::MemoryStore;
use crate::sqlite_schema::{
    MEMORY_MIGRATION_0001_CHECKSUM, MEMORY_MIGRATION_0001_SQL,
    MEMORY_MIGRATION_0002_CHECKSUM, MEMORY_MIGRATION_0002_SQL,
    MEMORY_MIGRATION_0003_CHECKSUM, MEMORY_MIGRATION_0003_SQL,
};
use crate::types::{CandidateMemory, MemoryEpisode, MemoryKind, MemoryRecord};
use crate::{MemoryError, MemoryQuery, RetrievalContext};
use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Deterministic acceptance threshold. Candidates below this are rejected.
const CONFIDENCE_THRESHOLD: f64 = 0.7;

/// SQLite-backed memory store.
pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
}

impl SqliteMemoryStore {
    /// Open (or create) the memory store at the given SQLite path.
    /// Runs pending migrations.
    pub fn open(path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(path)
            .map_err(|e| MemoryError::Unavailable(format!("open memory db: {e}")))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| MemoryError::Unavailable(format!("set WAL: {e}")))?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MemoryError::Unavailable(format!("open in-memory: {e}")))?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<(), MemoryError> {
        // Create migration tracking table (same pattern as store crate)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_migration (
                version     INTEGER PRIMARY KEY,
                name        TEXT NOT NULL,
                checksum    TEXT NOT NULL,
                dirty       INTEGER NOT NULL DEFAULT 0,
                applied_at  INTEGER NOT NULL
            );",
        )
        .map_err(|e| MemoryError::Internal(format!("create migration table: {e}")))?;

        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM memory_migration",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        let migrations: &[(i64, &str, &str, &str)] = &[
            (
                1,
                "0001_memory",
                MEMORY_MIGRATION_0001_CHECKSUM,
                MEMORY_MIGRATION_0001_SQL,
            ),
            (
                2,
                "0002_ranking_provenance",
                MEMORY_MIGRATION_0002_CHECKSUM,
                MEMORY_MIGRATION_0002_SQL,
            ),
            (
                3,
                "0003_evidence_semantics",
                MEMORY_MIGRATION_0003_CHECKSUM,
                MEMORY_MIGRATION_0003_SQL,
            ),
        ];

        for &(version, name, checksum, sql) in migrations {
            if version <= current_version {
                continue;
            }

            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO memory_migration (version, name, checksum, dirty, applied_at) VALUES (?1, ?2, ?3, 1, ?4)",
                rusqlite::params![version, name, checksum, now],
            )
            .map_err(|e| MemoryError::Internal(format!("mark dirty: {e}")))?;

            conn.execute_batch(sql)
                .map_err(|e| MemoryError::Internal(format!("apply migration: {e}")))?;

            conn.execute(
                "UPDATE memory_migration SET dirty = 0 WHERE version = ?1",
                rusqlite::params![version],
            )
            .map_err(|e| MemoryError::Internal(format!("clear dirty: {e}")))?;
        }

        Ok(())
    }

    /// Run extraction and acceptance on all episodes.
    pub async fn extract_and_accept(
        &self,
        extractor: &dyn MemoryExtractor,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let episodes = {
            let conn = self.conn.lock().unwrap();
            Self::query_all_episodes(&conn)?
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

    fn query_all_episodes(conn: &Connection) -> Result<Vec<MemoryEpisode>, MemoryError> {
        let mut stmt = conn
            .prepare(
                "SELECT episode_id, source_trace_id, session_id, event_kind, role, content, created_at
                 FROM memory_episode ORDER BY created_at",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare episodes: {e}")))?;

        let episodes = stmt
            .query_map([], |row| {
                Ok(MemoryEpisode {
                    episode_id: row.get(0)?,
                    source_trace_id: row.get(1)?,
                    session_id: row.get(2)?,
                    event_kind: row.get(3)?,
                    role: match row.get::<_, String>(4)?.as_str() {
                        "user" => crate::types::EpisodeRole::User,
                        "assistant" => crate::types::EpisodeRole::Assistant,
                        "tool" => crate::types::EpisodeRole::Tool,
                        _ => crate::types::EpisodeRole::System,
                    },
                    content: row.get(5)?,
                    created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(6)?, 0)
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| MemoryError::QueryFailed(format!("query episodes: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::QueryFailed(format!("collect episodes: {e}")))?;

        Ok(episodes)
    }

    fn confidence_to_bps(confidence: f64) -> i64 {
        (confidence * 10000.0).round() as i64
    }

    fn bps_to_confidence(bps: i64) -> f64 {
        bps as f64 / 10000.0
    }

    fn kind_to_str(kind: MemoryKind) -> &'static str {
        match kind {
            MemoryKind::Fact => "fact",
            MemoryKind::Decision => "decision",
            MemoryKind::Preference => "preference",
        }
    }

    fn str_to_kind(s: &str) -> MemoryKind {
        match s {
            "decision" => MemoryKind::Decision,
            "preference" => MemoryKind::Preference,
            _ => MemoryKind::Fact,
        }
    }

    /// List all records including superseded, for ranked search.
    async fn list_all_records_for_search(&self) -> Result<Vec<MemoryRecord>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT record_id, kind, claim, confidence_bps, status, valid_from, valid_until,
                        superseded_by, created_at, updated_at, scope_kind, evidence_kind, normalized_text_hash, supersedes_record_id, conflict_group_id
                 FROM memory_record
                 WHERE status IN ('active', 'superseded')
                 ORDER BY created_at DESC",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare list all: {e}")))?;

        let records = stmt
            .query_map([], |row| Ok(Self::row_to_record(row)))
            .map_err(|e| MemoryError::QueryFailed(format!("query all: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::QueryFailed(format!("collect: {e}")))?;

        // Attach sources
        let mut result = Vec::with_capacity(records.len());
        for mut record in records {
            let sources: Vec<(String, String)> = {
                let mut src_stmt = conn.prepare(
                    "SELECT episode_id, source_trace_id FROM memory_record_source WHERE record_id = ?1",
                ).map_err(|e| MemoryError::QueryFailed(format!("prepare sources: {e}")))?;
                let rows = src_stmt.query_map(rusqlite::params![record.record_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }).map_err(|e| MemoryError::QueryFailed(format!("query sources: {e}")))?;
                rows.collect::<Result<Vec<_>, _>>().map_err(|e| MemoryError::QueryFailed(format!("collect sources: {e}")))?
            };

            record.source_episode_ids = sources.iter().map(|(ep, _)| ep.clone()).collect();
            record.source_trace_ids = sources.iter().map(|(_, tr)| tr.clone()).collect();
            result.push(record);
        }

        Ok(result)
    }

    /// Test-only: get a reference to the underlying connection.
    /// DO NOT use in production code.
    #[cfg(feature = "sqlite-testing")]
    pub fn conn_for_test(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn project_episode(&self, episode: MemoryEpisode) -> Result<(), MemoryError> {
        let conn = self.conn.lock().unwrap();

        // Idempotent: INSERT OR IGNORE by UNIQUE(source_trace_id)
        conn.execute(
            "INSERT OR IGNORE INTO memory_episode
                (episode_id, source_trace_id, session_id, event_kind, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                episode.episode_id,
                episode.source_trace_id,
                episode.session_id,
                episode.event_kind,
                match episode.role {
                    crate::types::EpisodeRole::User => "user",
                    crate::types::EpisodeRole::Assistant => "assistant",
                    crate::types::EpisodeRole::Tool => "tool",
                    crate::types::EpisodeRole::System => "system",
                },
                episode.content,
                episode.created_at.timestamp(),
            ],
        )
        .map_err(|e| MemoryError::Internal(format!("insert episode: {e}")))?;

        Ok(())
    }

    async fn get_episodes(&self, session_id: &str) -> Result<Vec<MemoryEpisode>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT episode_id, source_trace_id, session_id, event_kind, role, content, created_at
                 FROM memory_episode WHERE session_id = ?1 ORDER BY created_at",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare: {e}")))?;

        let episodes = stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok(MemoryEpisode {
                    episode_id: row.get(0)?,
                    source_trace_id: row.get(1)?,
                    session_id: row.get(2)?,
                    event_kind: row.get(3)?,
                    role: match row.get::<_, String>(4)?.as_str() {
                        "user" => crate::types::EpisodeRole::User,
                        "assistant" => crate::types::EpisodeRole::Assistant,
                        "tool" => crate::types::EpisodeRole::Tool,
                        _ => crate::types::EpisodeRole::System,
                    },
                    content: row.get(5)?,
                    created_at: chrono::DateTime::from_timestamp(row.get::<_, i64>(6)?, 0)
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| MemoryError::QueryFailed(format!("query: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::QueryFailed(format!("collect: {e}")))?;

        Ok(episodes)
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

        let conn = self.conn.lock().unwrap();

        // Check for duplicate using normalized_text_hash
        let claim_hash = compute_normalized_hash(&candidate.claim);
        let existing: Option<String> = conn
            .query_row(
                "SELECT record_id FROM memory_record
                 WHERE normalized_text_hash = ?1 AND evidence_kind = 'AcceptedClaim' AND status = 'active'
                 LIMIT 1",
                rusqlite::params![claim_hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(existing_id) = existing {
            // Attach new source episodes
            for ep_id in &candidate.source_episode_ids {
                // Look up source_trace_id from episode
                let source_trace_id: Option<String> = conn
                    .query_row(
                        "SELECT source_trace_id FROM memory_episode WHERE episode_id = ?1",
                        rusqlite::params![ep_id],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(trace_id) = source_trace_id {
                    conn.execute(
                        "INSERT OR IGNORE INTO memory_record_source (record_id, episode_id, source_trace_id)
                         VALUES (?1, ?2, ?3)",
                        rusqlite::params![existing_id, ep_id, trace_id],
                    )
                    .map_err(|e| MemoryError::Internal(format!("attach source: {e}")))?;
                }
            }

            // Return the updated record
            let record = Self::read_record(&conn, &existing_id)?
                .ok_or_else(|| MemoryError::Internal("record vanished".into()))?;
            return Ok(Some(record));
        }

        // Create new record
        let record_id = format!("mem_{}", ulid::Ulid::new());
        let kind = match candidate.kind {
            crate::types::CandidateKind::Fact => MemoryKind::Fact,
            crate::types::CandidateKind::Decision => MemoryKind::Decision,
            crate::types::CandidateKind::Preference => MemoryKind::Preference,
        };
        let now = Utc::now();

        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, evidence_kind, normalized_text_hash)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                record_id,
                Self::kind_to_str(kind),
                candidate.claim,
                Self::confidence_to_bps(candidate.confidence),
                now.timestamp(),
                now.timestamp(),
                now.timestamp(),
                "AcceptedClaim",
                claim_hash,
            ],
        )
        .map_err(|e| MemoryError::Internal(format!("insert record: {e}")))?;

        // Insert sources
        for ep_id in &candidate.source_episode_ids {
            let source_trace_id: Option<String> = conn
                .query_row(
                    "SELECT source_trace_id FROM memory_episode WHERE episode_id = ?1",
                    rusqlite::params![ep_id],
                    |row| row.get(0),
                )
                .ok();

            if let Some(trace_id) = source_trace_id {
                conn.execute(
                    "INSERT OR IGNORE INTO memory_record_source (record_id, episode_id, source_trace_id)
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![record_id, ep_id, trace_id],
                )
                .map_err(|e| MemoryError::Internal(format!("insert source: {e}")))?;
            }
        }

        let record = Self::read_record(&conn, &record_id)?
            .ok_or_else(|| MemoryError::Internal("record vanished".into()))?;
        Ok(Some(record))
    }

    async fn supersede_record(
        &self,
        old_record_id: &str,
        new_claim: String,
    ) -> Result<MemoryRecord, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let old_record = Self::read_record(&conn, old_record_id)?
            .ok_or_else(|| MemoryError::Internal(format!("record not found: {old_record_id}")))?;

        let new_record_id = format!("mem_{}", ulid::Ulid::new());
        let now = Utc::now();

        // Create new record — preserve original evidence_kind
        let evidence_kind_str = format!("{:?}", old_record.evidence_kind);
        let claim_hash = compute_normalized_hash(&new_claim);
        conn.execute(
            "INSERT INTO memory_record (record_id, kind, claim, confidence_bps, status, valid_from, created_at, updated_at, evidence_kind, normalized_text_hash, supersedes_record_id)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                new_record_id,
                Self::kind_to_str(old_record.kind),
                new_claim,
                Self::confidence_to_bps(old_record.confidence),
                now.timestamp(),
                now.timestamp(),
                now.timestamp(),
                evidence_kind_str,
                claim_hash,
                old_record_id,
            ],
        )
        .map_err(|e| MemoryError::Internal(format!("insert superseding record: {e}")))?;

        // Copy sources from old record
        conn.execute(
            "INSERT OR IGNORE INTO memory_record_source (record_id, episode_id, source_trace_id)
             SELECT ?1, episode_id, source_trace_id FROM memory_record_source WHERE record_id = ?2",
            rusqlite::params![new_record_id, old_record_id],
        )
        .map_err(|e| MemoryError::Internal(format!("copy sources: {e}")))?;

        // Mark old record as superseded
        conn.execute(
            "UPDATE memory_record SET status = 'superseded', superseded_by = ?1, valid_until = ?2, updated_at = ?2 WHERE record_id = ?3",
            rusqlite::params![new_record_id, now.timestamp(), old_record_id],
        )
        .map_err(|e| MemoryError::Internal(format!("supersede old: {e}")))?;

        let new_record = Self::read_record(&conn, &new_record_id)?
            .ok_or_else(|| MemoryError::Internal("new record vanished".into()))?;
        Ok(new_record)
    }

    async fn search_records(&self, query: MemoryQuery) -> Result<RetrievalContext, MemoryError> {
        let conn = self.conn.lock().unwrap();
        let max = query.max_results.unwrap_or(10);

        // Fetch all active records
        let mut stmt = conn
            .prepare(
                "SELECT record_id, kind, claim, confidence_bps, status, valid_from, valid_until,
                        superseded_by, created_at, updated_at, scope_kind, evidence_kind, normalized_text_hash, supersedes_record_id, conflict_group_id
                 FROM memory_record
                 WHERE status = 'active'
                 ORDER BY created_at DESC",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare search: {e}")))?;

        let records: Vec<MemoryRecord> = stmt
            .query_map([], |row| Ok(Self::row_to_record(row)))
            .map_err(|e| MemoryError::QueryFailed(format!("search: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::QueryFailed(format!("collect: {e}")))?;

        // Token-based scoring
        let query_tokens = crate::query::tokenize(&query.text);
        let mut scored: Vec<(f64, MemoryRecord)> = Vec::new();

        for record in records {
            let claim_tokens = crate::query::tokenize(&record.claim);
            let match_count = query_tokens
                .iter()
                .filter(|qt| claim_tokens.iter().any(|ct| ct == *qt))
                .count();

            if match_count == 0 {
                continue;
            }

            // Score: fraction of query tokens matched, boosted by confidence
            let coverage = match_count as f64 / query_tokens.len().max(1) as f64;
            let score = coverage * record.confidence;
            scored.push((score, record));
        }

        // Sort by score descending
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

    async fn search_ranked(
        &self,
        query: MemoryQuery,
        mode: crate::supersession::RetrievalMode,
    ) -> Result<crate::retrieval::RankedRetrievalContext, MemoryError> {
        use crate::ranking::{compute_final_score, evidence_bps_from_kind, RankingWeights, MemoryRankScore};
        use crate::retrieval::{RankedMemoryHit, RankedRetrievalContext};
        use crate::supersession::{should_exclude_superseded, supersession_penalty};

        let records = if mode == crate::supersession::RetrievalMode::CurrentState {
            self.list_active_records().await?
        } else {
            // For Default/ChangeHistory/ConflictSearch, include superseded records
            self.list_all_records_for_search().await?
        };
        let query_tokens = crate::query::tokenize(&query.text);

        let mut hits: Vec<RankedMemoryHit> = records
            .iter()
            .filter(|r| {
                if should_exclude_superseded(r.superseded_by.is_some(), mode) {
                    return false;
                }
                true
            })
            .filter_map(|r| {
                let record_tokens = crate::query::tokenize(&r.claim);
                let overlap = query_tokens
                    .iter()
                    .filter(|qt| record_tokens.iter().any(|rt| rt == *qt))
                    .count() as u16;
                if overlap == 0 {
                    return None;
                }

                let relevance_bps = if query_tokens.is_empty() {
                    0
                } else {
                    (overlap as u32 * 10000 / query_tokens.len() as u32).min(10000) as u16
                };

                let derived_kind = r.derived_evidence_kind();
                let evidence_bps_raw = evidence_bps_from_kind(&derived_kind);
                let penalty = supersession_penalty(r.superseded_by.is_some(), mode);
                let evidence_bps = if evidence_bps_raw > penalty { evidence_bps_raw - penalty } else { 0 };

                let score = MemoryRankScore {
                    relevance_bps,
                    provenance_bps: 7000,
                    scope_bps: 7000,
                    recency_bps: 7000,
                    confidence_bps: (r.confidence * 10000.0) as u16,
                    evidence_bps,
                    verification_bps: 0, // populated post-ranking by coordinator
                    final_bps: 0,
                };

                let weights = RankingWeights::default();
                let final_bps = compute_final_score(&score, &weights);

                Some(RankedMemoryHit {
                    id: r.record_id.clone(),
                    text: r.claim.clone(),
                    score: MemoryRankScore { final_bps, ..score },
                    evidence_kind: derived_kind,
                    source_episode_ids: r.source_episode_ids.clone(),
                    source_trace_ids: r.source_trace_ids.clone(),
                    scope: crate::provenance::MemoryScope::Global,
                    provenance: crate::provenance::ProvenanceSnapshot::default(),
                    confidence_bps: (r.confidence * 10000.0) as u16,
                    reason: format!("relevance={}, evidence={:?}", relevance_bps, derived_kind),
                })
            })
            .collect();

        hits.sort_by(|a, b| b.score.final_bps.cmp(&a.score.final_bps));

        let total_hits = hits.len();
        Ok(RankedRetrievalContext {
            hits,
            query_text: query.text,
            total_hits,
        })
    }

    async fn list_active_records(&self) -> Result<Vec<MemoryRecord>, MemoryError> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT record_id, kind, claim, confidence_bps, status, valid_from, valid_until,
                        superseded_by, created_at, updated_at, scope_kind, evidence_kind, normalized_text_hash, supersedes_record_id, conflict_group_id
                 FROM memory_record
                 WHERE status = 'active'
                 ORDER BY created_at DESC",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare list: {e}")))?;

        let records: Vec<MemoryRecord> = stmt
            .query_map([], |row| Ok(Self::row_to_record(row)))
            .map_err(|e| MemoryError::QueryFailed(format!("list: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| MemoryError::QueryFailed(format!("collect: {e}")))?;

        // Populate source episode/trace IDs for each record
        let mut populated = Vec::new();
        for mut record in records {
            let mut src_stmt = conn
                .prepare(
                    "SELECT episode_id, source_trace_id FROM memory_record_source WHERE record_id = ?1",
                )
                .map_err(|e| MemoryError::QueryFailed(format!("prepare sources: {e}")))?;

            let sources: Vec<(String, String)> = src_stmt
                .query_map(rusqlite::params![record.record_id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(|e| MemoryError::QueryFailed(format!("sources: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MemoryError::QueryFailed(format!("collect sources: {e}")))?;

            record.source_episode_ids = sources.iter().map(|(ep, _)| ep.clone()).collect();
            record.source_trace_ids = sources.iter().map(|(_, tr)| tr.clone()).collect();
            populated.push(record);
        }

        Ok(populated)
    }
}

impl SqliteMemoryStore {
    fn read_record(conn: &Connection, record_id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let mut stmt = conn
            .prepare(
                "SELECT record_id, kind, claim, confidence_bps, status, valid_from, valid_until,
                        superseded_by, created_at, updated_at, scope_kind, evidence_kind, normalized_text_hash, supersedes_record_id, conflict_group_id
                 FROM memory_record WHERE record_id = ?1",
            )
            .map_err(|e| MemoryError::QueryFailed(format!("prepare read: {e}")))?;

        let mut record = stmt
            .query_row(rusqlite::params![record_id], |row| Ok(Self::row_to_record(row)))
            .ok();

        // Also read source episode/trace IDs
        if let Some(ref mut rec) = record {
            let mut src_stmt = conn
                .prepare(
                    "SELECT episode_id, source_trace_id FROM memory_record_source WHERE record_id = ?1",
                )
                .map_err(|e| MemoryError::QueryFailed(format!("prepare sources: {e}")))?;

            let sources: Vec<(String, String)> = src_stmt
                .query_map(rusqlite::params![record_id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(|e| MemoryError::QueryFailed(format!("sources: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MemoryError::QueryFailed(format!("collect sources: {e}")))?;

            rec.source_episode_ids = sources.iter().map(|(ep, _)| ep.clone()).collect();
            rec.source_trace_ids = sources.iter().map(|(_, tr)| tr.clone()).collect();
        }

        Ok(record)
    }

    fn row_to_record(row: &rusqlite::Row<'_>) -> MemoryRecord {
        let kind_str: String = row.get(1).unwrap_or_default();
        let confidence_bps: i64 = row.get(3).unwrap_or(0);
        let status: String = row.get(4).unwrap_or_default();
        let valid_from_ts: Option<i64> = row.get(5).ok();
        let valid_until_ts: Option<i64> = row.get(6).ok();
        let created_at_ts: i64 = row.get(8).unwrap_or(0);
        let evidence_kind_str: Option<String> = row.get(11).ok().flatten();
        let superseded_by: Option<String> = row.get(7).ok().flatten();

        let stored_kind = evidence_kind_str
            .as_deref()
            .and_then(|s| match s {
                "AcceptedClaim" => Some(EvidenceKind::AcceptedClaim),
                "UserStatedClaim" => Some(EvidenceKind::UserStatedClaim),
                "DeterministicEvidence" => Some(EvidenceKind::DeterministicEvidence),
                "RawObservation" => Some(EvidenceKind::RawObservation),
                "LlmExtractedCandidate" => Some(EvidenceKind::LlmExtractedCandidate),
                "SupersededClaim" => Some(EvidenceKind::SupersededClaim),
                "ConflictingClaim" => Some(EvidenceKind::ConflictingClaim),
                _ => None,
            })
            .unwrap_or(EvidenceKind::AcceptedClaim);

        // Derive lifecycle label: superseded records get SupersededClaim at retrieval
        // Original evidence_kind is preserved in the stored column
        let evidence_kind = if superseded_by.is_some() {
            EvidenceKind::SupersededClaim
        } else {
            stored_kind
        };

        MemoryRecord {
            record_id: row.get(0).unwrap_or_default(),
            claim: row.get(2).unwrap_or_default(),
            kind: Self::str_to_kind(&kind_str),
            confidence: Self::bps_to_confidence(confidence_bps),
            source_episode_ids: vec![],
            source_trace_ids: vec![],
            created_at: chrono::DateTime::from_timestamp(created_at_ts, 0).unwrap_or_default(),
            valid_until: valid_until_ts
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)),
            superseded_by,
            evidence_kind,
            normalized_text_hash: row.get(12).ok().flatten().unwrap_or_default(),
            supersedes_record_id: row.get(13).ok().flatten(),
            conflict_group_id: row.get(14).ok().flatten(),
        }
    }
}

#[async_trait]
impl crate::store::MemoryReadStore for SqliteMemoryStore {
    async fn search(
        &self,
        query: MemoryQuery,
    ) -> Result<RetrievalContext, MemoryError> {
        self.search_records(query).await
    }
}
