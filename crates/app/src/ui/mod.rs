//! UI layer for OpenWand desktop app.
//!
//! Contains DTOs, services, bridges, and replay. The UI consumes these types,
//! never raw store internals.

pub mod dto;
pub mod governance_actions;
pub mod governance_components;
pub mod governance_state;
pub mod inspector_components;
pub mod inspector_state;
pub mod memory_dto;
pub mod memory_service;
pub mod provider_components;
pub mod provider_config;
pub mod replay;
pub mod run_bridge;
pub mod run_dto;
pub mod session_actions;
pub mod session_components;
pub mod service;
pub mod skills_goals_components;
pub mod skills_goals_state;
pub mod task_plan_components;
pub mod task_plan_state;
pub mod workflow_proposal_components;
pub mod workflow_proposal_state;
pub mod workflow_readiness_components;
pub mod workflow_readiness_state;

pub use dto::{CreateSessionRequest, UiMessage, UiMessageRole, UiSessionSummary, UiSessionView};
pub use memory_dto::{UiFilteredMemoryPanel, UiMemoryPanelRow, UiMemoryPanelSummary};
pub use replay::UiTimelineItem;
pub use run_dto::{UiRunEvent, UiRunState, UiRunStatus};
pub use service::{UiServiceError, UiSessionService};
