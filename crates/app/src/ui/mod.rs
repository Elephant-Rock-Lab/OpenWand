//! UI layer for OpenWand desktop app.
//!
//! Contains DTOs, services, bridges, and replay. The UI consumes these types,
//! never raw store internals.

pub mod dto;
pub mod replay;
pub mod run_bridge;
pub mod run_dto;
pub mod service;

pub use dto::{CreateSessionRequest, UiMessage, UiMessageRole, UiSessionSummary, UiSessionView};
pub use replay::UiTimelineItem;
pub use run_dto::{UiRunEvent, UiRunState, UiRunStatus};
pub use service::{UiServiceError, UiSessionService};
