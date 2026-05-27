//! OpenWand Store
//!
//! Implements trace and memory storage traits from openwand-trace and openwand-memory.
//! Owns no domain truth — only physical persistence.
//!
//! This crate is where `openwand-core` and `openwand-trace` meet:
//! `TraceStore<StoredEvent>` is the concrete seam, where `StoredEvent`
//! wraps `OpenWandTraceEvent` and implements `TraceEventEnvelope`.

pub mod backends;
pub mod envelope;
pub mod error;
pub mod registry;
pub mod registry_store;

pub use envelope::StoredEvent;
pub use error::StoreError;
pub use registry::{NewSessionRecord, SessionListFilter, SessionRecord, SessionRegistryUpdate, SessionSummary};
pub use registry_store::SessionRegistryStore;
