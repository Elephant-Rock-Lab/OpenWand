//! Memory UI service — bridges MemoryStore to Dioxus UI.

use crate::ui::memory_dto::{UiMemoryPanel, UiMemoryRecord};
use openwand_memory::MemoryStore;

/// Build a memory panel from the memory store.
pub async fn build_memory_panel(store: &dyn MemoryStore) -> Result<UiMemoryPanel, String> {
    let records = store
        .list_active_records()
        .await
        .map_err(|e| e.to_string())?;

    let active_count = records.len();
    let ui_records: Vec<UiMemoryRecord> = records
        .into_iter()
        .map(|r| {
            let is_active = r.superseded_by.is_none()
                && r.valid_until.map_or(true, |v| v > chrono::Utc::now());
            UiMemoryRecord {
                record_id: r.record_id,
                claim: r.claim,
                kind: format!("{:?}", r.kind).to_lowercase(),
                confidence: r.confidence,
                status: if is_active { "active" } else { "superseded" }.into(),
                source_count: r.source_episode_ids.len(),
                source_trace_ids: r.source_trace_ids,
                created_at: r.created_at.timestamp(),
                superseded_by: r.superseded_by,
            }
        })
        .collect();

    Ok(UiMemoryPanel {
        total_records: ui_records.len(),
        active_count,
        records: ui_records,
    })
}
