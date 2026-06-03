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
pub mod workflow_proposal_builder;
pub mod workflow_proposal_review;
pub mod workflow_proposal_validation;
pub mod workflow_readiness;
pub mod workflow_execution_gate;
pub mod workflow_run;
pub mod workflow_run_lifecycle;
pub mod workflow_run_validation;
pub mod workflow_readiness_validation;
pub mod workflow_readiness_evaluator;
pub mod tool_intent_resolution;
