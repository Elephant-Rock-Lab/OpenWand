//! UI session service — the bridge between store and UI.
//!
//! The UI consumes this service, never the raw store.
//! This is the composition boundary where store types become UI types.

use crate::ui::dto::{
    CreateSessionRequest, UiMessage, UiMessageRole, UiSessionSummary, UiSessionView,
};
use openwand_core::SessionId;
use openwand_store::{
    NewSessionRecord, SessionListFilter, SessionRegistryStore, SessionRegistryUpdate,
};
use std::sync::Arc;

/// Error type for UI session operations.
#[derive(Debug, thiserror::Error)]
pub enum UiServiceError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Store error: {0}")]
    Store(#[from] openwand_store::StoreError),
    #[error("Internal: {0}")]
    Internal(String),
}

/// The UI session service. Wraps the store registry and adds UI logic.
pub struct UiSessionService {
    registry: Arc<dyn SessionRegistryStore>,
}

impl UiSessionService {
    pub fn new(registry: Arc<dyn SessionRegistryStore>) -> Self {
        Self { registry }
    }

    /// List all non-archived sessions, most recently updated first.
    pub fn list_sessions(&self) -> Result<Vec<UiSessionSummary>, UiServiceError> {
        let summaries = self
            .registry
            .list_sessions(SessionListFilter {
                include_archived: false,
                limit: Some(50),
            })
            .map_err(UiServiceError::Store)?;

        Ok(summaries
            .into_iter()
            .map(|s| UiSessionSummary {
                session_id: s.session_id,
                title: s.title,
                status: s.status,
                updated_at: s.updated_at,
                last_message_preview: s.last_message_preview,
                model: s.model,
                current_phase: s.current_phase,
            })
            .collect())
    }

    /// Create a new session. Returns the summary for immediate display.
    pub fn create_session(
        &self,
        request: CreateSessionRequest,
    ) -> Result<UiSessionSummary, UiServiceError> {
        let session_id = SessionId::new().to_string();

        let record = self
            .registry
            .create_session(NewSessionRecord {
                session_id: session_id.clone(),
                title: request.title,
                provider: request.provider,
                model: request.model,
                base_url: request.base_url,
                working_directory: request.working_directory,
                interaction_mode: request.interaction_mode,
            })
            .map_err(UiServiceError::Store)?;

        Ok(UiSessionSummary {
            session_id: record.session_id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
            model: record.model,
            current_phase: record.current_phase,
        })
    }

    /// Open a session for viewing. Returns the full view including messages.
    ///
    /// For 02b-2, messages are loaded from the registry's last known state.
    /// Full trace replay is deferred to 02b-4.
    pub fn open_session(&self, session_id: &str) -> Result<UiSessionView, UiServiceError> {
        let record = self
            .registry
            .get_session(session_id)
            .map_err(UiServiceError::Store)?
            .ok_or_else(|| UiServiceError::NotFound(session_id.to_string()))?;

        // Update last_opened_at
        let now = chrono::Utc::now().timestamp();
        let update = SessionRegistryUpdate {
            session_id: session_id.to_string(),
            // We'd like to update last_opened_at but our current schema doesn't
            // have it in the update struct. For now, just touch updated_at.
            ..Default::default()
        };
        let _ = self.registry.update_session(update);

        // For 02b-2, we return an empty message list.
        // Full trace→message replay belongs in 02b-4.
        let messages = Vec::new();

        Ok(UiSessionView {
            summary: UiSessionSummary {
                session_id: record.session_id,
                title: record.title,
                status: record.status,
                updated_at: record.updated_at,
                last_message_preview: record.last_message_preview,
                model: record.model,
                current_phase: record.current_phase,
            },
            messages,
            interaction_mode: record.interaction_mode,
            current_step: record.current_step,
            provider: record.provider,
            base_url: record.base_url,
            working_directory: record.working_directory,
        })
    }
}
