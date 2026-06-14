//! Bridge: TraceEventEnvelope impl for OpenWandTraceEvent.
//!
//! Orphan rules prevent directly implementing a foreign trait for a foreign type.
//! We use a newtype wrapper (`StoredEvent`) to own the impl locally.
//! All store APIs surface `StoredEvent` — callers wrap at the boundary.

use openwand_core::OpenWandTraceEvent;
use openwand_trace::TraceEventEnvelope;

/// Newtype wrapper that bridges OpenWandTraceEvent into the trace substrate.
///
/// Usage:
/// ```ignore
/// let event = OpenWandTraceEvent::Session(SessionEvent::Started { ... });
/// let stored = StoredEvent::from(event);
/// // stored now implements TraceEventEnvelope
/// ```
#[derive(Debug, Clone)]
pub struct StoredEvent(pub OpenWandTraceEvent);

impl From<OpenWandTraceEvent> for StoredEvent {
    fn from(event: OpenWandTraceEvent) -> Self {
        Self(event)
    }
}

impl From<StoredEvent> for OpenWandTraceEvent {
    fn from(stored: StoredEvent) -> Self {
        stored.0
    }
}

impl std::ops::Deref for StoredEvent {
    type Target = OpenWandTraceEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TraceEventEnvelope for StoredEvent {
    fn event_kind(&self) -> &'static str {
        self.0.event_kind()
    }

    fn schema_version(&self) -> u16 {
        self.0.schema_version()
    }
}

/// Implement hash verification policy for StoredEvent.
///
/// Serializes the inner `OpenWandTraceEvent` (via `.0`) to match the
/// canonical JSON form used by the SQLite store during append.
/// This ensures hash recomputation produces the same value as the original
/// `compute_entry_hash` call in the writer.
impl openwand_trace::verifier::HashVerificationPolicy<StoredEvent>
    for openwand_trace::verifier::Blake3HashPolicy
{
    fn serialize_event(
        &self,
        event: &StoredEvent,
    ) -> Result<String, serde_json::Error> {
        // Serialize the inner OpenWandTraceEvent, not the wrapper.
        // This matches writer.rs: serde_json::to_string(&command.event.0)
        serde_json::to_string(&event.0)
    }

    fn compute_entry_hash(
        &self,
        global_sequence: u64,
        stream_scope: &str,
        stream_id: &str,
        stream_sequence: u64,
        event_kind: &str,
        event_payload_json: &str,
        prev_hash: Option<&openwand_trace::EntryHash>,
    ) -> openwand_trace::EntryHash {
        openwand_trace::verifier::Blake3HashPolicy::compute_hash(
            global_sequence,
            stream_scope,
            stream_id,
            stream_sequence,
            event_kind,
            event_payload_json,
            prev_hash,
        )
    }
}
