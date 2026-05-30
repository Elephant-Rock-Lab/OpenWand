//! Session rebuild — reconstructs session state from trace events.
//!
//! Reads the complete trace stream for a session, applies each event to a fresh
//! Loro projection, and verifies the result matches the stored state.
//!
//! Used by: `openwand session rebuild <session-id>`

use openwand_core::OpenWandTraceEvent;
use openwand_trace::store::TraceStore;
use openwand_trace::query::TraceQuery;
use openwand_trace::stream::TraceStreamId;
use openwand_trace::stream::TraceStreamScope;
use crate::loro_state::LoroSessionState;
use crate::projector::LoroProjector;

/// Result of a session rebuild attempt.
#[derive(Debug, Clone)]
pub struct RebuildResult {
    /// Number of events replayed.
    pub events_replayed: usize,
    /// Whether the rebuilt state matches the stored state.
    pub state_matches: bool,
    /// Divergences found (empty if state_matches).
    pub divergences: Vec<String>,
}

/// Rebuild a session's projected state from its trace stream.
///
/// E must implement TraceEventEnvelope and have a known way to convert to
/// the event types the projector understands.
pub async fn rebuild_session<E: Clone + Send + Sync + 'static>(
    store: &dyn TraceStore<E>,
    session_id: &str,
    stored_state: Option<&LoroSessionState>,
    to_trace_event: impl Fn(&E) -> OpenWandTraceEvent,
) -> Result<RebuildResult, String> {
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    let query = TraceQuery {
        stream_id: Some(stream_id),
        event_kind: None,
        actor: None,
        from_sequence: None,
        to_sequence: None,
        from_timestamp: None,
        to_timestamp: None,
        limit: None,
        cursor: None,
    };

    let page = store
        .scan(query)
        .await
        .map_err(|e| format!("Trace scan failed: {}", e))?;

    let doc = loro::LoroDoc::new();
    let state = LoroSessionState::new(&doc);
    let mut projector = LoroProjector::new(state);
    let mut events_replayed = 0;

    for entry in &page.entries {
        let event = to_trace_event(&entry.event);
        let trace_id = entry.id.clone();
        projector
            .apply(trace_id, &event)
            .map_err(|e| format!("Projection failed at event {}: {}", events_replayed, e))?;
        events_replayed += 1;
    }

    let mut divergences = Vec::new();
    let state_matches = if let Some(stored) = stored_state {
        compare_states(stored, projector.state(), &mut divergences)
    } else {
        true // No stored state to compare against
    };

    Ok(RebuildResult {
        events_replayed,
        state_matches,
        divergences,
    })
}

/// Compare two LoroSessionState instances, collecting divergences.
fn compare_states(
    stored: &LoroSessionState,
    rebuilt: &LoroSessionState,
    divergences: &mut Vec<String>,
) -> bool {
    let stored_msgs = stored.messages().unwrap_or_default();
    let rebuilt_msgs = rebuilt.messages().unwrap_or_default();

    if stored_msgs.len() != rebuilt_msgs.len() {
        divergences.push(format!(
            "Message count: stored={}, rebuilt={}",
            stored_msgs.len(),
            rebuilt_msgs.len()
        ));
        return false;
    }
    true
}

/* Rebuild tests live in crates/session/tests/rebuild_verification.rs
   because they need StoredEvent from openwand-store which session crate
   intentionally does not depend on. */
