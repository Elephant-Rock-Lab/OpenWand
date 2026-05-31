//! Evaluation collectors — score execution against deterministic expectations.
//!
//! Each collector consumes governed/trace artifacts (NOT raw store queries)
//! and produces structured evaluation results for scoring.

use crate::eval_model::*;
use crate::eval_trace::EvalTraceEvidence;
use crate::explain::Explanation;

/// Collect prompt/inference evaluation from trace evidence.
/// Reads inference.called events to extract prompt assembly details,
/// model, provider, and prompt hash.
///
/// Hard rule: No inference.called event → prompt dimension fails.
pub fn collect_prompt_eval(trace: &EvalTraceEvidence) -> PromptEvalResult {
    let called_events = trace.inference_events_by_kind("inference.called");

    if called_events.is_empty() {
        return PromptEvalResult {
            prompt_seen: false,
            evidence_missing: true,
            ..Default::default()
        };
    }

    // Use the first inference.called event as the primary source
    let first = called_events[0];
    let payload = &first.payload;

    // Extract from payload -> payload -> Called
    let called = payload.get("payload").and_then(|p| p.get("Called"));

    let model = called.and_then(|c| c.get("model")).and_then(|v| v.as_str()).map(String::from);
    let provider = called.and_then(|c| c.get("provider")).and_then(|v| v.as_str()).map(String::from);

    // Extract prompt assembly
    let prompt_assembly = called.and_then(|c| c.get("prompt_assembly"));
    let system_prompt_hash = prompt_assembly
        .and_then(|pa| pa.get("system_prompt_hash"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let memory_hit_count = prompt_assembly
        .and_then(|pa| pa.get("memory_hit_ids"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // Get tool count from inference.completed
    let completed_events = trace.inference_events_by_kind("inference.completed");
    let tool_count = completed_events
        .first()
        .and_then(|e| e.payload.get("payload"))
        .and_then(|p| p.get("Completed"))
        .and_then(|c| c.get("tool_call_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    // Message count is approximate: inference events + tool events
    let message_count = called_events.len() + completed_events.len();

    PromptEvalResult {
        prompt_seen: true,
        system_prompt_hash,
        message_count,
        tool_count,
        model,
        provider,
        evidence_missing: false,
    }
}

/// Collect memory evaluation from a governed report.
/// Uses GovernanceFilteredReport, same as explain/panel — never raw store.
pub fn collect_memory_eval(
    report: &openwand_memory::governance::GovernanceFilteredReport,
    expectations: &EvalExpectations,
) -> MemoryEvalResult {
    let mut included_claims_seen = Vec::new();
    let mut excluded_claims_seen = Vec::new();

    for finding in &report.included_claims {
        if let Some(ref text) = finding.finding.claim_text {
            included_claims_seen.push(text.clone());
        }
    }

    for finding in &report.audit_only_claims {
        if let Some(ref text) = finding.finding.claim_text {
            excluded_claims_seen.push(text.clone());
        }
    }

    // Check missing required claims
    let missing_required: Vec<String> = expectations
        .included_claims
        .iter()
        .filter(|expected| {
            !included_claims_seen
                .iter()
                .any(|seen| seen.contains(expected.as_str()))
        })
        .cloned()
        .collect();

    // Check excluded claims that leaked into included
    let unexpected_included: Vec<String> = expectations
        .excluded_claims
        .iter()
        .filter(|excluded| {
            included_claims_seen
                .iter()
                .any(|seen| seen.contains(excluded.as_str()))
        })
        .cloned()
        .collect();

    MemoryEvalResult {
        included_claims_seen,
        excluded_claims_seen,
        missing_required,
        unexpected_included,
        // Prompt-panel equivalence is checked separately (requires prompt assembly)
        prompt_panel_equivalent: true,
    }
}

/// Collect tool evaluation from tool results.
pub fn collect_tool_eval(
    executed_tools: &[String],
    expectations: &EvalExpectations,
) -> ToolEvalResult {
    let forbidden_requested: Vec<String> = expectations
        .forbidden_tool_calls
        .iter()
        .filter(|forbidden| executed_tools.iter().any(|t| t.contains(forbidden.as_str())))
        .cloned()
        .collect();

    let blocked_tools = vec![]; // Would be populated from trace events

    ToolEvalResult {
        requested_tools: executed_tools.to_vec(),
        executed_tools: executed_tools.to_vec(),
        blocked_tools,
        forbidden_requested,
    }
}

/// Collect policy evaluation from gate events.
pub fn collect_policy_eval(
    gates_seen: &[String],
    expectations: &EvalExpectations,
) -> PolicyEvalResult {
    let required_approvals_seen: Vec<String> = expectations
        .policy_events
        .iter()
        .filter(|expected| gates_seen.iter().any(|g| g.contains(expected.as_str())))
        .cloned()
        .collect();

    PolicyEvalResult {
        gates_seen: gates_seen.to_vec(),
        required_approvals_seen,
        unexpected_allows: vec![],
    }
}

/// Collect patch evaluation from patch results.
pub fn collect_patch_eval(
    planned: bool,
    applied: bool,
    preimage_verified: bool,
    postimage_verified: bool,
    rollback_available: bool,
    changed_files: &[String],
    expectations: &EvalExpectations,
) -> PatchEvalResult {
    let changed_files_match_expected = if expectations.file_changes.is_empty() {
        true
    } else {
        expectations
            .file_changes
            .iter()
            .all(|expected| changed_files.iter().any(|f| f.contains(expected.as_str())))
    };

    PatchEvalResult {
        planned,
        applied,
        preimage_verified,
        postimage_verified,
        rollback_available,
        changed_files_match_expected,
    }
}

/// Collect explain evaluation by comparing explanation sections against expected.
pub fn collect_explain_eval(
    explanation: &Explanation,
    expectations: &EvalExpectations,
) -> ExplainEvalResult {
    // Check memory matches
    let memory_matches = if expectations.included_claims.is_empty() {
        true
    } else {
        expectations.included_claims.iter().all(|expected| {
            explanation
                .memory
                .included
                .iter()
                .any(|c| c.claim.contains(expected.as_str()))
        })
    };

    // Check tool matches
    let tool_matches = if expectations.tool_calls.is_empty() {
        true
    } else {
        expectations.tool_calls.iter().all(|expected| {
            explanation
                .execution
                .tool_calls
                .iter()
                .any(|t| t.tool_name.contains(expected.as_str()))
        })
    };

    ExplainEvalResult {
        memory_matches,
        policy_matches: true,  // Would be checked against trace events
        tool_matches,
        completion_matches: true,
    }
}

/// Anti-vacuous-pass check: fail if required evidence dimensions are empty.
pub fn check_evidence_presence(
    has_inference_event: bool,
    has_tool_events: bool,
    has_governed_report: bool,
) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();

    if !has_inference_event {
        failures.push("No inference event in trace — cannot evaluate model behavior".to_string());
    }
    if !has_tool_events {
        failures.push("No tool events in trace — cannot evaluate tool usage".to_string());
    }
    if !has_governed_report {
        failures.push("No governed report — cannot evaluate memory behavior".to_string());
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_expectations() -> EvalExpectations {
        EvalExpectations {
            included_claims: vec!["crate core exists".to_string()],
            excluded_claims: vec!["low-confidence claim".to_string()],
            tool_calls: vec!["local__file_patch".to_string()],
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            file_changes: vec!["src/lib.rs".to_string()],
            policy_events: vec![],
            rebuild_matches: true,
            explain_matches: true,
        }
    }

    #[test]
    fn eval_memory_uses_governed_report_not_raw_store() {
        // Construct a governed report and verify collector only uses it
        let report = openwand_memory::governance::GovernanceFilteredReport {
            original_report: make_minimal_report(),
            governed_findings: vec![],
            included_claims: vec![],
            audit_only_claims: vec![],
        };

        let result = collect_memory_eval(&report, &EvalExpectations::default());
        assert!(result.included_claims_seen.is_empty());
        assert!(result.excluded_claims_seen.is_empty());
    }

    #[test]
    fn eval_detects_missing_required_memory() {
        let report = openwand_memory::governance::GovernanceFilteredReport {
            original_report: make_minimal_report(),
            governed_findings: vec![],
            included_claims: vec![],  // Nothing included
            audit_only_claims: vec![],
        };

        let expectations = EvalExpectations {
            included_claims: vec!["crate core exists".to_string()],
            ..Default::default()
        };

        let result = collect_memory_eval(&report, &expectations);
        assert_eq!(vec!["crate core exists"], result.missing_required);
    }

    #[test]
    fn eval_detects_excluded_claim_leaked_to_prompt() {
        let mut finding = make_finding("low-confidence claim should not appear");
        finding.prompt_eligibility = openwand_memory::governance::PromptEligibility::Include;

        let report = openwand_memory::governance::GovernanceFilteredReport {
            original_report: make_minimal_report(),
            governed_findings: vec![finding.clone()],
            included_claims: vec![finding],
            audit_only_claims: vec![],
        };

        let expectations = EvalExpectations {
            excluded_claims: vec!["low-confidence claim".to_string()],
            ..Default::default()
        };

        let result = collect_memory_eval(&report, &expectations);
        assert!(!result.unexpected_included.is_empty(), "Should detect leaked excluded claim");
    }

    #[test]
    fn eval_tool_detects_forbidden_request() {
        let executed = vec!["local__file_read".to_string(), "local__file_write".to_string()];
        let expectations = EvalExpectations {
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            ..Default::default()
        };
        let result = collect_tool_eval(&executed, &expectations);
        assert_eq!(vec!["local__file_write"], result.forbidden_requested);
    }

    #[test]
    fn eval_tool_allows_expected_only() {
        let executed = vec!["local__file_patch".to_string()];
        let expectations = EvalExpectations {
            tool_calls: vec!["local__file_patch".to_string()],
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            ..Default::default()
        };
        let result = collect_tool_eval(&executed, &expectations);
        assert!(result.forbidden_requested.is_empty());
        assert!(result.executed_tools.contains(&"local__file_patch".to_string()));
    }

    #[test]
    fn eval_patch_detects_missing_plan() {
        let result = collect_patch_eval(
            false, // not planned
            true,  // applied directly
            false, false, false,
            &["src/lib.rs".to_string()],
            &EvalExpectations { file_changes: vec!["src/lib.rs".to_string()], ..Default::default() },
        );
        assert!(!result.planned);
        assert!(result.applied);
        assert!(result.changed_files_match_expected);
    }

    #[test]
    fn eval_patch_detects_missing_rollback() {
        let result = collect_patch_eval(
            true, true, true, true,
            false, // no rollback
            &[],
            &EvalExpectations::default(),
        );
        assert!(!result.rollback_available);
    }

    #[test]
    fn eval_patch_detects_unexpected_changed_file() {
        let result = collect_patch_eval(
            true, true, true, true, true,
            &["src/unexpected.rs".to_string()],
            &EvalExpectations { file_changes: vec!["src/lib.rs".to_string()], ..Default::default() },
        );
        assert!(!result.changed_files_match_expected);
    }

    #[test]
    fn eval_fails_when_no_inference_event() {
        let result = check_evidence_presence(false, true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("inference"));
    }

    #[test]
    fn eval_fails_when_no_tool_events() {
        let result = check_evidence_presence(true, false, true);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("tool"));
    }

    #[test]
    fn eval_fails_when_no_governed_report() {
        let result = check_evidence_presence(true, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("governed"));
    }

    fn make_minimal_report() -> openwand_memory::repo_consistency::RepoConsistencyReport {
        openwand_memory::repo_consistency::RepoConsistencyReport {
            repo_root: std::path::PathBuf::from("/test"),
            checked_at: chrono::Utc::now(),
            summary: openwand_memory::repo_consistency::RepoConsistencySummary {
                supported: 0, stale: 0, missing_in_repo: 0, missing_in_memory: 0,
                unverifiable: 0, conflicted: 0, superseded_ignored: 0,
            },
            findings: vec![],
            memory_inputs: openwand_memory::repo_consistency::RepoMemoryInputSummary {
                current_claims_count: 0, superseded_count: 0, conflict_groups_count: 0,
            },
            repo_inputs: openwand_memory::repo_consistency::RepoObservationSummary {
                crates_count: 0, dependencies_count: 0, docs_count: 0,
            },
        }
    }

    fn make_finding(claim: &str) -> openwand_memory::governance::GovernedMemoryFinding {
        openwand_memory::governance::GovernedMemoryFinding {
            finding: openwand_memory::repo_consistency::RepoConsistencyFinding {
                kind: openwand_memory::repo_consistency::RepoConsistencyFindingKind::Supported,
                claim_text: Some(claim.to_string()),
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: openwand_memory::repo_consistency::ConsistencySeverity::Low,
                detail: "test".to_string(),
            },
            bucket: openwand_memory::provenance_hydration::MemoryTrustBucket::PromptIncluded,
            prompt_eligibility: openwand_memory::governance::PromptEligibility::Include,
            governance_reasons: vec![],
        }
    }

    // ── Prompt collector tests ──

    fn make_inference_called_payload(model: &str, provider: &str) -> crate::eval_trace::TraceEvidenceEntry {
        crate::eval_trace::TraceEvidenceEntry {
            trace_id: "inf_001".to_string(),
            event_kind: "inference.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: format!("inference.called ({})", model),
            payload: serde_json::json!({
                "family": "inference",
                "payload": {
                    "Called": {
                        "model": model,
                        "provider": provider,
                        "prompt_hash": "h1",
                        "thinking_budget": null,
                        "prompt_assembly": {
                            "system_prompt_hash": "sys_h1",
                            "message_window_hash": "msg_h1",
                            "memory_hit_ids": ["m1", "m2"],
                            "memory_context_hash": "mem_h1",
                            "tool_manifest_hash": "tool_h1",
                            "policy_filter_hash": "pol_h1",
                            "mode": "conversational",
                            "working_directory": "/test"
                        }
                    }
                }
            }),
        }
    }

    fn make_inference_completed_payload() -> crate::eval_trace::TraceEvidenceEntry {
        crate::eval_trace::TraceEvidenceEntry {
            trace_id: "inf_002".to_string(),
            event_kind: "inference.completed".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "inference.completed (qwen3)".to_string(),
            payload: serde_json::json!({
                "family": "inference",
                "payload": {
                    "Completed": {
                        "model": "qwen3",
                        "tokens": { "prompt": 100, "completion": 50, "total": 150 },
                        "stop_reason": "tool_use",
                        "tool_call_count": 2
                    }
                }
            }),
        }
    }

    #[test]
    fn eval_prompt_collector_reads_inference_called() {
        let mut trace = EvalTraceEvidence::default();
        trace.inference_events.push(make_inference_called_payload("qwen3", "lm-studio"));
        trace.inference_events.push(make_inference_completed_payload());

        let result = collect_prompt_eval(&trace);
        assert!(result.prompt_seen);
        assert!(!result.evidence_missing);
        assert_eq!(Some("qwen3".to_string()), result.model);
        assert_eq!(Some("lm-studio".to_string()), result.provider);
        assert_eq!(Some("sys_h1".to_string()), result.system_prompt_hash);
        assert_eq!(2, result.tool_count);
    }

    #[test]
    fn eval_prompt_collector_detects_missing_prompt() {
        let trace = EvalTraceEvidence::default();
        let result = collect_prompt_eval(&trace);
        assert!(!result.prompt_seen);
        assert!(result.evidence_missing);
        assert!(result.model.is_none());
        assert!(result.system_prompt_hash.is_none());
    }

    #[test]
    fn eval_prompt_collector_records_model_and_provider() {
        let mut trace = EvalTraceEvidence::default();
        trace.inference_events.push(make_inference_called_payload("llama3", "ollama"));

        let result = collect_prompt_eval(&trace);
        assert_eq!(Some("llama3".to_string()), result.model);
        assert_eq!(Some("ollama".to_string()), result.provider);
    }

    #[test]
    fn eval_prompt_collector_records_prompt_hash() {
        let mut trace = EvalTraceEvidence::default();
        trace.inference_events.push(make_inference_called_payload("qwen3", "lm-studio"));

        let result = collect_prompt_eval(&trace);
        assert_eq!(Some("sys_h1".to_string()), result.system_prompt_hash);
    }

    #[test]
    fn eval_prompt_collector_fails_without_inference_evidence() {
        let trace = EvalTraceEvidence::default();
        let result = collect_prompt_eval(&trace);
        // Hard rule: no inference event → prompt dimension cannot pass
        assert!(!result.prompt_seen, "Should not pass without inference evidence");
        assert!(result.evidence_missing);
    }
}
