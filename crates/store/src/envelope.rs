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
