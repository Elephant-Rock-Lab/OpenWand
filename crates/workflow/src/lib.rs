//! OpenWand Workflow — Task plan DTOs, validation, and deterministic builder.
//!
//! Plans describe intended work. They are evidence artifacts only.
//! A plan is not execution. A reviewed plan is not an execution grant.

pub mod builder;
pub mod context;
pub mod plan;
pub mod plan_review;
pub mod validation;
pub mod workflow_proposal;
pub mod workflow_proposal_validation;
