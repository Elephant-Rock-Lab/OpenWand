//! Append command for the trace store.

use crate::actor::Actor;
use crate::relation::TraceRelationDraft;
use crate::stream::{IdempotencyKey, TraceStreamId};

/// Command to append a trace entry.
/// Callers construct this. The store assigns the rest.
#[derive(Debug, Clone)]
pub struct AppendTraceEntry<E> {
    /// Who caused this event
    pub actor: Actor,

    /// What happened
    pub event: E,

    /// How this event relates to other events
    pub relations: Vec<TraceRelationDraft>,

    /// Which stream this entry belongs to
    pub stream_id: TraceStreamId,

    /// Prevents duplicate appends on retry
    pub idempotency_key: Option<IdempotencyKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Actor;
    use crate::stream::{TraceStreamId, TraceStreamScope};

    #[test]
    fn append_trace_entry_compiles_generic() {
        // Prove AppendTraceEntry is generic over E
        let cmd: AppendTraceEntry<String> = AppendTraceEntry {
            actor: Actor::User,
            event: "hello".into(),
            relations: vec![],
            stream_id: TraceStreamId {
                scope: TraceStreamScope::Global,
                id: "test".into(),
            },
            idempotency_key: None,
        };
        assert_eq!("hello", cmd.event);
    }
}
