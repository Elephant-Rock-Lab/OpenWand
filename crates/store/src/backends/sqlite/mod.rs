//! SQLite backend for OpenWand trace store.
//!
//! Uses a single serialized writer (spawn_blocking) to avoid blocking
//! the async runtime while maintaining write ordering.

pub mod hash;
pub mod migrations;
pub mod schema;
pub mod store;
pub mod writer;

pub use store::{SqliteStore, SqliteStoreConfig};
