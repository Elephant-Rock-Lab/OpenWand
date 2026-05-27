//! OpenWand Policy — deterministic trust gate for all tool actions.
//!
//! Policy evaluates tool calls against rules. It is the authority boundary
//! between "LLM wants to do X" and "X actually happens."
//!
//! Principles:
//! - Deterministic: same inputs → same outputs
//! - Conservative: when in doubt, block
//! - Fail-closed: evaluation error → block, not allow
//! - Mode-aware: InteractionMode can only raise confirmation, never lower it

pub mod builtin;
pub mod decision;
pub mod engine;
pub mod error;
pub mod eval;
pub mod mapping;
pub mod request;
pub(crate) mod risk;
pub mod rule;
pub mod tool;

pub use builtin::*;
pub use decision::*;
pub use engine::*;
pub use error::*;
pub use eval::*;
pub use mapping::*;
pub use request::*;
pub use rule::*;
pub use tool::*;
