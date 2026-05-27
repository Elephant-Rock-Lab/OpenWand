//! UI layer for OpenWand desktop app.
//!
//! Contains DTOs, services, and components. The UI consumes these types,
//! never raw store internals.

pub mod dto;
pub mod service;

pub use dto::{CreateSessionRequest, UiMessage, UiMessageRole, UiSessionSummary, UiSessionView};
pub use service::{UiServiceError, UiSessionService};
