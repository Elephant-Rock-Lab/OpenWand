//! OpenWand Store
//!
//! Implements trace and memory storage traits from openwand-trace and openwand-memory.
//! Owns no domain truth — only physical persistence.
//!
//! This crate is where `openwand-core` and `openwand-trace` meet:
//! `TraceStore<StoredEvent>` is the concrete seam, where `StoredEvent`
//! wraps `OpenWandTraceEvent` and implements `TraceEventEnvelope`.

pub mod envelope;
pub mod error;

pub use envelope::StoredEvent;
pub use error::StoreError;
