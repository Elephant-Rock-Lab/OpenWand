//! OpenWand Session — 10-phase agent loop with trace-first mutation.
//!
//! Session coordinates the agent loop:
//! LLM proposes → Policy gates → Tools execute → Trace records → Loro projects → Memory derives → UI observes
//!
//! Key invariants:
//! 1. SessionRunner is the only writer.
//! 2. Trace append precedes every durable state mutation.
//! 3. Loro is rebuildable projection, never authority.
//!
//! Session consumes `Arc<dyn ToolExecutor>` and never knows about MCP.

pub mod adapters;
pub mod agent_event;
pub mod approval_recovery;
pub mod config;
pub mod error;
pub mod loro_state;
pub mod message;
pub mod mutation;
pub mod phase;
pub mod projector;
pub mod runner;
pub mod tool;

#[cfg(feature = "testing")]
pub mod testing;

pub use agent_event::*;
pub use config::*;
pub use error::*;
pub use loro_state::*;
pub use message::*;
pub use phase::*;
pub use runner::*;
pub use tool::*;
