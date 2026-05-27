use crate::agent_event::AgentEvent;
use crate::loro_state::LoroSessionState;
use crate::projector::LoroProjector;
use crate::SessionError;
use openwand_core::events::OpenWandTraceEvent;
use openwand_store::StoredEvent;
use openwand_trace::{Actor, AppendTraceEntry, IdempotencyKey, TraceId, TraceRelationDraft, TraceStore};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Helper for trace-first mutation pattern.
pub struct MutationHelper {
    trace: Arc<dyn TraceStore<StoredEvent>>,
    projector: tokio::sync::Mutex<LoroProjector>,
    loro_state: LoroSessionState,
    agent_event_tx: broadcast::Sender<AgentEvent>,
}

impl MutationHelper {
    pub fn new(
        trace: Arc<dyn TraceStore<StoredEvent>>,
        projector: LoroProjector,
        loro_state: LoroSessionState,
        agent_event_tx: broadcast::Sender<AgentEvent>,
    ) -> Self {
        Self {
            trace,
            projector: tokio::sync::Mutex::new(projector),
            loro_state,
            agent_event_tx,
        }
    }

    /// Apply a session mutation: trace first, then Loro projection, then AgentEvent.
    pub async fn apply(
        &self,
        actor: Actor,
        event: OpenWandTraceEvent,
        relations: Vec<TraceRelationDraft>,
        idempotency_key: Option<IdempotencyKey>,
        stream_id: openwand_trace::TraceStreamId,
    ) -> Result<TraceId, SessionError> {
        // 1. Trace append (hard stop on failure)
        let stored = StoredEvent::from(event.clone());
        let trace_id = self
            .trace
            .append(AppendTraceEntry {
                actor,
                event: stored,
                relations,
                stream_id,
                idempotency_key,
            })
            .await
            .map_err(SessionError::Trace)?;

        // 2. Loro projection (degraded on failure)
        {
            let mut projector = self.projector.lock().await;
            match projector.apply(trace_id.clone(), &event) {
                Ok(()) => {}
                Err(err) => {
                    tracing::warn!(
                        ?trace_id,
                        error = %err,
                        "Loro projection failed; trace remains authoritative"
                    );
                    self.loro_state
                        .mark_projection_stale(trace_id.clone(), err.to_string())
                        .map_err(SessionError::ProjectionStaleMarker)?;
                }
            }
        }

        // 3. AgentEvent emission (non-fatal)
        let _ = self.agent_event_tx.send(AgentEvent::PhaseEntered {
            session_id: openwand_core::SessionId::new(),
            phase: event.event_kind().to_string(),
            step: 0,
        });

        Ok(trace_id)
    }
}
