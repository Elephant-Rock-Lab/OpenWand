//! TraceEventEnvelope — versioning contract for trace events.

/// Versioning contract for trace events.
/// Ensures persisted events have stable kind names and schema versions
/// independent of Rust enum layout.
///
/// OpenWandTraceEvent implements this trait (in openwand-store via StoredEvent).
/// For testing, the impl is also provided directly on OpenWandTraceEvent
/// when the `testing` feature is enabled.
pub trait TraceEventEnvelope {
    /// Stable event kind name. Used for storage and queries.
    /// Must never change once an event is persisted.
    fn event_kind(&self) -> &'static str;

    /// Schema version of this event's payload.
    /// Increment when fields are added or semantics change.
    /// Old versions must remain readable.
    fn schema_version(&self) -> u16;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove the trait is object-safe enough for generic bounds.
    struct DummyEvent;
    impl TraceEventEnvelope for DummyEvent {
        fn event_kind(&self) -> &'static str { "dummy.happened" }
        fn schema_version(&self) -> u16 { 1 }
    }

    #[test]
    fn trace_event_envelope_trait_compiles() {
        let e = DummyEvent;
        assert_eq!("dummy.happened", e.event_kind());
        assert_eq!(1, e.schema_version());
    }
}
