//! OpenWand app library.
//!
//! Shared types and services used by both CLI and desktop binaries.

pub mod eval_collector;
pub mod eval_compare;
pub mod eval_model;
pub mod eval_proposal;
pub mod eval_remote_push_execution;
pub mod eval_remote_push_proposal;
pub mod eval_remote_push_readiness;
pub mod eval_post_commit_verify;
pub mod eval_proposal_execution;
pub mod eval_proposal_review;
pub mod eval_readiness;
pub mod eval_reports;
pub mod eval_summary;
pub mod eval_trace;
pub mod explain;
pub mod memory_coordinator;
pub mod memory_evaluation_model;
pub mod memory_evaluation;
pub mod session_capability;
pub mod session_runtime;
pub mod task_planning;
pub mod ui;
pub mod workflow_proposal;
pub mod workflow_readiness;
pub mod workflow_execution;
pub mod workflow_session_bridge;
pub mod workflow_action_routing;
