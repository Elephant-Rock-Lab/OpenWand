//! Memory projection coordinator.
//!
//! Subscribes to run completions and automatically runs memory projection.
//! Lives in the app crate because it bridges trace, memory, and session.
//!
//! Memory failures are non-fatal to the run loop.

use openwand_core::SessionId;
use openwand_memory::{
    EpisodeRole, MemoryEpisode, MemoryExtractor, MemoryStore,
};
use openwand_store::StoredEvent;
use openwand_trace::{TraceQuery, TraceStore};
use std::sync::Arc;

/// Result of a projection run.
#[derive(Debug, Clone)]
pub struct ProjectionResult {
    pub episodes_projected: usize,
    pub candidates_extracted: usize,
    pub records_accepted: usize,
    pub errors: Vec<String>,
}

impl ProjectionResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Coordinates automatic memory projection after session runs.
pub struct MemoryCoordinator {
    memory_store: Arc<dyn MemoryStore>,
    extractor: Arc<dyn MemoryExtractor>,
    trace: Arc<dyn TraceStore<StoredEvent>>,
}

impl MemoryCoordinator {
    pub fn new(
        memory_store: Arc<dyn MemoryStore>,
        extractor: Arc<dyn MemoryExtractor>,
        trace: Arc<dyn TraceStore<StoredEvent>>,
    ) -> Self {
        Self {
            memory_store,
            extractor,
            trace,
        }
    }

    /// Run projection for a session after a run completes.
    /// Errors are captured in the result, not propagated.
    pub async fn project_after_run(
        &self,
        session_id: &SessionId,
    ) -> ProjectionResult {
        let mut result = ProjectionResult {
            episodes_projected: 0,
            candidates_extracted: 0,
            records_accepted: 0,
            errors: Vec::new(),
        };

        // Scan trace entries for this session
        let query = TraceQuery {
            stream_id: Some(openwand_trace::TraceStreamId {
                scope: openwand_trace::TraceStreamScope::Session,
                id: session_id.to_string(),
            }),
            limit: Some(1000),
            ..Default::default()
        };

        let scan_result = match self.trace.scan(query).await {
            Ok(r) => r,
            Err(e) => {
                result.errors.push(format!("scan trace: {e}"));
                return result;
            }
        };

        // Project each relevant trace entry as an episode
        for entry in &scan_result.entries {
            let episode = match Self::trace_entry_to_episode(entry, session_id) {
                Some(ep) => ep,
                None => continue,
            };

            match self.memory_store.project_episode(episode).await {
                Ok(()) => result.episodes_projected += 1,
                Err(e) => result.errors.push(format!("project episode: {e}")),
            }
        }

        // Extract candidates from all episodes for this session
        let episodes = match self
            .memory_store
            .get_episodes(&session_id.to_string())
            .await
        {
            Ok(eps) => eps,
            Err(e) => {
                result.errors.push(format!("get episodes: {e}"));
                return result;
            }
        };

        let candidates = self.extractor.extract(&episodes).await;
        result.candidates_extracted = candidates.len();

        for candidate in candidates {
            match self.memory_store.accept_candidate(candidate).await {
                Ok(Some(_)) => result.records_accepted += 1,
                Ok(None) => {}
                Err(e) => result.errors.push(format!("accept candidate: {e}")),
            }
        }

        result
    }

    /// Manual rebuild: re-project everything. Idempotent by source_trace_id.
    pub async fn rebuild_from_trace(
        &self,
        session_id: &SessionId,
    ) -> ProjectionResult {
        self.project_after_run(session_id).await
    }

    /// Convert a trace entry to a memory episode, if relevant.
    fn trace_entry_to_episode(
        entry: &openwand_trace::TraceEntry<StoredEvent>,
        session_id: &SessionId,
    ) -> Option<MemoryEpisode> {
        use openwand_core::events::OpenWandTraceEvent;
        use chrono::Utc;

        let event: &OpenWandTraceEvent = &entry.event;

        match event {
            OpenWandTraceEvent::Session(
                openwand_core::events::SessionEvent::UserMessageInjected { text },
            ) => Some(MemoryEpisode {
                episode_id: format!("ep_{}", entry.id),
                source_trace_id: entry.id.0.clone(),
                session_id: session_id.to_string(),
                event_kind: "session.user_message_injected".into(),
                role: EpisodeRole::User,
                content: text.clone(),
                created_at: Utc::now(),
            }),

            OpenWandTraceEvent::Session(
                openwand_core::events::SessionEvent::AssistantMessageGenerated { text, .. },
            ) => Some(MemoryEpisode {
                episode_id: format!("ep_{}", entry.id),
                source_trace_id: entry.id.0.clone(),
                session_id: session_id.to_string(),
                event_kind: "session.assistant_message_generated".into(),
                role: EpisodeRole::Assistant,
                content: text.clone(),
                created_at: Utc::now(),
            }),

            _ => None,
        }
    }
}
