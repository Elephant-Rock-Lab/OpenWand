//! Repo consistency check — top-level module.
//!
//! Read-only comparison of memory claims against observable repo facts.
//! No mutation of Git, files, or memory.

mod claim_match;
mod classify;
mod detect_missing;
mod memory_input;
mod observe;
mod report;

pub use claim_match::*;
pub use classify::*;
pub use detect_missing::*;
pub use memory_input::*;
pub use observe::*;
pub use report::*;
