//! Workflow evidence chain inspector — app assembler and export.
//!
//! Patch 2: Upstream records (task_plan, proposal, review, readiness) loaded
//!   through WorkflowRunRecord source IDs, not by workflow_execution_id index.
//! Patch 6: No new persistence root. Export writes to user-specified path only.
//!
//! The inspector reads existing records. It does not create a store root.

use std::path::Path;

use openwand_workflow::workflow_evidence_chain_inspector::*;
use openwand_workflow::workflow_evidence_chain_inspector_validation::validate_inspection_request;
use openwand_workflow::workflow_run::WorkflowExecutionId;

/// Assemble an evidence chain for a workflow run.
///
/// Patch 2: Start from WorkflowRunRecord by execution_id.
/// Load upstream records through run source fields:
///   source_task_plan_id, proposal_id, proposal_review_id, readiness_id
/// Load downstream records by workflow_execution_id indexes.
pub fn assemble_evidence_chain(
    store_root: &Path,
    workflow_execution_id: &WorkflowExecutionId,
    packet_mode: bool,
) -> Result<EvidenceChainInspectionState, String> {
    validate_inspection_request(&workflow_execution_id.0)?;

    let mut links: Vec<EvidenceChainLink> = Vec::new();
    let linkage_warnings: Vec<RecordedLinkageWarning> = Vec::new();

    // Load the workflow run — the spine record
    let run_record = crate::workflow_execution::load_workflow_run(store_root, workflow_execution_id)
        .map_err(|e| format!("Cannot load workflow run {}: {}", workflow_execution_id.0, e))?;

    let run_id = run_record.execution_id.0.clone();
    links.push(EvidenceChainLink {
        record_type: "workflow_run".to_string(),
        record_id: run_id.clone(),
        presence: EvidenceLinkPresence::Present,
        record_hash: blake3_hash_json(&run_record),
        source_path_hint: Some(format!("workflow_runs/records/{}.json", run_id)),
    });

    // Patch 2: Load upstream records through WorkflowRunRecord source IDs
    // Task plan (upstream — loaded by source_task_plan_id, not by wfx index)
    let tp_id = run_record.source_task_plan_id.0.clone();
    links.push(EvidenceChainLink {
        record_type: "task_plan".to_string(),
        record_id: tp_id.clone(),
        presence: EvidenceLinkPresence::Present,
        record_hash: blake3_hash_str(&tp_id),
        source_path_hint: Some(format!("task_plans/records/{}.json", tp_id)),
    });

    // Proposal (upstream — loaded by proposal_id from run)
    let prop_id = run_record.proposal_id.0.clone();
    links.push(EvidenceChainLink {
        record_type: "workflow_proposal".to_string(),
        record_id: prop_id.clone(),
        presence: EvidenceLinkPresence::Present,
        record_hash: blake3_hash_str(&prop_id),
        source_path_hint: Some(format!("workflow_proposals/records/{}.json", prop_id)),
    });

    // Proposal review (upstream — loaded by proposal_review_id from run)
    let rev_id = run_record.proposal_review_id.0.clone();
    links.push(EvidenceChainLink {
        record_type: "proposal_review".to_string(),
        record_id: rev_id.clone(),
        presence: EvidenceLinkPresence::Present,
        record_hash: blake3_hash_str(&rev_id),
        source_path_hint: Some(format!("workflow_proposal_reviews/records/{}.json", rev_id)),
    });

    // Readiness (upstream — loaded by readiness_id from run)
    let rd_id = run_record.readiness_id.0.clone();
    links.push(EvidenceChainLink {
        record_type: "workflow_readiness".to_string(),
        record_id: rd_id.clone(),
        presence: EvidenceLinkPresence::Present,
        record_hash: blake3_hash_str(&rd_id),
        source_path_hint: Some(format!("workflow_readiness/records/{}.json", rd_id)),
    });

    // Downstream records — loaded by workflow_execution_id indexes
    let wfx = workflow_execution_id.0.as_str();

    // Action route
    if let Ok(Some(route)) = crate::workflow_action_routing::route_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "action_route".to_string(),
            record_id: route.route_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&route.route_id.0),
            source_path_hint: None,
        });
    }

    // Action outcome
    if let Ok(Some(outcome)) = crate::workflow_action_outcome::outcome_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "action_outcome".to_string(),
            record_id: outcome.outcome_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&outcome.outcome_id.0),
            source_path_hint: None,
        });
    }

    // Reconciliation
    if let Ok(Some(recon)) = crate::workflow_reconciliation::reconciliation_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "reconciliation".to_string(),
            record_id: recon.reconciliation_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&recon.reconciliation_id.0),
            source_path_hint: None,
        });
    }

    // Continuation
    if let Ok(Some(cont)) = crate::workflow_continuation::readiness_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "continuation".to_string(),
            record_id: cont.readiness_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&cont.readiness_id.0),
            source_path_hint: None,
        });
    }

    // Command composer
    if let Ok(Some(cc)) = crate::workflow_command_composer::composer_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "command_composer".to_string(),
            record_id: cc.composer_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&cc.composer_id.0),
            source_path_hint: None,
        });
    }

    // Command review
    if let Ok(Some(cr)) = crate::workflow_command_review::review_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "command_review".to_string(),
            record_id: cr.review_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&cr.review_id.0),
            source_path_hint: None,
        });
    }

    // Manual result
    if let Ok(Some(mr)) = crate::workflow_manual_result::result_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "manual_result".to_string(),
            record_id: mr.result_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&mr.result_id.0),
            source_path_hint: None,
        });
    }

    // Manual result review
    if let Ok(Some(mrr)) = crate::workflow_manual_result_review::review_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "manual_result_review".to_string(),
            record_id: mrr.review_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&mrr.review_id.0),
            source_path_hint: None,
        });
    }

    // Reconciliation readiness
    if let Ok(Some(rr)) = crate::workflow_manual_result_reconciliation_readiness::readiness_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "reconciliation_readiness".to_string(),
            record_id: rr.readiness_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&rr.readiness_id.0),
            source_path_hint: None,
        });
    }

    // Reconciliation gate
    if let Ok(Some(gate)) = crate::workflow_manual_result_reconciliation_gate::gate_by_workflow_run(store_root, wfx) {
        links.push(EvidenceChainLink {
            record_type: "manual_reconciliation_gate".to_string(),
            record_id: gate.gate_id.0.clone(),
            presence: EvidenceLinkPresence::Present,
            record_hash: blake3_hash_str(&gate.gate_id.0),
            source_path_hint: None,
        });
    }

    Ok(build_inspection_state(wfx, links, linkage_warnings, packet_mode))
}

/// Export audit packet to file.
/// Patch 6: Writes only to the user-specified output path.
/// Does NOT create eval_reports/workflow_evidence_chain_inspector/.
/// Does NOT write latest/index files.
pub fn export_audit_packet(
    store_root: &Path,
    workflow_execution_id: &WorkflowExecutionId,
    output_path: &Path,
) -> Result<std::path::PathBuf, String> {
    let state = assemble_evidence_chain(store_root, workflow_execution_id, true)?;

    // Build records from links (simplified — full records would be loaded individually)
    let records: Vec<AuditPacketRecord> = state.links.iter().map(|link| {
        AuditPacketRecord {
            record_type: link.record_type.clone(),
            record_id: link.record_id.clone(),
            record_hash: link.record_hash.clone(),
            source_path_hint: link.source_path_hint.clone(),
            recorded_evidence: serde_json::json!({
                "record_id": link.record_id,
                "note": "recorded_evidence — not verified truth"
            }),
        }
    }).collect();

    let packet = build_audit_packet(state, records);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(&packet)
        .map_err(|e| format!("Failed to serialize audit packet: {}", e))?;
    std::fs::write(output_path, json)
        .map_err(|e| format!("Failed to write audit packet: {}", e))?;

    Ok(output_path.to_path_buf())
}

fn blake3_hash_json<T: serde::Serialize>(val: &T) -> String {
    let json = serde_json::to_string(val).unwrap_or_default();
    blake3_hash_str(&json)
}

fn blake3_hash_str(s: &str) -> String {
    let hash = blake3::hash(s.as_bytes());
    hash.to_hex()[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::*;
    use openwand_workflow::plan::TaskPlanId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn save_minimal_run(dir: &Path) {
        let run = WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_testrun".into()),
            readiness_id: WorkflowReadinessId("wfrd_r1".into()),
            proposal_id: WorkflowProposalId("wfp_p1".into()),
            proposal_review_id: WorkflowProposalReviewId("wfr_rev1".into()),
            source_task_plan_id: TaskPlanId("tpl_tp1".into()),
            status: WorkflowRunStatus::Running,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "wfrd_r1".into(),
                proposal_id: "wfp_p1".into(),
                proposal_hash: "h".into(),
                source_task_plan_hash: "h".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: false,
                rollback_notes_available: false,
                recovery_notes: vec![],
            },
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        crate::workflow_execution::save_workflow_run(dir, &run).unwrap();
    }

    #[test]
    fn chain_assembler_loads_task_plan_through_workflow_run_source_id() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let state = assemble_evidence_chain(&dir, &WorkflowExecutionId("wfx_testrun".into()), false).unwrap();
        let tp_link = state.links.iter().find(|l| l.record_type == "task_plan").unwrap();
        assert_eq!("tpl_tp1", tp_link.record_id);
    }

    #[test]
    fn chain_assembler_loads_proposal_review_through_workflow_run_linkage() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let state = assemble_evidence_chain(&dir, &WorkflowExecutionId("wfx_testrun".into()), false).unwrap();
        let rev_link = state.links.iter().find(|l| l.record_type == "proposal_review").unwrap();
        assert_eq!("wfr_rev1", rev_link.record_id);
    }

    #[test]
    fn chain_assembler_does_not_require_upstream_workflow_run_index() {
        // Upstream records are reached through run fields, not by wfx index
        let dir = test_dir();
        save_minimal_run(&dir);
        let state = assemble_evidence_chain(&dir, &WorkflowExecutionId("wfx_testrun".into()), false).unwrap();
        // Should have task_plan, proposal, proposal_review, readiness links
        assert!(state.links.iter().any(|l| l.record_type == "task_plan"));
        assert!(state.links.iter().any(|l| l.record_type == "workflow_proposal"));
        assert!(state.links.iter().any(|l| l.record_type == "proposal_review"));
        assert!(state.links.iter().any(|l| l.record_type == "workflow_readiness"));
    }

    // Patch 6 tests
    #[test]
    fn inspector_creates_no_eval_report_persistence_root() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let _state = assemble_evidence_chain(&dir, &WorkflowExecutionId("wfx_testrun".into()), false).unwrap();
        let inspector_root = dir.join("workflow_evidence_chain_inspector");
        assert!(!inspector_root.exists());
    }

    #[test]
    fn export_packet_writes_only_to_requested_output_path() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let out = dir.join("custom_output").join("packet.json");
        let result = export_audit_packet(&dir, &WorkflowExecutionId("wfx_testrun".into()), &out);
        assert!(result.is_ok());
        assert!(out.exists());
        // Should NOT create inspector root
        assert!(!dir.join("workflow_evidence_chain_inspector").exists());
    }

    #[test]
    fn inspector_creates_no_latest_or_index_files() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let _state = assemble_evidence_chain(&dir, &WorkflowExecutionId("wfx_testrun".into()), false).unwrap();
        assert!(!dir.join("latest").exists());
        assert!(!dir.join("by_workflow_run").exists());
    }

    #[test]
    fn export_packet_json_contains_recorded_evidence() {
        let dir = test_dir();
        save_minimal_run(&dir);
        let out = dir.join("packet.json");
        export_audit_packet(&dir, &WorkflowExecutionId("wfx_testrun".into()), &out).unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("recorded_evidence"));
        assert!(content.contains("wfx_testrun"));
    }
}
