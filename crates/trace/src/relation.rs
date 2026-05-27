//! Trace relations — typed causal edges between entries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::TraceId;

/// A typed causal/provenance edge between trace entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelation {
    pub from: TraceId,
    pub to: TraceId,
    pub kind: TraceRelationKind,
    pub created_at: DateTime<Utc>,
}

/// Typed causal relationships between trace entries.
/// Lives in `openwand-trace` — graph substrate, not domain vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraceRelationKind {
    CausedBy,
    DerivedFrom,
    Verifies,
    Invalidates,
    Supersedes,
    Refines,
    ConflictsWith,
    Implements,
    Reverts,
    References,
}

/// A relation draft to be created alongside a new entry.
/// `from` is implicitly the new entry's ID (assigned by store).
#[derive(Debug, Clone)]
pub struct TraceRelationDraft {
    pub to: TraceId,
    pub kind: TraceRelationKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_relation_kind_roundtrip() {
        let kinds = [
            TraceRelationKind::CausedBy,
            TraceRelationKind::DerivedFrom,
            TraceRelationKind::Verifies,
            TraceRelationKind::Invalidates,
            TraceRelationKind::Supersedes,
            TraceRelationKind::Refines,
            TraceRelationKind::ConflictsWith,
            TraceRelationKind::Implements,
            TraceRelationKind::Reverts,
            TraceRelationKind::References,
        ];
        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let restored: TraceRelationKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, &restored, "round-trip failed for {json}");
        }
    }
}
