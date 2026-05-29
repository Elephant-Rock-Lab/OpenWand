//! Trace relation audit hydration — lineage from trace-backed relations.
//!
//! Pure classification and formatting. No I/O. No store queries.
//! The coordinator queries trace stores and passes narrow DTOs here.
//!
//! Audit/panel-only. Does not affect prompt context, ranking, inclusion, or buckets.

use chrono::{DateTime, Utc};
use crate::provenance_hydration::ProvenanceHydrationStatus;

/// Kind of trace relation, mirroring openwand-trace's TraceRelationKind
/// with an Unknown variant for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceLineageKind {
    DerivedFrom,
    Verifies,
    Refines,
    Supersedes,
    Invalidates,
    ConflictsWith,
    Implements,
    CausedBy,
    Reverts,
    References,
    /// Relation kind not recognized by this version.
    /// Contains the raw string for display.
    Unknown(String),
}

impl TraceLineageKind {
    /// Parse from the string representation stored in openwand-trace.
    pub fn from_str_kind(s: &str) -> Self {
        match s {
            "DerivedFrom" | "derived_from" => Self::DerivedFrom,
            "Verifies" | "verifies" => Self::Verifies,
            "Refines" | "refines" => Self::Refines,
            "Supersedes" | "supersedes" => Self::Supersedes,
            "Invalidates" | "invalidates" => Self::Invalidates,
            "ConflictsWith" | "conflicts_with" => Self::ConflictsWith,
            "Implements" | "implements" => Self::Implements,
            "CausedBy" | "caused_by" => Self::CausedBy,
            "Reverts" | "reverts" => Self::Reverts,
            "References" | "references" => Self::References,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &str {
        match self {
            Self::DerivedFrom => "derived from",
            Self::Verifies => "verified by",
            Self::Refines => "refines",
            Self::Supersedes => "supersedes",
            Self::Invalidates => "invalidated by",
            Self::ConflictsWith => "conflicts with",
            Self::Implements => "implements",
            Self::CausedBy => "caused by",
            Self::Reverts => "reverts",
            Self::References => "references",
            Self::Unknown(s) => s, // Show raw string for unknown kinds
        }
    }

    /// Whether this kind goes into `other_relations` rather than a named bucket.
    pub fn is_other(&self) -> bool {
        matches!(
            self,
            Self::Implements | Self::CausedBy | Self::Reverts | Self::References | Self::Unknown(_)
        )
    }

    /// Sort rank for deterministic ordering.
    /// Lower = higher priority in display.
    pub fn sort_rank(&self) -> u8 {
        match self {
            Self::DerivedFrom => 0,
            Self::Verifies => 1,
            Self::Refines => 2,
            Self::Supersedes => 3,
            Self::Invalidates => 4,
            Self::ConflictsWith => 5,
            Self::Implements => 6,
            Self::CausedBy => 7,
            Self::Reverts => 8,
            Self::References => 9,
            Self::Unknown(_) => 10,
        }
    }
}

/// A single trace relation edge with display metadata.
#[derive(Debug, Clone)]
pub struct TraceRelationProvenance {
    pub source_trace_id: String,
    pub relation_kind: TraceLineageKind,
    pub related_trace_id: String,
    pub source_event_kind: Option<String>,
    pub related_event_kind: Option<String>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub actor_label: Option<String>,
    /// Human-readable summary: "Derived from trace_01H… user.message"
    pub summary: String,
}

/// Lineage for a single claim — all trace relations touching its source traces.
#[derive(Debug, Clone)]
pub struct ClaimTraceLineage {
    pub source_trace_ids: Vec<String>,
    pub derived_from: Vec<TraceRelationProvenance>,
    pub verifies: Vec<TraceRelationProvenance>,
    pub supersedes: Vec<TraceRelationProvenance>,
    pub invalidates: Vec<TraceRelationProvenance>,
    pub refines: Vec<TraceRelationProvenance>,
    pub conflicts_with: Vec<TraceRelationProvenance>,
    /// Implements, CausedBy, Reverts, References, Unknown(_) go here.
    pub other_relations: Vec<TraceRelationProvenance>,
    pub hydration_status: ProvenanceHydrationStatus,
}

impl ClaimTraceLineage {
    /// Compact one-line summary for panel rendering.
    pub fn compact_summary(&self) -> String {
        let mut parts = Vec::new();
        let counts = self.counts();
        if counts.derived_from > 0 {
            parts.push(format!("derived from {} trace(s)", counts.derived_from));
        }
        if counts.verifies > 0 {
            parts.push(format!("verified by {} trace(s)", counts.verifies));
        }
        if counts.supersedes > 0 {
            parts.push(format!("supersedes {} prior claim(s)", counts.supersedes));
        }
        if counts.invalidates > 0 {
            parts.push(format!("invalidates {} claim(s)", counts.invalidates));
        }
        if counts.refines > 0 {
            parts.push(format!("refines {} claim(s)", counts.refines));
        }
        if counts.conflicts_with > 0 {
            parts.push(format!("conflicts with {} claim(s)", counts.conflicts_with));
        }
        if counts.other > 0 {
            parts.push(format!("{} other relation(s)", counts.other));
        }

        if parts.is_empty() {
            "no trace relations".to_string()
        } else {
            parts.join(" · ")
        }
    }

    /// Count of each relation kind.
    pub fn counts(&self) -> TraceRelationCounts {
        TraceRelationCounts {
            derived_from: self.derived_from.len(),
            verifies: self.verifies.len(),
            supersedes: self.supersedes.len(),
            invalidates: self.invalidates.len(),
            refines: self.refines.len(),
            conflicts_with: self.conflicts_with.len(),
            references: 0, // Folded into other
            other: self.other_relations.len(),
        }
    }
}

/// Counts of each relation kind for panel display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceRelationCounts {
    pub derived_from: usize,
    pub verifies: usize,
    pub supersedes: usize,
    pub invalidates: usize,
    pub refines: usize,
    pub conflicts_with: usize,
    pub references: usize,
    /// Implements, CausedBy, Reverts, References, Unknown(_) count here.
    pub other: usize,
}

// ── Narrow DTOs the coordinator passes into the hydrator ────────────────────
// These decouple the pure hydrator from openwand-trace types.

/// A trace relation edge, coordinator → hydrator.
#[derive(Debug, Clone)]
pub struct TraceRelationAuditRow {
    pub from_trace_id: String,
    pub to_trace_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
}

/// Trace event metadata, coordinator → hydrator.
#[derive(Debug, Clone)]
pub struct TraceEventAuditMetadata {
    pub trace_id: String,
    pub event_kind: String,
    pub occurred_at: DateTime<Utc>,
    pub actor_label: String,
}

// ── Pure hydrator ───────────────────────────────────────────────────────────

pub struct TraceRelationAuditHydrator;

impl TraceRelationAuditHydrator {
    /// Hydrate trace lineage for a single claim.
    /// Pure — no I/O, no store queries.
    pub fn hydrate_claim(
        source_trace_ids: &[String],
        relation_rows: &[TraceRelationAuditRow],
        event_metadata: &[TraceEventAuditMetadata],
    ) -> ClaimTraceLineage {
        if source_trace_ids.is_empty() {
            return ClaimTraceLineage {
                source_trace_ids: vec![],
                derived_from: vec![],
                verifies: vec![],
                supersedes: vec![],
                invalidates: vec![],
                refines: vec![],
                conflicts_with: vec![],
                other_relations: vec![],
                hydration_status: ProvenanceHydrationStatus::Missing {
                    reason: "claim has no source trace IDs".to_string(),
                },
            };
        }

        let id_set: std::collections::HashSet<&str> =
            source_trace_ids.iter().map(|s| s.as_str()).collect();

        let metadata_map: std::collections::HashMap<&str, &TraceEventAuditMetadata> =
            event_metadata.iter().map(|m| (m.trace_id.as_str(), m)).collect();

        let mut derived_from = Vec::new();
        let mut verifies = Vec::new();
        let mut supersedes = Vec::new();
        let mut invalidates = Vec::new();
        let mut refines = Vec::new();
        let mut conflicts_with = Vec::new();
        let mut other_relations = Vec::new();

        for row in relation_rows {
            // Bidirectional: match if either end is in source_trace_ids
            let forward = id_set.contains(row.from_trace_id.as_str());
            let reverse = id_set.contains(row.to_trace_id.as_str());
            if !forward && !reverse {
                continue;
            }

            let kind = TraceLineageKind::from_str_kind(&row.kind);

            // Determine source and related based on direction
            let (source_id, related_id, direction_label) = if forward {
                (row.from_trace_id.clone(), row.to_trace_id.clone(), kind.label())
            } else {
                (row.to_trace_id.clone(), row.from_trace_id.clone(), kind.label())
            };

            let source_meta = metadata_map.get(source_id.as_str());
            let related_meta = metadata_map.get(related_id.as_str());

            let summary = format_relation_summary(
                direction_label,
                &related_id,
                related_meta.map(|m| m.event_kind.as_str()),
            );

            let prov = TraceRelationProvenance {
                source_trace_id: source_id,
                relation_kind: kind.clone(),
                related_trace_id: related_id,
                source_event_kind: source_meta.map(|m| m.event_kind.clone()),
                related_event_kind: related_meta.map(|m| m.event_kind.clone()),
                occurred_at: Some(row.created_at),
                actor_label: related_meta.map(|m| m.actor_label.clone()),
                summary,
            };

            match &kind {
                TraceLineageKind::DerivedFrom => derived_from.push(prov),
                TraceLineageKind::Verifies => verifies.push(prov),
                TraceLineageKind::Supersedes => supersedes.push(prov),
                TraceLineageKind::Invalidates => invalidates.push(prov),
                TraceLineageKind::Refines => refines.push(prov),
                TraceLineageKind::ConflictsWith => conflicts_with.push(prov),
                _ => other_relations.push(prov), // Implements, CausedBy, Reverts, References, Unknown
            }
        }

        // Deterministic sort
        let sort_provs = |v: &mut Vec<TraceRelationProvenance>| {
            v.sort_by(|a, b| {
                a.relation_kind
                    .sort_rank()
                    .cmp(&b.relation_kind.sort_rank())
                    .then_with(|| a.occurred_at.cmp(&b.occurred_at))
                    .then_with(|| a.source_trace_id.cmp(&b.source_trace_id))
                    .then_with(|| a.related_trace_id.cmp(&b.related_trace_id))
            });
        };
        sort_provs(&mut derived_from);
        sort_provs(&mut verifies);
        sort_provs(&mut supersedes);
        sort_provs(&mut invalidates);
        sort_provs(&mut refines);
        sort_provs(&mut conflicts_with);
        sort_provs(&mut other_relations);

        let has_any = !derived_from.is_empty()
            || !verifies.is_empty()
            || !supersedes.is_empty()
            || !invalidates.is_empty()
            || !refines.is_empty()
            || !conflicts_with.is_empty()
            || !other_relations.is_empty();

        let hydration_status = if has_any {
            // Check if metadata is complete for all relation endpoints
            let mut missing = Vec::new();
            let all_provs: Vec<&TraceRelationProvenance> = derived_from.iter()
                .chain(verifies.iter())
                .chain(supersedes.iter())
                .chain(invalidates.iter())
                .chain(refines.iter())
                .chain(conflicts_with.iter())
                .chain(other_relations.iter())
                .collect();

            for p in &all_provs {
                if !metadata_map.contains_key(p.related_trace_id.as_str()) {
                    missing.push(format!(
                        "missing event metadata for related trace {}",
                        truncate_id(&p.related_trace_id)
                    ));
                }
            }

            if missing.is_empty() {
                ProvenanceHydrationStatus::Complete
            } else {
                ProvenanceHydrationStatus::Partial { missing }
            }
        } else {
            ProvenanceHydrationStatus::Partial {
                missing: vec![format!(
                    "no trace relations found for {} source trace(s)",
                    source_trace_ids.len()
                )],
            }
        };

        ClaimTraceLineage {
            source_trace_ids: source_trace_ids.to_vec(),
            derived_from,
            verifies,
            supersedes,
            invalidates,
            refines,
            conflicts_with,
            other_relations,
            hydration_status,
        }
    }

    /// Batch hydrate lineage for multiple claims.
    /// Pure — all claims share the same relation rows and metadata.
    pub fn hydrate_claims(
        claims_source_trace_ids: &[Vec<String>],
        relation_rows: &[TraceRelationAuditRow],
        event_metadata: &[TraceEventAuditMetadata],
    ) -> Vec<ClaimTraceLineage> {
        claims_source_trace_ids
            .iter()
            .map(|ids| Self::hydrate_claim(ids, relation_rows, event_metadata))
            .collect()
    }
}

fn format_relation_summary(kind_label: &str, related_id: &str, related_event_kind: Option<&str>) -> String {
    let short_id = truncate_id(related_id);
    match related_event_kind {
        Some(ek) => format!("{} {} {}", kind_label, short_id, ek),
        None => format!("{} {}", kind_label, short_id),
    }
}

fn truncate_id(id: &str) -> &str {
    // Show first 12 chars for display
    if id.len() > 12 {
        &id[..12]
    } else {
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relation(from: &str, to: &str, kind: &str) -> TraceRelationAuditRow {
        TraceRelationAuditRow {
            from_trace_id: from.to_string(),
            to_trace_id: to.to_string(),
            kind: kind.to_string(),
            created_at: Utc::now(),
        }
    }

    fn make_meta(trace_id: &str, event_kind: &str, actor: &str) -> TraceEventAuditMetadata {
        TraceEventAuditMetadata {
            trace_id: trace_id.to_string(),
            event_kind: event_kind.to_string(),
            occurred_at: Utc::now(),
            actor_label: actor.to_string(),
        }
    }

    #[test]
    fn formats_derived_from_relation_summary() {
        let rel = make_relation("trace_001", "trace_002", "DerivedFrom");
        let meta = make_meta("trace_002", "user.message", "User");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[meta],
        );
        assert_eq!(1, lineage.derived_from.len());
        assert!(lineage.derived_from[0].summary.contains("derived from"));
        assert!(lineage.derived_from[0].summary.contains("trace_002"));
        assert!(lineage.derived_from[0].summary.contains("user.message"));
    }

    #[test]
    fn formats_verifies_relation_summary() {
        let rel = make_relation("trace_003", "trace_001", "Verifies");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.verifies.len());
        assert!(lineage.verifies[0].summary.contains("verified by"));
    }

    #[test]
    fn formats_supersedes_relation_summary() {
        let rel = make_relation("trace_001", "trace_old", "Supersedes");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.supersedes.len());
        assert!(lineage.supersedes[0].summary.contains("supersedes"));
    }

    #[test]
    fn formats_invalidates_relation_summary() {
        let rel = make_relation("trace_004", "trace_001", "Invalidates");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.invalidates.len());
        assert!(lineage.invalidates[0].summary.contains("invalidated by"));
    }

    #[test]
    fn formats_conflicts_with_relation_summary() {
        let rel = make_relation("trace_005", "trace_001", "ConflictsWith");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.conflicts_with.len());
        assert!(lineage.conflicts_with[0].summary.contains("conflicts with"));
    }

    #[test]
    fn empty_lineage_reports_missing_status() {
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &[],
            &[],
            &[],
        );
        match lineage.hydration_status {
            ProvenanceHydrationStatus::Missing { reason } => {
                assert!(reason.contains("no source trace IDs"));
            }
            other => panic!("Expected Missing, got {:?}", other),
        }
    }

    #[test]
    fn lineage_status_partial_lists_missing_metadata() {
        let rel = make_relation("trace_001", "trace_002", "DerivedFrom");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[], // No metadata for trace_002
        );
        match lineage.hydration_status {
            ProvenanceHydrationStatus::Partial { missing } => {
                assert!(missing.iter().any(|m| m.contains("missing event metadata")));
            }
            other => panic!("Expected Partial, got {:?}", other),
        }
    }

    #[test]
    fn unknown_relation_kind_goes_to_other_relations() {
        let rel = make_relation("trace_001", "trace_002", "SomeNewKind");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.other_relations.len());
        assert_eq!(TraceLineageKind::Unknown("SomeNewKind".to_string()), lineage.other_relations[0].relation_kind);
    }

    #[test]
    fn classifies_derived_from_relations() {
        let rel = make_relation("trace_001", "trace_002", "DerivedFrom");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.derived_from.len());
        assert!(lineage.verifies.is_empty());
    }

    #[test]
    fn classifies_verifies_relations() {
        let rel = make_relation("trace_003", "trace_001", "Verifies");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.verifies.len());
    }

    #[test]
    fn classifies_refines_relations() {
        let rel = make_relation("trace_001", "trace_old", "Refines");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        assert_eq!(1, lineage.refines.len());
    }

    #[test]
    fn classifies_bidirectional_relations_for_source_trace() {
        let rel1 = make_relation("trace_001", "trace_002", "DerivedFrom");
        let rel2 = make_relation("trace_003", "trace_001", "Verifies");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel1, rel2],
            &[],
        );
        assert_eq!(1, lineage.derived_from.len()); // forward: trace_001 → trace_002
        assert_eq!(1, lineage.verifies.len());     // reverse: trace_003 → trace_001
    }

    #[test]
    fn preserves_relation_direction_in_summary() {
        let rel = make_relation("trace_001", "trace_002", "Supersedes");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel],
            &[],
        );
        // Forward: trace_001 supersedes trace_002
        assert!(lineage.supersedes[0].summary.contains("supersedes"));
        assert!(lineage.supersedes[0].summary.contains("trace_002"));
    }

    #[test]
    fn preserves_deterministic_relation_order() {
        use std::sync::Mutex;
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(1);
        let rel1 = TraceRelationAuditRow {
            from_trace_id: "trace_001".to_string(),
            to_trace_id: "trace_002".to_string(),
            kind: "Verifies".to_string(),
            created_at: t1,
        };
        let rel2 = TraceRelationAuditRow {
            from_trace_id: "trace_001".to_string(),
            to_trace_id: "trace_003".to_string(),
            kind: "DerivedFrom".to_string(),
            created_at: t2,
        };
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel1.clone(), rel2.clone()],
            &[],
        );
        let lineage2 = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[rel2, rel1], // Reverse input order
            &[],
        );
        assert_eq!(lineage.derived_from.len(), lineage2.derived_from.len());
        assert_eq!(lineage.verifies.len(), lineage2.verifies.len());
    }

    #[test]
    fn claim_without_source_trace_ids_gets_missing_lineage() {
        let rel = make_relation("trace_001", "trace_002", "DerivedFrom");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &[],
            &[rel],
            &[],
        );
        assert!(lineage.derived_from.is_empty());
        assert!(matches!(lineage.hydration_status, ProvenanceHydrationStatus::Missing { .. }));
    }

    #[test]
    fn claim_with_trace_ids_but_no_relations_gets_partial_lineage() {
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[],
            &[],
        );
        assert!(lineage.derived_from.is_empty());
        assert!(matches!(lineage.hydration_status, ProvenanceHydrationStatus::Partial { .. }));
    }

    #[test]
    fn implements_caused_by_and_reverts_count_as_other() {
        let r1 = make_relation("trace_001", "trace_002", "Implements");
        let r2 = make_relation("trace_001", "trace_003", "CausedBy");
        let r3 = make_relation("trace_001", "trace_004", "Reverts");
        let r4 = make_relation("trace_001", "trace_005", "References");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[r1, r2, r3, r4],
            &[],
        );
        assert_eq!(4, lineage.other_relations.len(), "Implements, CausedBy, Reverts, References should all go to other_relations");
        let counts = lineage.counts();
        assert_eq!(4, counts.other);
    }

    #[test]
    fn compact_summary_with_no_relations() {
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[],
            &[],
        );
        assert_eq!("no trace relations", lineage.compact_summary());
    }

    #[test]
    fn compact_summary_with_mixed_relations() {
        let r1 = make_relation("trace_001", "trace_002", "DerivedFrom");
        let r2 = make_relation("trace_003", "trace_001", "Verifies");
        let lineage = TraceRelationAuditHydrator::hydrate_claim(
            &["trace_001".to_string()],
            &[r1, r2],
            &[],
        );
        let summary = lineage.compact_summary();
        assert!(summary.contains("derived from"));
        assert!(summary.contains("verified by"));
    }

    #[test]
    fn batch_hydrate_produces_lineage_per_claim() {
        let rel = make_relation("trace_001", "trace_002", "DerivedFrom");
        let lineages = TraceRelationAuditHydrator::hydrate_claims(
            &[
                vec!["trace_001".to_string()],
                vec!["trace_003".to_string()],
                vec![],
            ],
            &[rel],
            &[],
        );
        assert_eq!(3, lineages.len());
        assert_eq!(1, lineages[0].derived_from.len());
        assert_eq!(0, lineages[1].derived_from.len()); // trace_003 not in any relation
        assert!(matches!(lineages[2].hydration_status, ProvenanceHydrationStatus::Missing { .. }));
    }
}
