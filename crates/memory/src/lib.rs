//! OpenWand Memory — retrieval and knowledge projection.
//!
//! Session reads memory via `MemoryReadStore`.
//! Memory extraction proposes; deterministic policy accepts.
//! Trace provenance authorizes.

pub mod conflict;
pub mod dedup;
pub mod error;
pub mod evidence;
pub mod extractor;
pub mod in_memory;
pub mod memory_store;
pub mod prompt_assembly;
pub mod panel_view;
pub mod provenance_hydration;
pub mod trace_relation_hydration;
pub mod evaluation;
pub mod evaluation_judge;
pub mod evaluation_report;
pub mod evaluation_coverage;
pub mod evaluation_delta;
pub mod verification_signal;
pub mod governance;
pub mod provenance;
pub mod query;
pub mod ranking;
pub mod repo_consistency;
pub mod retrieval;
pub mod store;
pub mod supersession;
pub mod types;

#[cfg(feature = "sqlite")]
pub mod sqlite_schema;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;

#[cfg(feature = "testing")]
pub mod testing;

pub use error::*;
pub use extractor::*;
pub use prompt_assembly::*;
pub use in_memory::*;
pub use memory_store::*;
pub use query::*;
pub use retrieval::*;
pub use store::*;
pub use types::*;

#[cfg(feature = "sqlite")]
pub use sqlite_store::SqliteMemoryStore;
