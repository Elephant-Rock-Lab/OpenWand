//! OpenWand skill definitions — declarative capability context.
//!
//! Skills describe reusable capabilities. They are context, not authority.
//!
//! This crate provides:
//! - Skill manifest DTOs (loaded from .openwand/skills.toml)
//! - Validation and registry
//! - Read-only context projection for session consumption
//!
//! Dependencies: serde, serde_json, toml, thiserror, tracing only.
//! No dependency on openwand-session, openwand-tools, openwand-policy,
//! openwand-memory, openwand-trace, openwand-store, tokio, uuid.

pub mod context;
pub mod manifest;
pub mod registry;

pub use context::SkillContextSummary;
pub use manifest::{SkillContextKind, SkillDefinition, SkillId, SkillManifest};
pub use registry::{
    load_skill_registry, SkillRegistry, SkillValidationIssue, SkillValidationReport,
    SkillValidationSeverity,
};
