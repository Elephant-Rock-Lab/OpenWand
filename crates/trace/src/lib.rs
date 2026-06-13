//! OpenWand Trace Substrate
//!
//! Generic append-only trace store. No dependency on openwand-core.
//! Core defines `OpenWandTraceEvent`; this crate provides `TraceStore<E>`.
//! They meet at `TraceStore<OpenWandTraceEvent>` (bridged in openwand-store).

pub mod ids;
pub mod actor;
pub mod stream;
pub mod relation;
pub mod entry;
pub mod query;
pub mod append;
pub mod envelope;
pub mod store;
pub mod projector;
pub mod error;
pub mod verifier;

#[cfg(feature = "testing")]
pub mod testing;

pub use ids::*;
pub use actor::*;
pub use stream::*;
pub use relation::*;
pub use entry::*;
pub use query::*;
pub use append::*;
pub use envelope::*;
pub use store::*;
pub use projector::*;
pub use error::*;
