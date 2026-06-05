//! Workflow operator console assembler.
//!
//! Patch 3: No persistence. Recomputes console state from existing evidence indexes.
//! The console observes, summarizes, and links evidence. It does not create records
//! or perform recommended operations.

use std::path::Path;

use openwand_workflow::workflow_operator_console::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

/// Assemble console state from existing evidence indexes.
/// Patch 3: Returns computed state, writes nothing.
pub fn assemble_console_state(
    store_root: &Path,
    workflow_execution_id: &WorkflowExecutionId,
) -> Result<WorkflowOperatorConsoleState, String> {
    let wfx = workflow_execution_id.0.as_str();

    // Gather latest records from each evidence area
    let stages = vec![]; // Would load from workflow run
    let run_status = "unknown".to_string();
    let detected = openwand_workflow::workflow_loop_state::WorkflowDetectedLoopState::Inconclusive;

    // Build evidence chain from indexes
    let mut evidence_chain = Vec::new();
    let mut chain_warnings = Vec::new();

    // Check manual-result ladder indexes
    let cc_id = load_gate_index(store_root, "by_command_composer", wfx);
    let cr_id = load_gate_index(store_root, "by_command_review", wfx);
    let mr_id = load_gate_index(store_root, "by_manual_result", wfx);
    let mrr_id = load_gate_index(store_root, "by_manual_result_review", wfx);
    let rr_id = load_gate_index(store_root, "by_reconciliation_readiness", wfx);
    let gate_id = load_gate_index(store_root, "by_workflow_run", wfx);

    if let Some(ref id) = cc_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "command_composer".into(), record_id: id.clone(), status: "found".into(), summary: "Command composed".into() });
    }
    if let Some(ref id) = cr_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "command_review".into(), record_id: id.clone(), status: "found".into(), summary: "Command reviewed".into() });
    }
    if let Some(ref id) = mr_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_result".into(), record_id: id.clone(), status: "found".into(), summary: "Manual result captured".into() });
    }
    if let Some(ref id) = mrr_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_result_review".into(), record_id: id.clone(), status: "found".into(), summary: "Manual result reviewed".into() });
    }
    if let Some(ref id) = rr_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "reconciliation_readiness".into(), record_id: id.clone(), status: "found".into(), summary: "Reconciliation readiness evaluated".into() });
    }
    if let Some(ref id) = gate_id {
        evidence_chain.push(ConsoleEvidenceLink { link_kind: "manual_reconciliation_gate".into(), record_id: id.clone(), status: "found".into(), summary: "Manual reconciliation gate evaluated".into() });
    }

    // Patch 4: validate chain consistency
    chain_warnings = validate_manual_result_chain(
        cc_id.as_deref(), cr_id.as_deref(), mr_id.as_deref(),
        mrr_id.as_deref(), rr_id.as_deref(), gate_id.as_deref(),
    );

    Ok(build_console_state(
        workflow_execution_id.clone(),
        run_status,
        stages,
        &detected,
        None,
        evidence_chain,
        chain_warnings,
    ))
}

fn load_gate_index(store_root: &Path, index_name: &str, key: &str) -> Option<String> {
    // Try manual reconciliation gate index first
    let gate_idx = store_root.join("workflow_manual_result_reconciliation_gates")
        .join(index_name).join(format!("{}.json", key));
    if let Ok(id) = std::fs::read_to_string(&gate_idx) {
        return Some(id.trim().to_string());
    }
    // Fallback to other indexes
    let readiness_idx = store_root.join("workflow_manual_result_reconciliation_readiness")
        .join(index_name).join(format!("{}.json", key));
    if let Ok(id) = std::fs::read_to_string(&readiness_idx) {
        return Some(id.trim().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_dir() -> PathBuf { tempfile::tempdir().unwrap().into_path() }

    #[test]
    fn assemble_console_state_returns_state_without_writing() {
        let d = test_dir();
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert_eq!(WorkflowExecutionId("wfx_t".into()), state.workflow_execution_id);
        // No files written
        assert!(!d.join("workflow_operator_console").exists());
    }

    #[test]
    fn assemble_console_creates_no_console_record() {
        let d = test_dir();
        assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        // No console-specific directory
        let entries: Vec<_> = std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).collect();
        assert!(entries.is_empty(), "Console should not write any files");
    }

    #[test]
    fn assemble_console_writes_no_eval_report_files() {
        let d = test_dir();
        assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(!d.join("eval_reports").exists());
    }

    #[test]
    fn assemble_console_finds_manual_reconciliation_gate() {
        let d = test_dir();
        // Create a gate index
        let idx_dir = d.join("workflow_manual_result_reconciliation_gates").join("by_workflow_run");
        std::fs::create_dir_all(&idx_dir).unwrap();
        std::fs::write(idx_dir.join("wfx_t.json"), "wmrrg_test").unwrap();

        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(state.evidence_chain.iter().any(|e| e.link_kind == "manual_reconciliation_gate"));
    }

    #[test]
    fn assemble_console_chain_consistent_with_full_ladder() {
        let d = test_dir();
        let base = d.join("workflow_manual_result_reconciliation_gates");
        for (idx, id) in [
            ("by_command_composer", "wcc_1"), ("by_command_review", "wcrv_1"),
            ("by_manual_result", "wmr_1"), ("by_manual_result_review", "wmrr_1"),
            ("by_reconciliation_readiness", "wmrrr_1"), ("by_workflow_run", "wmrrg_1"),
        ] {
            let dir = base.join(idx);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("wfx_t.json"), id).unwrap();
        }
        let state = assemble_console_state(&d, &WorkflowExecutionId("wfx_t".into())).unwrap();
        assert!(state.evidence_chain_consistent, "Chain should be consistent with full ladder");
        assert_eq!(6, state.evidence_chain.len());
    }
}
