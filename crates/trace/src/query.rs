//! Query types for scanning the trace log.

use chrono::{DateTime, Utc};

use crate::ids::TraceId;
use crate::relation::TraceRelationKind;
use crate::stream::TraceStreamId;

/// Query parameters for scanning the trace log.
#[derive(Debug, Clone, Default)]
pub struct TraceQuery {
    pub stream_id: Option<TraceStreamId>,
    pub event_kind: Option<String>,
    pub actor: Option<ActorFilter>,
    pub from_sequence: Option<u64>,
    pub to_sequence: Option<u64>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub cursor: Option<TraceId>,
}

#[derive(Debug, Clone)]
pub enum ActorFilter {
    UserOnly,
    LlmOnly,
    SystemOnly,
    Component(String),
}

/// A page of trace query results.
#[derive(Debug, Clone)]
pub struct TracePage<E> {
    pub entries: Vec<crate::entry::TraceEntry<E>>,
    pub next_cursor: Option<TraceId>,
    pub total: usize,
}

/// Query parameters for scanning relations.
#[derive(Debug, Clone, Default)]
pub struct RelationQuery {
    pub from: Option<TraceId>,
    pub to: Option<TraceId>,
    pub kind: Option<TraceRelationKind>,
    pub depth: Option<usize>,
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_query_default_is_empty() {
        let q = TraceQuery::default();
        assert!(q.stream_id.is_none());
        assert!(q.event_kind.is_none());
        assert!(q.actor.is_none());
        assert!(q.from_sequence.is_none());
        assert!(q.to_sequence.is_none());
        assert!(q.from_timestamp.is_none());
        assert!(q.to_timestamp.is_none());
        assert!(q.limit.is_none());
        assert!(q.cursor.is_none());
    }

    #[test]
    fn relation_query_default_is_empty() {
        let q = RelationQuery::default();
        assert!(q.from.is_none());
        assert!(q.to.is_none());
        assert!(q.kind.is_none());
        assert!(q.depth.is_none());
        assert!(q.limit.is_none());
    }
}
