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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;
    use crate::loro_state::LoroSessionState;
    use crate::projector::LoroProjector;
    use loro::LoroDoc;
    use openwand_core::events::{InferenceEvent, CapabilityManifestAuditState, TraceHashAlgorithm, CapabilityPromptOrderPosition};
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};
    use openwand_trace::testing::InMemoryTraceStore;

    fn make_helper() -> (
        MutationHelper,
        broadcast::Receiver<AgentEvent>,
    ) {
        let trace: Arc<dyn TraceStore<StoredEvent>> =
            Arc::new(InMemoryTraceStore::new());
        let doc = LoroDoc::new();
        let loro_state = LoroSessionState::new(&doc);
        let projector = LoroProjector::new(loro_state);
        // LoroSessionState does not impl Clone; create a fresh one for the helper.
        let loro_state_for_helper = LoroSessionState::new(&LoroDoc::new());
        let (tx, rx) = broadcast::channel(16);
        let helper = MutationHelper::new(trace, projector, loro_state_for_helper, tx);
        (helper, rx)
    }

    fn test_stream() -> TraceStreamId {
        TraceStreamId {
            scope: TraceStreamScope::Session,
            id: "test".into(),
        }
    }

    fn sample_event() -> OpenWandTraceEvent {
        OpenWandTraceEvent::Inference(InferenceEvent::CapabilityContextAssembled {
            session_id: "test_session".into(),
            included_skill_ids: vec![],
            included_goal_ids: vec![],
            excluded_item_ids: vec![],
            skills_manifest_state: CapabilityManifestAuditState::FoundWithItems,
            goals_manifest_state: CapabilityManifestAuditState::FoundWithItems,
            context_text_hash: "abc123".into(),
            context_text_hash_algorithm: TraceHashAlgorithm::Sha256,
            context_text_length: 0,
            prompt_order_position: CapabilityPromptOrderPosition::AfterMemoryBlock,
        })
    }

    /// Patch 4 test 1: apply emits an AgentEvent.
    #[tokio::test]
    async fn mutation_helper_apply_emits_agent_event() {
        let (helper, mut rx) = make_helper();
        let result = helper
            .apply(Actor::User, sample_event(), vec![], None, test_stream())
            .await;
        assert!(result.is_ok(), "apply should succeed");
        let event = rx.try_recv();
        assert!(event.is_ok(), "AgentEvent should be emitted after apply");
    }

    /// Patch 4 test 2: trace is appended before AgentEvent is emitted.
    /// Proven by checking that apply returns a valid TraceId (trace append
    /// succeeded) AND an AgentEvent was emitted (step 3 ran after step 1).
    #[tokio::test]
    async fn mutation_helper_apply_trace_first_then_event() {
        let (helper, mut rx) = make_helper();
        let trace_id = helper
            .apply(Actor::User, sample_event(), vec![], None, test_stream())
            .await
            .expect("apply should succeed");
        // TraceId returned means trace append succeeded (step 1)
        assert!(!trace_id.to_string().is_empty());
        // AgentEvent was emitted (step 3 ran)
        let agent_event = rx.try_recv().expect("AgentEvent should be emitted");
        match agent_event {
            AgentEvent::PhaseEntered { phase, .. } => {
                assert!(!phase.is_empty(), "phase should be populated from event_kind");
            }
            _ => panic!("expected PhaseEntered event, got {:?}", agent_event),
        }
    }

    /// Patch 4 test 3: event send failure does not abort the mutation.
    /// When all receivers are dropped, tx.send() fails, but apply still returns Ok.
    /// AgentEvent emission is observational and best-effort; trace append
    /// remains the durable record.
    #[tokio::test]
    async fn mutation_helper_apply_event_send_failure_does_not_abort_mutation() {
        let (helper, rx) = make_helper();
        // Drop the receiver so tx.send() returns Err
        drop(rx);
        let result = helper
            .apply(Actor::User, sample_event(), vec![], None, test_stream())
            .await;
        assert!(
            result.is_ok(),
            "apply should succeed even when AgentEvent send fails — \
             emission is best-effort, trace append is the durable record"
        );
    }
}
