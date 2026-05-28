//! OpenWand Memory — retrieval and knowledge projection.
//!
//! Session reads memory via `MemoryReadStore`.
//! Memory extraction proposes; deterministic policy accepts.
//! Trace provenance authorizes.

pub mod error;
pub mod evidence;
pub mod extractor;
pub mod in_memory;
pub mod memory_store;
pub mod provenance;
pub mod query;
pub mod ranking;
pub mod retrieval;
pub mod store;
pub mod types;

#[cfg(feature = "sqlite")]
pub mod sqlite_schema;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;

#[cfg(feature = "testing")]
pub mod testing;

pub use error::*;
pub use extractor::*;
pub use in_memory::*;
pub use memory_store::*;
pub use query::*;
pub use retrieval::*;
pub use store::*;
pub use types::*;

#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteMemoryStore;
