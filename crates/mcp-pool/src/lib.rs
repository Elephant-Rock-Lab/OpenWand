//! OpenWand MCP Server Pool
//!
//! Manages MCP server lifecycle, tool discovery, and tool execution.
//! Uses rmcp internally but exposes only pool-owned DTOs.
//! No rmcp types escape this crate.

pub mod config;
pub mod discovered;
pub mod error;
pub mod gateway;
pub mod pool;
pub mod result;
pub mod state;
pub mod testing;

pub use config::*;
pub use discovered::*;
pub use error::*;
pub use gateway::*;
pub use result::*;
pub use state::*;
