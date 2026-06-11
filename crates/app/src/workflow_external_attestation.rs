//! External attestation persistence.
//!
//! Patch 5: persistence indexes for target, chain, kind, and source lookup.
//! Idempotency: same key returns existing attestation.
//! Attestations preserve history; they do not supersede prior attestations.

use std::path::Path;

use openwand_workflow::workflow_external_attestation::*;

fn attestations_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_external_attestations")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    attestations_root(store_root).join("records")
}

fn by_workflow_run_dir(store_root: &Path) -> std::path::PathBuf {
    attestations_root(store_root).join("by_workflow_run")
}

fn by_target_dir(store_root: &Path, target_kind: &ExternalAttestationTargetKind) -> std::path::PathBuf {
    attestations_root(store_root).join("by_target").join(format!("{:?}", target_kind).to_lowercase())
}

fn by_target_id_dir(store_root: &Path) -> std::path::PathBuf {
    attestations_root(store_root).join("by_target_id")
}

fn by_kind_dir(store_root: &Path, kind: &ExternalAttestationKind) -> std::path::PathBuf {
    attestations_root(store_root).join("by_kind").join(format!("{:?}", kind).to_lowercase())
}

fn by_source_dir(store_root: &Path, source_name: &str) -> std::path::PathBuf {
    let hash = blake3::hash(source_name.as_bytes());
    attestations_root(store_root).join("by_source").join(&hash.to_hex()[..16])
}

/// Save an external attestation. Idempotent on idempotency_key.
pub fn save_external_attestation(
    store_root: &Path,
    attestation: &WorkflowExternalAttestation,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create attestations dir: {}", e))?;

    // Idempotency: check existing
    let existing = list_external_attestations(store_root)?;
    for ex in &existing {
        if ex.idempotency_key == attestation.idempotency_key {
            let path = dir.join(format!("{}.json", ex.attestation_id.0));
            return Ok(path);
        }
    }

    let path = dir.join(format!("{}.json", attestation.attestation_id.0));
    let json = serde_json::to_string_pretty(attestation)
        .map_err(|e| format!("Failed to serialize attestation: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write attestation: {}", e))?;

    // Write indexes
    write_index(&by_workflow_run_dir(store_root),
        &attestation.target.workflow_execution_id.0, &attestation.attestation_id.0)?;
    write_index(&by_target_dir(store_root, &attestation.target.target_kind),
        &attestation.target.target_id, &attestation.attestation_id.0)?;
    write_index(&by_target_id_dir(store_root),
        &attestation.target.target_id, &attestation.attestation_id.0)?;
    let kind_str = format!("{:?}", attestation.kind).to_lowercase();
    write_index(&by_kind_dir(store_root, &attestation.kind),
        &kind_str, &attestation.attestation_id.0)?;
    let source_hash = blake3::hash(attestation.source.name.as_bytes());
    let source_key = &source_hash.to_hex()[..16];
    write_index(&by_source_dir(store_root, &attestation.source.name),
        source_key, &attestation.attestation_id.0)?;

    Ok(path)
}

fn write_index(dir: &std::path::PathBuf, key: &str, value: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create index dir: {}", e))?;
    // Append to index file (multiple attestations per target is valid)
    let idx_path = dir.join(format!("{}.json", key));
    let mut existing: Vec<String> = if idx_path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&idx_path).unwrap_or_default()).unwrap_or_default()
    } else {
        vec![]
    };
    if !existing.contains(&value.to_string()) {
        existing.push(value.to_string());
    }
    let json = serde_json::to_string(&existing)
        .map_err(|e| format!("Failed to serialize index: {}", e))?;
    std::fs::write(&idx_path, json)
        .map_err(|e| format!("Failed to write index: {}", e))?;
    Ok(())
}

/// Load an attestation by ID.
pub fn load_external_attestation(
    store_root: &Path,
    id: &WorkflowExternalAttestationId,
) -> Result<WorkflowExternalAttestation, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read attestation {}: {}", id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse attestation {}: {}", id.0, e))
}

/// List all attestations.
pub fn list_external_attestations(
    store_root: &Path,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Ok(json) = std::fs::read_to_string(&path)
                && let Ok(att) = serde_json::from_str::<WorkflowExternalAttestation>(&json) {
                    results.push(att);
                }
    }
    Ok(results)
}

/// Latest attestation.
pub fn latest_external_attestation(store_root: &Path) -> Result<Option<WorkflowExternalAttestation>, String> {
    let all = list_external_attestations(store_root)?;
    Ok(all.into_iter().last())
}

/// Attestations by workflow run.
pub fn attestations_by_workflow_run(
    store_root: &Path,
    workflow_execution_id: &str,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    load_index_list(store_root, &by_workflow_run_dir(store_root), workflow_execution_id)
}

/// Attestations by target.
pub fn attestations_by_target(
    store_root: &Path,
    target_kind: &ExternalAttestationTargetKind,
    target_id: &str,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    load_index_list(store_root, &by_target_dir(store_root, target_kind), target_id)
}

/// Attestations by target ID (any target kind).
pub fn attestations_by_target_id(
    store_root: &Path,
    target_id: &str,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    load_index_list(store_root, &by_target_id_dir(store_root), target_id)
}

/// Attestations by kind.
pub fn attestations_by_kind(
    store_root: &Path,
    kind: &ExternalAttestationKind,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    let kind_str = format!("{:?}", kind).to_lowercase();
    load_index_list(store_root, &by_kind_dir(store_root, kind), &kind_str)
}

/// Attestations by source name.
pub fn attestations_by_source(
    store_root: &Path,
    source_name: &str,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    let hash = blake3::hash(source_name.as_bytes());
    let key = &hash.to_hex()[..16];
    load_index_list(store_root, &by_source_dir(store_root, source_name), key)
}

fn load_index_list(
    store_root: &Path,
    index_dir: &std::path::PathBuf,
    key: &str,
) -> Result<Vec<WorkflowExternalAttestation>, String> {
    let idx_path = index_dir.join(format!("{}.json", key));
    if !idx_path.exists() { return Ok(vec![]); }
    let ids: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(&idx_path).unwrap_or_default()
    ).unwrap_or_default();
    let mut results = Vec::new();
    for id in ids {
        if let Ok(att) = load_external_attestation(store_root, &WorkflowExternalAttestationId(id)) {
            results.push(att);
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_external_attestation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn test_attestation(id_suffix: &str, key: &str) -> WorkflowExternalAttestation {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: format!("wmr_{}", id_suffix),
            expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "Alice".into(),
            source_role: "reviewer".into(),
            source_system_identifier: None,
            claim: format!("Claim {}", id_suffix),
            references: vec![],
            reported_signature: None,
            attested_at: chrono::Utc::now(),
            idempotency_key: key.into(),
        };
        build_external_attestation(req)
    }

    #[test]
    fn external_attestation_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        let loaded = load_external_attestation(&dir, &att.attestation_id).unwrap();
        assert_eq!(att.attestation_id, loaded.attestation_id);
        assert_eq!(att.claim, loaded.claim);
    }

    #[test]
    fn external_attestation_by_workflow_run_returns_expected() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        let results = attestations_by_workflow_run(&dir, "wfx_t").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn external_attestation_by_target_returns_expected() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        let results = attestations_by_target(&dir, &ExternalAttestationTargetKind::ManualResult, "wmr_r1").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn external_attestation_by_kind_returns_expected() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        let results = attestations_by_kind(&dir, &ExternalAttestationKind::ThirdPartySignoff).unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn external_attestation_by_source_returns_expected() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        let results = attestations_by_source(&dir, "Alice").unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn same_idempotency_key_returns_existing_attestation() {
        let dir = test_dir();
        let att1 = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att1).unwrap();
        let att2 = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att2).unwrap();
        let all = list_external_attestations(&dir).unwrap();
        assert_eq!(1, all.len());
    }

    #[test]
    fn different_key_preserves_attestation_history() {
        let dir = test_dir();
        let att1 = test_attestation("r1", "key1");
        let att2 = test_attestation("r1", "key2");
        save_external_attestation(&dir, &att1).unwrap();
        save_external_attestation(&dir, &att2).unwrap();
        let all = list_external_attestations(&dir).unwrap();
        assert_eq!(2, all.len());
    }

    #[test]
    fn attestation_writes_only_attestation_evidence() {
        let dir = test_dir();
        let att = test_attestation("r1", "key1");
        save_external_attestation(&dir, &att).unwrap();
        // Should not write to other persistence roots
        assert!(!dir.join("workflow_runs").exists());
        assert!(!dir.join("workflow_manual_results").exists());
    }
}
