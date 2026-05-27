//! OpenWand Core — vocabulary crate
//!
//! Domain IDs, shared vocabulary enums, snapshot DTOs, and event families.
//! This crate contains types only — no logic, no async, no runtime.
//!
//! Allowed dependencies: `serde`, `serde_json`, `chrono`, `ulid`.
//! Forbidden: `tokio`, `blake3`, `uuid`, `thiserror`, `loro`, `rig`, `rmcp`,
//! and all other `openwand-*` crates.

pub mod ids;
pub mod mode;
pub mod risk;
pub mod memory_vocab;
pub mod tool_vocab;
pub mod session_vocab;
pub mod snapshots;
pub mod events;

// Flat re-exports — users write:
//   use openwand_core::{SessionId, ToolEffect, InteractionMode};
//   use openwand_core::{OpenWandTraceEvent, ToolEvent};
pub use ids::*;
pub use mode::*;
pub use risk::*;
pub use memory_vocab::*;
pub use tool_vocab::*;
pub use session_vocab::*;
pub use snapshots::*;
pub use events::*;
