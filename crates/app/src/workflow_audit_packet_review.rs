//! Audit packet review persistence.
//!
//! Patch 6: indexes by workflow_run, inspection, packet_hash, chain_hash, reviewer.

use std::path::Path;

use openwand_workflow::workflow_audit_packet_review::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

fn review_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("audit_packet_reviews")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    review_root(store_root).join("records")
}

pub fn save_audit_packet_review(
    store_root: &Path,
    record: &AuditPacketReview,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create review dir: {}", e))?;

    // Idempotency
    let existing = list_audit_packet_reviews(store_root)?;
    for ex in &existing {
        if ex.idempotency_key == record.idempotency_key {
            return Ok(dir.join(format!("{}.json", ex.review_id.0)));
        }
    }

    let path = dir.join(format!("{}.json", record.review_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize review: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write review: {}", e))?;

    // Indexes
    write_index(&review_root(store_root).join("by_workflow_run"),
        &record.workflow_execution_id.0, &record.review_id.0)?;
    write_index(&review_root(store_root).join("by_inspection"),
        &record.inspection_id, &record.review_id.0)?;
    write_index(&review_root(store_root).join("by_audit_packet_hash"),
        &record.audit_packet_hash, &record.review_id.0)?;
    write_index(&review_root(store_root).join("by_chain_hash"),
        &record.chain_hash, &record.review_id.0)?;

    Ok(path)
}

fn write_index(dir: &std::path::Path, key: &str, value: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create index dir: {}", e))?;
    let idx_path = dir.join(format!("{}.json", key.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect::<String>()));
    let mut existing: Vec<String> = if idx_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&idx_path).unwrap_or_default()).unwrap_or_default()
    } else { vec![] };
    if !existing.contains(&value.to_string()) {
        existing.push(value.to_string());
    }
    std::fs::write(&idx_path, serde_json::to_string(&existing).unwrap())
        .map_err(|e| format!("Failed to write index: {}", e))?;
    Ok(())
}

pub fn load_audit_packet_review(
    store_root: &Path,
    id: &AuditPacketReviewId,
) -> Result<AuditPacketReview, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read review {}: {}", id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse review {}: {}", id.0, e))
}

pub fn list_audit_packet_reviews(
    store_root: &Path,
) -> Result<Vec<AuditPacketReview>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        if entry.path().extension().map_or(false, |ext| ext == "json") {
            if let Ok(json) = std::fs::read_to_string(entry.path()) {
                if let Ok(rec) = serde_json::from_str::<AuditPacketReview>(&json) {
                    results.push(rec);
                }
            }
        }
    }
    Ok(results)
}

pub fn review_by_workflow_run(
    store_root: &Path,
    wfx: &str,
) -> Result<Vec<AuditPacketReview>, String> {
    load_index_list(store_root, &review_root(store_root).join("by_workflow_run"), wfx)
}

pub fn review_by_inspection(
    store_root: &Path,
    inspection_id: &str,
) -> Result<Vec<AuditPacketReview>, String> {
    load_index_list(store_root, &review_root(store_root).join("by_inspection"), inspection_id)
}

pub fn review_by_audit_packet_hash(
    store_root: &Path,
    hash: &str,
) -> Result<Vec<AuditPacketReview>, String> {
    load_index_list(store_root, &review_root(store_root).join("by_audit_packet_hash"), hash)
}

fn load_index_list(
    store_root: &Path,
    index_dir: &std::path::Path,
    key: &str,
) -> Result<Vec<AuditPacketReview>, String> {
    let safe_key: String = key.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    let idx_path = index_dir.join(format!("{}.json", safe_key));
    if !idx_path.exists() { return Ok(vec![]); }
    let ids: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&idx_path).unwrap_or_default()
    ).unwrap_or_default();
    let mut results = Vec::new();
    for id in ids {
        if let Ok(rec) = load_audit_packet_review(store_root, &AuditPacketReviewId(id)) {
            results.push(rec);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_audit_packet_review::*;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn test_review(suffix: &str, key: &str) -> AuditPacketReview {
        let req = AuditPacketReviewRequest {
            inspection_id: format!("weci_{}", suffix),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_audit_packet_hash: format!("pkt_{}", suffix),
            expected_chain_hash: format!("chain_{}", suffix),
            reviewer: "alice".into(),
            decision: AuditPacketReviewDecision::ReviewedWithCaveats,
            scope: "test".into(),
            caveats: vec![],
            idempotency_key: key.into(),
        };
        build_audit_packet_review(req)
    }

    #[test]
    fn review_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let rec = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec).unwrap();
        let loaded = load_audit_packet_review(&dir, &rec.review_id).unwrap();
        assert_eq!(rec.review_id, loaded.review_id);
    }

    #[test]
    fn review_by_workflow_run_returns_expected() {
        let dir = test_dir();
        let rec = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec).unwrap();
        let results = review_by_workflow_run(&dir, "wfx_t").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn review_by_inspection_returns_expected() {
        let dir = test_dir();
        let rec = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec).unwrap();
        let results = review_by_inspection(&dir, "weci_r1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn review_by_packet_hash_returns_expected() {
        let dir = test_dir();
        let rec = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec).unwrap();
        let results = review_by_audit_packet_hash(&dir, "pkt_r1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn same_idempotency_key_returns_existing_review() {
        let dir = test_dir();
        let rec1 = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec1).unwrap();
        let rec2 = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec2).unwrap();
        let all = list_audit_packet_reviews(&dir).unwrap();
        assert_eq!(1, all.len());
    }

    #[test]
    fn different_key_preserves_review_history() {
        let dir = test_dir();
        let rec1 = test_review("r1", "key1");
        save_audit_packet_review(&dir, &rec1).unwrap();
        let rec2 = test_review("r1", "key2");
        save_audit_packet_review(&dir, &rec2).unwrap();
        let all = list_audit_packet_reviews(&dir).unwrap();
        assert_eq!(2, all.len());
    }
}
