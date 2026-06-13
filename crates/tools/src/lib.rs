//! OpenWand Tools — unified local + MCP dispatch seam.
//!
//! Session calls `ToolExecutor`. Never knows about local vs MCP.
//! rmcp types stay inside openwand-mcp-pool.

pub mod composite;
pub mod descriptor;
pub mod effect;
pub mod error;
pub mod executor;
pub mod local;
pub mod file_patch;
pub mod naming;
pub mod normalize;
pub mod result;
pub mod sandbox;

/// Windows NtCreateFile handle-relative file write helpers.
/// Isolated module with documented safety invariants.
#[cfg(windows)]
#[allow(dead_code)] // Used by sandbox.rs Windows impl
mod sandbox_ntapi;

pub use composite::*;
pub use descriptor::*;
pub use effect::*;
pub use error::*;
pub use executor::*;
pub use local::*;
pub use naming::*;
pub use normalize::*;
pub use result::*;
