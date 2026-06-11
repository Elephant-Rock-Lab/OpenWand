//! Audit packet distribution persistence.
//!
//! Patch 6: indexes by workflow_run, review, inspection, packet_hash, destination_kind.

use std::path::Path;

use openwand_workflow::workflow_audit_packet_distribution::*;

fn dist_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("audit_packet_distributions")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    dist_root(store_root).join("records")
}

pub fn save_audit_packet_distribution(
    store_root: &Path,
    record: &AuditPacketDistribution,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create distribution dir: {}", e))?;

    // Idempotency
    let existing = list_audit_packet_distributions(store_root)?;
    for ex in &existing {
        if ex.idempotency_key == record.idempotency_key {
            return Ok(dir.join(format!("{}.json", ex.distribution_id.0)));
        }
    }

    let path = dir.join(format!("{}.json", record.distribution_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize distribution: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write distribution: {}", e))?;

    // Indexes
    write_index(&dist_root(store_root).join("by_workflow_run"),
        &record.workflow_execution_id.0, &record.distribution_id.0)?;
    write_index(&dist_root(store_root).join("by_review"),
        &record.review_id.0, &record.distribution_id.0)?;
    write_index(&dist_root(store_root).join("by_inspection"),
        &record.inspection_id, &record.distribution_id.0)?;
    write_index(&dist_root(store_root).join("by_audit_packet_hash"),
        &record.audit_packet_hash, &record.distribution_id.0)?;
    write_index(&dist_root(store_root).join("by_destination_kind"),
        &format!("{:?}", record.destination.destination_kind).to_lowercase(),
        &record.distribution_id.0)?;

    Ok(path)
}

fn write_index(dir: &std::path::Path, key: &str, value: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create index dir: {}", e))?;
    let safe_key: String = key.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    let idx_path = dir.join(format!("{}.json", safe_key));
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

pub fn load_audit_packet_distribution(
    store_root: &Path,
    id: &AuditPacketDistributionId,
) -> Result<AuditPacketDistribution, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read distribution {}: {}", id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse distribution {}: {}", id.0, e))
}

pub fn list_audit_packet_distributions(
    store_root: &Path,
) -> Result<Vec<AuditPacketDistribution>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        if entry.path().extension().is_some_and(|ext| ext == "json")
            && let Ok(json) = std::fs::read_to_string(entry.path())
                && let Ok(rec) = serde_json::from_str::<AuditPacketDistribution>(&json) {
                    results.push(rec);
                }
    }
    Ok(results)
}

pub fn distribution_by_workflow_run(
    store_root: &Path,
    wfx: &str,
) -> Result<Vec<AuditPacketDistribution>, String> {
    load_index_list(store_root, &dist_root(store_root).join("by_workflow_run"), wfx)
}

pub fn distribution_by_review(
    store_root: &Path,
    review_id: &str,
) -> Result<Vec<AuditPacketDistribution>, String> {
    load_index_list(store_root, &dist_root(store_root).join("by_review"), review_id)
}

pub fn distribution_by_audit_packet_hash(
    store_root: &Path,
    hash: &str,
) -> Result<Vec<AuditPacketDistribution>, String> {
    load_index_list(store_root, &dist_root(store_root).join("by_audit_packet_hash"), hash)
}

pub fn distribution_by_inspection(
    store_root: &Path,
    inspection_id: &str,
) -> Result<Vec<AuditPacketDistribution>, String> {
    load_index_list(store_root, &dist_root(store_root).join("by_inspection"), inspection_id)
}

fn load_index_list(
    store_root: &Path,
    index_dir: &std::path::Path,
    key: &str,
) -> Result<Vec<AuditPacketDistribution>, String> {
    let safe_key: String = key.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    let idx_path = index_dir.join(format!("{}.json", safe_key));
    if !idx_path.exists() { return Ok(vec![]); }
    let ids: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&idx_path).unwrap_or_default()
    ).unwrap_or_default();
    let mut results = Vec::new();
    for id in ids {
        if let Ok(rec) = load_audit_packet_distribution(store_root, &AuditPacketDistributionId(id)) {
            results.push(rec);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_audit_packet_distribution::*;
    use openwand_workflow::workflow_audit_packet_review::AuditPacketReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn test_distribution(suffix: &str, key: &str) -> AuditPacketDistribution {
        let req = AuditPacketDistributionRequest {
            review_id: AuditPacketReviewId(format!("wapr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_review_hash: format!("rev_{}", suffix),
            audit_packet_hash: format!("pkt_{}", suffix),
            chain_hash: format!("chain_{}", suffix),
            inspection_id: format!("weci_{}", suffix),
            destination: AuditPacketDistributionDestination {
                destination_kind: AuditPacketDestinationKind::FileShare,
                label: "test".into(),
                reference: "test_ref".into(),
                operator_supplied_hash: None,
                notes: vec![],
            },
            distribution_notes: vec![],
            idempotency_key: key.into(),
        };
        build_audit_packet_distribution(req)
    }

    #[test]
    fn distribution_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let rec = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec).unwrap();
        let loaded = load_audit_packet_distribution(&dir, &rec.distribution_id).unwrap();
        assert_eq!(rec.distribution_id, loaded.distribution_id);
    }

    #[test]
    fn distribution_by_workflow_run_returns_expected() {
        let dir = test_dir();
        let rec = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec).unwrap();
        let results = distribution_by_workflow_run(&dir, "wfx_t").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn distribution_by_review_returns_expected() {
        let dir = test_dir();
        let rec = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec).unwrap();
        let results = distribution_by_review(&dir, "wapr_d1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn distribution_by_packet_hash_returns_expected() {
        let dir = test_dir();
        let rec = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec).unwrap();
        let results = distribution_by_audit_packet_hash(&dir, "pkt_d1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn distribution_by_inspection_returns_expected() {
        let dir = test_dir();
        let rec = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec).unwrap();
        let results = distribution_by_inspection(&dir, "weci_d1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn same_idempotency_key_returns_existing_distribution() {
        let dir = test_dir();
        let rec1 = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec1).unwrap();
        let rec2 = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec2).unwrap();
        let all = list_audit_packet_distributions(&dir).unwrap();
        assert_eq!(1, all.len());
    }

    #[test]
    fn different_key_preserves_distribution_history() {
        let dir = test_dir();
        let rec1 = test_distribution("d1", "key1");
        save_audit_packet_distribution(&dir, &rec1).unwrap();
        let rec2 = test_distribution("d1", "key2");
        save_audit_packet_distribution(&dir, &rec2).unwrap();
        let all = list_audit_packet_distributions(&dir).unwrap();
        assert_eq!(2, all.len());
    }
}
