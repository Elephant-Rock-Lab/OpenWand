//! Trace entry — the core record in the append-only log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::actor::Actor;
use crate::ids::TraceId;
use crate::relation::TraceRelation;
use crate::stream::EntryHash;

/// A single append-only record in the trace log.
/// Generic over event type E — completely independent of domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry<E> {
    /// Unique ID, assigned by store
    pub id: TraceId,

    /// Which stream this entry belongs to
    pub stream_id: crate::stream::TraceStreamId,

    /// Monotonic sequence within the stream
    pub stream_sequence: u64,

    /// Global monotonic sequence across all streams
    pub global_sequence: u64,

    /// When this event occurred (wall clock)
    pub occurred_at: DateTime<Utc>,

    /// Who or what caused this event
    pub actor: Actor,

    /// The typed event payload
    pub event: E,

    /// Stable event kind name
    pub event_kind: String,

    /// Schema version of the event payload
    pub event_schema_version: u16,

    /// Schema version of the trace envelope
    pub trace_schema_version: u16,

    /// Hash of the previous entry in this stream (integrity chain)
    pub prev_hash: Option<EntryHash>,

    /// Hash of this entry (integrity check)
    pub entry_hash: EntryHash,
}

/// A trace entry with its relations loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntryWithRelations<E> {
    pub entry: TraceEntry<E>,
    pub relations: Vec<TraceRelation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{TraceStreamId, TraceStreamScope};

    /// A minimal test event type to prove genericity.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEvent {
        label: String,
        value: i32,
    }

    #[test]
    fn trace_entry_roundtrip_generic_payload() {
        let entry = TraceEntry {
            id: TraceId::new(),
            stream_id: TraceStreamId {
                scope: TraceStreamScope::Global,
                id: "test".into(),
            },
            stream_sequence: 1,
            global_sequence: 1,
            occurred_at: Utc::now(),
            actor: Actor::User,
            event: TestEvent {
                label: "test_event".into(),
                value: 42,
            },
            event_kind: "test.happened".into(),
            event_schema_version: 1,
            trace_schema_version: 1,
            prev_hash: None,
            entry_hash: EntryHash("abc123".into()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let restored: TraceEntry<TestEvent> = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, restored.id);
        assert_eq!(entry.stream_sequence, restored.stream_sequence);
        assert_eq!(entry.event, restored.event);
    }
}
