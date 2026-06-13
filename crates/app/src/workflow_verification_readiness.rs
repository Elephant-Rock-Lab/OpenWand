//! Verification readiness persistence and assembler.
//!
//! Patch 8: persistence with target indexes and idempotency rules.

use std::path::Path;

use openwand_workflow::workflow_verification_readiness::*;

fn readiness_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_verification_readiness")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    readiness_root(store_root).join("records")
}

fn by_workflow_run_dir(store_root: &Path) -> std::path::PathBuf {
    readiness_root(store_root).join("by_workflow_run")
}

fn by_target_dir(store_root: &Path, target_kind: &VerificationReadinessTargetKind) -> std::path::PathBuf {
    readiness_root(store_root).join("by_target").join(format!("{:?}", target_kind).to_lowercase())
}

fn by_target_id_dir(store_root: &Path) -> std::path::PathBuf {
    readiness_root(store_root).join("by_target_id")
}

/// Save a verification readiness record. Idempotent on idempotency_key.
pub fn save_verification_readiness(
    store_root: &Path,
    record: &VerificationReadinessRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create readiness dir: {}", e))?;

    // Idempotency: same key returns existing
    let existing = list_verification_readiness(store_root)?;
    for ex in &existing {
        if ex.idempotency_key == record.idempotency_key {
            let path = dir.join(format!("{}.json", ex.readiness_id.0));
            return Ok(path);
        }
    }

    // Patch 8: Ready cannot duplicate for same target hash with different key
    if matches!(record.status, VerificationReadinessStatus::Ready) {
        for ex in &existing {
            if ex.target_id == record.target_id
                && ex.expected_target_hash == record.expected_target_hash
                && matches!(ex.status, VerificationReadinessStatus::Ready)
            {
                let path = dir.join(format!("{}.json", ex.readiness_id.0));
                return Ok(path);
            }
        }
    }

    let path = dir.join(format!("{}.json", record.readiness_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize readiness: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write readiness: {}", e))?;

    // Write indexes
    write_index(&by_workflow_run_dir(store_root),
        &record.workflow_execution_id.0, &record.readiness_id.0)?;
    write_index(&by_target_dir(store_root, &record.target_kind),
        &record.target_id, &record.readiness_id.0)?;
    write_index(&by_target_id_dir(store_root),
        &record.target_id, &record.readiness_id.0)?;

    Ok(path)
}

fn write_index(dir: &std::path::PathBuf, key: &str, value: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create index dir: {}", e))?;
    let idx_path = dir.join(format!("{}.json", key));
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

/// Load a readiness record by ID.
pub fn load_verification_readiness(
    store_root: &Path,
    id: &WorkflowVerificationReadinessId,
) -> Result<VerificationReadinessRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read readiness {}: {}", id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse readiness {}: {}", id.0, e))
}

/// List all readiness records.
pub fn list_verification_readiness(
    store_root: &Path,
) -> Result<Vec<VerificationReadinessRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        if entry.path().extension().is_some_and(|ext| ext == "json")
            && let Ok(json) = std::fs::read_to_string(entry.path())
                && let Ok(rec) = serde_json::from_str::<VerificationReadinessRecord>(&json) {
                    results.push(rec);
                }
    }
    Ok(results)
}

/// Latest readiness record.
pub fn latest_verification_readiness(store_root: &Path) -> Result<Option<VerificationReadinessRecord>, String> {
    Ok(list_verification_readiness(store_root)?.into_iter().last())
}

/// Readiness records by workflow run.
pub fn readiness_by_workflow_run(
    store_root: &Path,
    workflow_execution_id: &str,
) -> Result<Vec<VerificationReadinessRecord>, String> {
    load_index_list(store_root, &by_workflow_run_dir(store_root), workflow_execution_id)
}

/// Readiness records by target.
pub fn readiness_by_target(
    store_root: &Path,
    target_kind: &VerificationReadinessTargetKind,
    target_id: &str,
) -> Result<Vec<VerificationReadinessRecord>, String> {
    load_index_list(store_root, &by_target_dir(store_root, target_kind), target_id)
}

/// Readiness records by target ID (any kind).
pub fn readiness_by_target_id(
    store_root: &Path,
    target_id: &str,
) -> Result<Vec<VerificationReadinessRecord>, String> {
    load_index_list(store_root, &by_target_id_dir(store_root), target_id)
}

fn load_index_list(
    store_root: &Path,
    index_dir: &std::path::PathBuf,
    key: &str,
) -> Result<Vec<VerificationReadinessRecord>, String> {
    let idx_path = index_dir.join(format!("{}.json", key));
    if !idx_path.exists() { return Ok(vec![]); }
    let ids: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&idx_path).unwrap_or_default()
    ).unwrap_or_default();
    let mut results = Vec::new();
    for id in ids {
        if let Ok(rec) = load_verification_readiness(store_root, &WorkflowVerificationReadinessId(id)) {
            results.push(rec);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn test_record(suffix: &str, key: &str, force_blocked: bool) -> VerificationReadinessRecord {
        let request = VerificationReadinessRequest {
            target_kind: VerificationReadinessTargetKind::ManualResult,
            target_id: format!("wmr_{}", suffix),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_target_hash: format!("hash_{}", suffix),
            idempotency_key: key.into(),
        };
        let predicates = if force_blocked {
            vec![
                openwand_workflow::workflow_verification_readiness::p(
                    VerificationReadinessPredicate::TargetRecordExists, false, "Not found"
                ),
            ]
        } else {
            vec![
                openwand_workflow::workflow_verification_readiness::p(
                    VerificationReadinessPredicate::TargetRecordExists, true, "Found"
                ),
                openwand_workflow::workflow_verification_readiness::p(
                    VerificationReadinessPredicate::TargetHashMatchesRequest, true, "Match"
                ),
            ]
        };
        openwand_workflow::workflow_verification_readiness::build_readiness_record(&request, predicates)
    }

    #[test]
    fn verification_readiness_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let rec = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec).unwrap();
        let loaded = load_verification_readiness(&dir, &rec.readiness_id).unwrap();
        assert_eq!(rec.readiness_id, loaded.readiness_id);
    }

    #[test]
    fn verification_readiness_by_workflow_run_returns_expected() {
        let dir = test_dir();
        let rec = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec).unwrap();
        let results = readiness_by_workflow_run(&dir, "wfx_t").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn verification_readiness_by_target_returns_expected() {
        let dir = test_dir();
        let rec = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec).unwrap();
        let results = readiness_by_target(&dir, &VerificationReadinessTargetKind::ManualResult, "wmr_r1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn verification_readiness_by_target_id_returns_expected() {
        let dir = test_dir();
        let rec = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec).unwrap();
        let results = readiness_by_target_id(&dir, "wmr_r1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn same_idempotency_key_returns_existing_verification_readiness() {
        let dir = test_dir();
        let rec1 = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec1).unwrap();
        let rec2 = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec2).unwrap();
        let all = list_verification_readiness(&dir).unwrap();
        assert_eq!(1, all.len());
    }

    #[test]
    fn ready_readiness_cannot_duplicate_for_same_target_hash() {
        let dir = test_dir();
        let rec1 = test_record("r1", "key1", false);
        save_verification_readiness(&dir, &rec1).unwrap();
        let rec2 = test_record("r1", "key2", false);
        save_verification_readiness(&dir, &rec2).unwrap();
        let all = list_verification_readiness(&dir).unwrap();
        assert_eq!(1, all.len()); // same target_id + target_hash
    }

    #[test]
    fn blocked_readiness_can_retry_with_new_key() {
        let dir = test_dir();
        let rec1 = test_record("r1", "key1", true);
        save_verification_readiness(&dir, &rec1).unwrap();
        let rec2 = test_record("r1", "key2", true);
        save_verification_readiness(&dir, &rec2).unwrap();
        let all = list_verification_readiness(&dir).unwrap();
        assert_eq!(2, all.len()); // blocked can retry
    }

    #[test]
    fn inconclusive_readiness_can_retry_with_new_key() {
        let dir = test_dir();
        let rec1 = test_record("r1", "key1", true);
        save_verification_readiness(&dir, &rec1).unwrap();
        let rec2 = test_record("r1", "key2", true);
        save_verification_readiness(&dir, &rec2).unwrap();
        let all = list_verification_readiness(&dir).unwrap();
        assert_eq!(2, all.len());
    }
}
