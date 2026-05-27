//! Trace projector — materialized view consumer.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::entry::TraceEntry;
use crate::ids::TraceId;
use crate::relation::TraceRelation;

/// A projection consumes trace entries and updates a materialized view.
#[async_trait]
pub trait TraceProjector<E>: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    /// Name of this projector (for checkpointing).
    fn name(&self) -> &'static str;

    /// Whether this projector cares about the given event.
    fn applies_to(&self, event: &E) -> bool;

    /// Apply a trace entry to the projection.
    async fn apply(
        &mut self,
        entry: &TraceEntry<E>,
        relations: &[TraceRelation],
    ) -> Result<(), Self::Error>;
}

/// Tracks how far a projector has processed.
pub struct ProjectionCheckpoint {
    pub projector_name: String,
    pub last_global_sequence: u64,
    pub last_trace_id: Option<TraceId>,
    pub updated_at: DateTime<Utc>,
    pub error_count: u32,
    pub last_error: Option<String>,
}
