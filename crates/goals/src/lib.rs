//! OpenWand goal definitions — declarative outcome context.
//!
//! Goals describe intended outcomes. They are context, not authority.
//!
//! This crate provides:
//! - Goal manifest DTOs (loaded from .openwand/goals.toml)
//! - Validation and registry
//! - Read-only context projection for session consumption
//!
//! Dependencies: serde, serde_json, toml, thiserror, tracing only.
//! No dependency on openwand-session, openwand-tools, openwand-policy,
//! openwand-memory, openwand-trace, openwand-store, openwand-skills,
//! tokio, uuid, chrono, anyhow.

pub mod context;
pub mod manifest;
pub mod registry;

pub use context::GoalContextSummary;
pub use manifest::{GoalDefinition, GoalId, GoalManifest, GoalStatus};
pub use registry::{
    load_goal_registry, GoalRegistry, GoalValidationIssue, GoalValidationReport,
    GoalValidationSeverity,
};
