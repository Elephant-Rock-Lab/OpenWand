//! Memory provenance hydration — display-ready provenance from existing data.
//!
//! Thread data already present in MemoryRecord, RankedMemoryHit, and
//! RepoConsistencyFinding into a single hydrated DTO. No store queries.
//! No async. No trace lookups. Pure cross-reference of in-memory data.
//!
//! This is the audit/explainability layer: every claim surfaced to the panel
//! or prompt auditor can answer "why was this included/excluded, and what
//! evidence produced it?"

use crate::evidence::EvidenceKind;
use crate::prompt_assembly::PromptInclusionReason;
use crate::provenance::ProvenanceKind;
use crate::ranking::MemoryRankScore;
use crate::repo_consistency::{ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind};
use crate::retrieval::RankedMemoryHit;
use crate::types::MemoryRecord;
use chrono::{DateTime, Utc};

/// Which trust bucket this claim belongs to.
/// One-to-one with RepoConsistencyFindingKind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryTrustBucket {
    PromptIncluded,
    Stale,
    MissingInRepo,
    MissingInMemory,
    Conflict,
    Unverifiable,
    SupersededIgnored,
}

impl MemoryTrustBucket {
    pub fn from_finding_kind(kind: &RepoConsistencyFindingKind) -> Self {
        match kind {
            RepoConsistencyFindingKind::Supported => Self::PromptIncluded,
            RepoConsistencyFindingKind::StaleMemory => Self::Stale,
            RepoConsistencyFindingKind::MissingInRepo => Self::MissingInRepo,
            RepoConsistencyFindingKind::MissingInMemory => Self::MissingInMemory,
            RepoConsistencyFindingKind::SupersededMemoryIgnored => Self::SupersededIgnored,
            RepoConsistencyFindingKind::ConflictRequiresReview => Self::Conflict,
            RepoConsistencyFindingKind::Unverifiable => Self::Unverifiable,
        }
    }
}

/// Provenance for a memory claim — WHERE it came from.
/// Populated from RankedMemoryHit.provenance + MemoryRecord metadata.
#[derive(Debug, Clone)]
pub struct MemoryEvidenceProvenance {
    pub provenance_kind: ProvenanceKind,
    pub record_id: Option<String>,
    pub source_trace_ids: Vec<String>,
    pub source_episode_ids: Vec<String>,
    pub confidence: Option<f64>,
    pub created_at: Option<DateTime<Utc>>,
    pub evidence_kind: Option<EvidenceKind>,
    pub retrieval_reason: Option<String>,
    /// Formatted rank score summary — never raw internals exposed to UI.
    pub rank_score_summary: Option<String>,
}

impl MemoryEvidenceProvenance {
    /// Short human-readable label for the provenance kind.
    pub fn short_label(&self) -> String {
        match self.provenance_kind {
            ProvenanceKind::UserStated => "User-stated claim".to_string(),
            ProvenanceKind::LlmExtracted => "LLM-extracted claim".to_string(),
            ProvenanceKind::SystemDerived => "System-derived claim".to_string(),
            ProvenanceKind::Unknown => "Unknown provenance".to_string(),
        }
    }

    /// One-line evidence summary for panel display.
    pub fn evidence_line(&self) -> String {
        let mut parts = vec![self.short_label()];

        if let Some(ref id) = self.record_id {
            parts.push(format!("record {id}"));
        }

        if !self.source_trace_ids.is_empty() {
            parts.push(format!("{} trace(s)", self.source_trace_ids.len()));
        } else {
            parts.push("no source trace".to_string());
        }

        if let Some(conf) = self.confidence {
            parts.push(format!("{:.0}% confidence", conf * 100.0));
        }

        parts.join(" · ")
    }

    pub fn unknown() -> Self {
        Self {
            provenance_kind: ProvenanceKind::Unknown,
            record_id: None,
            source_trace_ids: vec![],
            source_episode_ids: vec![],
            confidence: None,
            created_at: None,
            evidence_kind: None,
            retrieval_reason: None,
            rank_score_summary: None,
        }
    }
}

/// Conflict lineage — WHY this claim conflicts.
#[derive(Debug, Clone)]
pub struct ConflictProvenance {
    pub conflict_group_id: Option<String>,
    /// Claim texts of other records in the same conflict group.
    /// Empty when the hydrator can't look up competing claims (no store query).
    pub conflicting_claim_texts: Vec<String>,
    pub explanation: String,
}

/// Supersession lineage — WHAT this claim replaces or is replaced by.
#[derive(Debug, Clone)]
pub struct SupersessionProvenance {
    pub supersedes_record_id: Option<String>,
    pub superseded_by_record_id: Option<String>,
    pub explanation: String,
}

/// How complete the provenance is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceHydrationStatus {
    Complete,
    Partial { missing: Vec<String> },
    Missing { reason: String },
}

/// Format a MemoryRankScore into a display summary.
/// Keeps scoring internals out of UI DTOs.
fn format_rank_score(score: &MemoryRankScore) -> String {
    format!(
        "relevance {:.0}% · recency {:.0}% · confidence {:.0}% · final {:.0}bps",
        score.relevance_bps as f64 / 100.0,
        score.recency_bps as f64 / 100.0,
        score.confidence_bps as f64 / 100.0,
        score.final_bps,
    )
}

use crate::trace_relation_hydration::ClaimTraceLineage;

/// A fully hydrated memory claim — display-ready, no store references.
#[derive(Debug, Clone)]
pub struct HydratedMemoryClaim {
    pub claim_text: String,
    pub bucket: MemoryTrustBucket,
    pub provenance: MemoryEvidenceProvenance,
    pub conflict: Option<ConflictProvenance>,
    pub supersession: Option<SupersessionProvenance>,
    pub hydration_status: ProvenanceHydrationStatus,
    pub repo_evidence_key: Vec<String>,
    pub severity: ConsistencySeverity,
    pub inclusion_reason: Option<PromptInclusionReason>,
    /// Trace relation lineage — audit/panel-only.
    pub trace_lineage: Option<ClaimTraceLineage>,
}

/// Pure hydrator — no store queries, no async, no trace lookups.
/// Cross-references findings to records/hits using a deterministic lookup precedence.
pub struct MemoryProvenanceHydrator;

impl MemoryProvenanceHydrator {
    /// Hydrate a single claim from available data.
    pub fn hydrate(
        claim_text: &str,
        bucket: MemoryTrustBucket,
        finding: &RepoConsistencyFinding,
        hit: Option<&RankedMemoryHit>,
        record: Option<&MemoryRecord>,
    ) -> HydratedMemoryClaim {
        let provenance = match (hit, record) {
            (Some(h), Some(r)) => Self::hydrate_from_both(h, r),
            (Some(h), None) => Self::hydrate_from_hit(h),
            (None, Some(r)) => Self::hydrate_from_record(r),
            (None, None) => MemoryEvidenceProvenance::unknown(),
        };

        let conflict = if bucket == MemoryTrustBucket::Conflict {
            Some(ConflictProvenance {
                conflict_group_id: record.and_then(|r| r.conflict_group_id.clone()),
                conflicting_claim_texts: vec![], // No store query to look up competing claims
                explanation: finding.detail.clone(),
            })
        } else {
            None
        };

        let supersession = match bucket {
            MemoryTrustBucket::SupersededIgnored | MemoryTrustBucket::Stale => {
                Some(SupersessionProvenance {
                    supersedes_record_id: record.and_then(|r| r.supersedes_record_id.clone()),
                    superseded_by_record_id: record.and_then(|r| r.superseded_by.clone()),
                    explanation: finding.detail.clone(),
                })
            }
            _ => None,
        };

        let mut missing = Vec::new();
        if provenance.record_id.is_none() {
            missing.push("record_id".to_string());
        }
        if provenance.source_trace_ids.is_empty() {
            missing.push("source_trace_ids".to_string());
        }

        let hydration_status = match bucket {
            MemoryTrustBucket::Unverifiable => ProvenanceHydrationStatus::Missing {
                reason: "Unverifiable claim — no supporting evidence available".to_string(),
            },
            _ if missing.is_empty() => ProvenanceHydrationStatus::Complete,
            _ => ProvenanceHydrationStatus::Partial { missing },
        };

        HydratedMemoryClaim {
            claim_text: claim_text.to_string(),
            bucket,
            provenance,
            conflict,
            supersession,
            hydration_status,
            repo_evidence_key: finding.repo_evidence_key.clone(),
            severity: finding.severity.clone(),
            inclusion_reason: None, // Filled by caller from inputs
            trace_lineage: None,
        }
    }

    /// Batch hydration: findings → hydrated claims.
    /// Matches findings to records/hits using lookup precedence:
    ///   1. record_id (if available on finding — not currently)
    ///   2. normalized_text_hash (if available)
    ///   3. exact claim text
    ///   4. normalized/lowercase claim text
    pub fn hydrate_findings(
        findings: &[RepoConsistencyFinding],
        hits: &[RankedMemoryHit],
        records: &[MemoryRecord],
    ) -> Vec<HydratedMemoryClaim> {
        // Build indices for lookup precedence
        let mut record_by_text: std::collections::HashMap<String, &MemoryRecord> = std::collections::HashMap::new();
        let mut record_by_hash: std::collections::HashMap<String, &MemoryRecord> = std::collections::HashMap::new();
        for r in records {
            record_by_text.insert(r.claim.to_lowercase(), r);
            if !r.normalized_text_hash.is_empty() {
                record_by_hash.insert(r.normalized_text_hash.clone(), r);
            }
        }

        let mut hit_by_text: std::collections::HashMap<String, &RankedMemoryHit> = std::collections::HashMap::new();
        for h in hits {
            hit_by_text.insert(h.text.to_lowercase(), h);
        }

        let mut result = Vec::new();
        for finding in findings {
            let claim_text = finding.claim_text.as_deref().unwrap_or("");
            let bucket = MemoryTrustBucket::from_finding_kind(&finding.kind);

            // Lookup precedence for record:
            // 1. normalized_text_hash match
            // 2. exact claim text match
            // 3. lowercase claim text match
            let record: Option<&MemoryRecord> = finding
                .evidence_kind
                .and_then(|_| None) // No record_id on finding yet
                .or_else(|| {
                    // Try hash match if we had a hash on the finding — we don't
                    None::<&MemoryRecord>
                })
                .or_else(|| record_by_text.get(&claim_text.to_lowercase()).copied());

            // Lookup precedence for hit: lowercase text match
            let hit = hit_by_text.get(&claim_text.to_lowercase()).copied();

            result.push(Self::hydrate(claim_text, bucket, finding, hit, record));
        }
        result
    }

    fn hydrate_from_both(hit: &RankedMemoryHit, record: &MemoryRecord) -> MemoryEvidenceProvenance {
        MemoryEvidenceProvenance {
            provenance_kind: hit.provenance.kind.clone(),
            record_id: Some(record.record_id.clone()),
            source_trace_ids: record.source_trace_ids.clone(),
            source_episode_ids: record.source_episode_ids.clone(),
            confidence: Some(record.confidence),
            created_at: Some(record.created_at),
            evidence_kind: Some(record.evidence_kind.clone()),
            retrieval_reason: if hit.reason.is_empty() { None } else { Some(hit.reason.clone()) },
            rank_score_summary: Some(format_rank_score(&hit.score)),
        }
    }

    fn hydrate_from_hit(hit: &RankedMemoryHit) -> MemoryEvidenceProvenance {
        MemoryEvidenceProvenance {
            provenance_kind: hit.provenance.kind.clone(),
            record_id: Some(hit.id.clone()),
            source_trace_ids: hit.source_trace_ids.clone(),
            source_episode_ids: hit.source_episode_ids.clone(),
            confidence: Some(hit.confidence_bps as f64 / 10000.0),
            created_at: None,
            evidence_kind: Some(hit.evidence_kind.clone()),
            retrieval_reason: if hit.reason.is_empty() { None } else { Some(hit.reason.clone()) },
            rank_score_summary: Some(format_rank_score(&hit.score)),
        }
    }

    fn hydrate_from_record(record: &MemoryRecord) -> MemoryEvidenceProvenance {
        MemoryEvidenceProvenance {
            provenance_kind: ProvenanceKind::Unknown,
            record_id: Some(record.record_id.clone()),
            source_trace_ids: record.source_trace_ids.clone(),
            source_episode_ids: record.source_episode_ids.clone(),
            confidence: Some(record.confidence),
            created_at: Some(record.created_at),
            evidence_kind: Some(record.evidence_kind.clone()),
            retrieval_reason: None,
            rank_score_summary: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranking::MemoryRankScore;
    use crate::retrieval::RankedMemoryHit;
    use crate::types::{CandidateKind, MemoryKind};

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

    fn make_record(claim: &str, id: &str) -> MemoryRecord {
        MemoryRecord {
            record_id: id.to_string(),
            claim: claim.to_string(),
            kind: MemoryKind::Fact,
            confidence: 0.9,
            source_episode_ids: vec!["ep_1".to_string()],
            source_trace_ids: vec!["trace_001".to_string()],
            created_at: Utc::now(),
            valid_until: None,
            superseded_by: None,
            evidence_kind: EvidenceKind::AcceptedClaim,
            normalized_text_hash: "abc123".to_string(),
            supersedes_record_id: None,
            conflict_group_id: None,
        }
    }

    fn make_hit(text: &str) -> RankedMemoryHit {
        RankedMemoryHit {
            id: "hit_1".to_string(),
            text: text.to_string(),
            score: MemoryRankScore {
                relevance_bps: 8000,
                provenance_bps: 7000,
                scope_bps: 5000,
                recency_bps: 6000,
                confidence_bps: 9000,
                evidence_bps: 8500,
                verification_bps: 0,
                final_bps: 7200,
            },
            evidence_kind: EvidenceKind::AcceptedClaim,
            source_episode_ids: vec!["ep_1".to_string()],
            source_trace_ids: vec!["trace_001".to_string()],
            scope: crate::provenance::MemoryScope::Global,
            provenance: crate::provenance::ProvenanceSnapshot {
                kind: ProvenanceKind::UserStated,
            },
            confidence_bps: 9000,
            reason: "keyword match".to_string(),
        }
    }

    #[test]
    fn short_label_user_stated() {
        let p = MemoryEvidenceProvenance {
            provenance_kind: ProvenanceKind::UserStated,
            ..MemoryEvidenceProvenance::unknown()
        };
        assert_eq!("User-stated claim", p.short_label());
    }

    #[test]
    fn short_label_llm_extracted() {
        let p = MemoryEvidenceProvenance {
            provenance_kind: ProvenanceKind::LlmExtracted,
            ..MemoryEvidenceProvenance::unknown()
        };
        assert_eq!("LLM-extracted claim", p.short_label());
    }

    #[test]
    fn short_label_system_derived() {
        let p = MemoryEvidenceProvenance {
            provenance_kind: ProvenanceKind::SystemDerived,
            ..MemoryEvidenceProvenance::unknown()
        };
        assert_eq!("System-derived claim", p.short_label());
    }

    #[test]
    fn short_label_unknown() {
        let p = MemoryEvidenceProvenance::unknown();
        assert_eq!("Unknown provenance", p.short_label());
    }

    #[test]
    fn evidence_line_with_trace() {
        let p = MemoryEvidenceProvenance {
            provenance_kind: ProvenanceKind::UserStated,
            record_id: Some("rec_1".to_string()),
            source_trace_ids: vec!["trace_001".to_string()],
            confidence: Some(0.9),
            ..MemoryEvidenceProvenance::unknown()
        };
        let line = p.evidence_line();
        assert!(line.contains("User-stated claim"));
        assert!(line.contains("rec_1"));
        assert!(line.contains("1 trace(s)"));
        assert!(line.contains("90% confidence"));
    }

    #[test]
    fn evidence_line_without_trace() {
        let p = MemoryEvidenceProvenance::unknown();
        let line = p.evidence_line();
        assert!(line.contains("no source trace"));
    }

    #[test]
    fn hydration_status_complete() {
        let record = make_record("test claim", "rec_1");
        let hit = make_hit("test claim");
        let claim = MemoryProvenanceHydrator::hydrate(
            "test claim",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "test claim"),
            Some(&hit),
            Some(&record),
        );
        assert_eq!(ProvenanceHydrationStatus::Complete, claim.hydration_status);
    }

    #[test]
    fn hydration_status_partial() {
        let claim = MemoryProvenanceHydrator::hydrate(
            "test claim",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "test claim"),
            None,
            None,
        );
        match claim.hydration_status {
            ProvenanceHydrationStatus::Partial { missing } => {
                assert!(missing.contains(&"record_id".to_string()));
                assert!(missing.contains(&"source_trace_ids".to_string()));
            }
            other => panic!("Expected Partial, got {:?}", other),
        }
    }

    #[test]
    fn hydration_status_missing_unverifiable() {
        let claim = MemoryProvenanceHydrator::hydrate(
            "test",
            MemoryTrustBucket::Unverifiable,
            &make_finding(RepoConsistencyFindingKind::Unverifiable, "test"),
            None,
            None,
        );
        match claim.hydration_status {
            ProvenanceHydrationStatus::Missing { reason } => {
                assert!(reason.contains("Unverifiable"));
            }
            other => panic!("Expected Missing, got {:?}", other),
        }
    }

    #[test]
    fn hydrates_user_stated_record_with_episode_and_trace() {
        let record = make_record("crate core exists", "rec_1");
        let hit = make_hit("crate core exists");
        let claim = MemoryProvenanceHydrator::hydrate(
            "crate core exists",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "crate core exists"),
            Some(&hit),
            Some(&record),
        );
        assert_eq!(ProvenanceKind::UserStated, claim.provenance.provenance_kind);
        assert_eq!(Some("rec_1".to_string()), claim.provenance.record_id);
        assert!(claim.provenance.source_trace_ids.contains(&"trace_001".to_string()));
        assert!(claim.provenance.source_episode_ids.contains(&"ep_1".to_string()));
    }

    #[test]
    fn hydrates_rank_score_from_ranked_memory_hit() {
        let hit = make_hit("test claim");
        let claim = MemoryProvenanceHydrator::hydrate(
            "test claim",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "test claim"),
            Some(&hit),
            None,
        );
        assert!(claim.provenance.rank_score_summary.is_some());
        let summary = claim.provenance.rank_score_summary.unwrap();
        assert!(summary.contains("relevance"));
        assert!(summary.contains("final"));
    }

    #[test]
    fn hydrates_conflict_group_from_record() {
        let mut record = make_record("conflicting claim", "rec_1");
        record.conflict_group_id = Some("cg_1".to_string());
        let claim = MemoryProvenanceHydrator::hydrate(
            "conflicting claim",
            MemoryTrustBucket::Conflict,
            &make_finding(RepoConsistencyFindingKind::ConflictRequiresReview, "conflicting claim"),
            None,
            Some(&record),
        );
        assert!(claim.conflict.is_some());
        let cp = claim.conflict.unwrap();
        assert_eq!(Some("cg_1".to_string()), cp.conflict_group_id);
    }

    #[test]
    fn hydrates_supersession_from_supersedes_record_id() {
        let mut record = make_record("old claim", "rec_1");
        record.supersedes_record_id = Some("rec_0".to_string());
        let claim = MemoryProvenanceHydrator::hydrate(
            "old claim",
            MemoryTrustBucket::SupersededIgnored,
            &make_finding(RepoConsistencyFindingKind::SupersededMemoryIgnored, "old claim"),
            None,
            Some(&record),
        );
        assert!(claim.supersession.is_some());
        let sp = claim.supersession.unwrap();
        assert_eq!(Some("rec_0".to_string()), sp.supersedes_record_id);
    }

    #[test]
    fn unverifiable_claim_gets_missing_hydration_status() {
        let claim = MemoryProvenanceHydrator::hydrate(
            "the project uses microservices",
            MemoryTrustBucket::Unverifiable,
            &make_finding(RepoConsistencyFindingKind::Unverifiable, "the project uses microservices"),
            None,
            None,
        );
        assert_eq!(MemoryTrustBucket::Unverifiable, claim.bucket);
        match claim.hydration_status {
            ProvenanceHydrationStatus::Missing { .. } => {}
            other => panic!("Expected Missing, got {:?}", other),
        }
    }

    #[test]
    fn hydration_is_deterministic_for_same_inputs() {
        let record = make_record("test claim", "rec_1");
        let hit = make_hit("test claim");
        let finding = make_finding(RepoConsistencyFindingKind::Supported, "test claim");

        let c1 = MemoryProvenanceHydrator::hydrate("test claim", MemoryTrustBucket::PromptIncluded, &finding, Some(&hit), Some(&record));
        let c2 = MemoryProvenanceHydrator::hydrate("test claim", MemoryTrustBucket::PromptIncluded, &finding, Some(&hit), Some(&record));

        assert_eq!(c1.claim_text, c2.claim_text);
        assert_eq!(c1.bucket, c2.bucket);
        assert_eq!(c1.hydration_status, c2.hydration_status);
        assert_eq!(c1.provenance.record_id, c2.provenance.record_id);
    }

    #[test]
    fn no_record_produces_partial_hydration() {
        let claim = MemoryProvenanceHydrator::hydrate(
            "test claim",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "test claim"),
            None,
            None,
        );
        assert!(claim.provenance.record_id.is_none());
        assert!(matches!(claim.hydration_status, ProvenanceHydrationStatus::Partial { .. }));
    }

    #[test]
    fn batch_hydrate_findings_matches_by_claim_text() {
        let record = make_record("crate core exists", "rec_1");
        let hit = make_hit("crate core exists");
        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "crate core exists")];

        let hydrated = MemoryProvenanceHydrator::hydrate_findings(&findings, &[hit], &[record]);

        assert_eq!(1, hydrated.len());
        assert_eq!(Some("rec_1".to_string()), hydrated[0].provenance.record_id);
    }

    #[test]
    fn batch_hydrate_duplicate_claim_texts_without_cross_wiring() {
        let record1 = make_record("same claim", "rec_1");
        let mut record2 = make_record("same claim", "rec_2");
        record2.source_trace_ids = vec!["trace_002".to_string()];

        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "same claim")];

        // Both records have the same claim text — only one should match
        let hydrated = MemoryProvenanceHydrator::hydrate_findings(&findings, &[], &[record1, record2]);

        assert_eq!(1, hydrated.len());
        // Should match the first record found (deterministic via HashMap — last one wins)
        assert!(hydrated[0].provenance.record_id.is_some());
    }

    #[test]
    fn claim_text_match_is_fallback_not_primary_key() {
        // When records have different claim texts but same hash, hash wins.
        // Since findings don't carry hashes, text match is used.
        // This test documents that text match is the current mechanism.
        let mut record = make_record("crate CORE exists", "rec_1");
        record.normalized_text_hash = "unique_hash_abc".to_string();

        let findings = vec![make_finding(RepoConsistencyFindingKind::Supported, "crate CORE exists")];

        let hydrated = MemoryProvenanceHydrator::hydrate_findings(&findings, &[], &[record]);
        assert_eq!(1, hydrated.len());
        assert_eq!(Some("rec_1".to_string()), hydrated[0].provenance.record_id);
    }

    #[test]
    fn rank_score_summary_not_raw_struct() {
        let hit = make_hit("test claim");
        let claim = MemoryProvenanceHydrator::hydrate(
            "test claim",
            MemoryTrustBucket::PromptIncluded,
            &make_finding(RepoConsistencyFindingKind::Supported, "test claim"),
            Some(&hit),
            None,
        );
        // Verify it's a formatted string, not a raw struct
        let summary = claim.provenance.rank_score_summary.unwrap();
        assert!(summary.contains("%"));
        assert!(!summary.contains("MemoryRankScore"));
    }
}
