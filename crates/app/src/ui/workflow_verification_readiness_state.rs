//! UI verification readiness state — display-only helpers.

use openwand_workflow::workflow_verification_readiness::*;

#[derive(Debug, Clone)]
pub struct VerificationReadinessSummaryRow {
    pub readiness_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub status: String,
    pub passed_count: usize,
    pub failed_count: usize,
}

pub fn readiness_summary(rec: &VerificationReadinessRecord) -> VerificationReadinessSummaryRow {
    let passed = rec.predicate_results.iter().filter(|r| r.passed).count();
    let failed = rec.predicate_results.iter().filter(|r| !r.passed).count();
    VerificationReadinessSummaryRow {
        readiness_id: rec.readiness_id.0.clone(),
        target_kind: format!("{:?}", rec.target_kind),
        target_id: rec.target_id.clone(),
        status: format!("{:?}", rec.status),
        passed_count: passed,
        failed_count: failed,
    }
}

pub fn verification_readiness_safety_warning() -> String {
    "Verification readiness is not verification. \
     It does not fetch, read, execute, verify signatures, inspect artifacts, \
     call shell/git, mutate workflow state, schedule verification, or certify truth.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_record() -> VerificationReadinessRecord {
        let request = VerificationReadinessRequest {
            target_kind: VerificationReadinessTargetKind::ManualResult,
            target_id: "wmr_t".into(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_target_hash: "h".into(),
            idempotency_key: "k".into(),
        };
        evaluate_readiness_metadata_only(&request, "reported_succeeded", "h", "wfx_t")
    }

    #[test]
    fn summary_row_extracts_fields() {
        let rec = test_record();
        let row = readiness_summary(&rec);
        assert!(row.readiness_id.starts_with("wvr_"));
        assert_eq!("ready", row.status.to_lowercase());
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = verification_readiness_safety_warning();
        assert!(w.contains("not verification"));
        assert!(w.contains("does not fetch"));
        assert!(w.contains("certify truth"));
    }
}
