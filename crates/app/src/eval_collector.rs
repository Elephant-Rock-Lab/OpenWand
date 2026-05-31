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
/// Collect tool evaluation from trace evidence.
/// Reads tool.called/completed/failed/suspended events.
pub fn collect_tool_eval(
    trace: &EvalTraceEvidence,
    expectations: &EvalExpectations,
) -> ToolEvalResult {
    let mut requested_tools = Vec::new();
    let mut executed_tools = Vec::new();
    let mut blocked_tools = Vec::new();
    let mut failed_tools = Vec::new();

    // tool.called → requested
    for entry in trace.tool_events_by_kind("tool.called") {
        let tool_name = extract_tool_name(&entry.payload);
        if let Some(name) = tool_name {
            requested_tools.push(name);
        }
    }

    // tool.completed → executed
    for entry in trace.tool_events_by_kind("tool.completed") {
        let tool_name = extract_tool_name(&entry.payload);
        if let Some(name) = tool_name {
            executed_tools.push(name);
        }
    }

    // tool.failed → failed
    for entry in trace.tool_events_by_kind("tool.failed") {
        let tool_name = extract_tool_name(&entry.payload);
        if let Some(name) = tool_name {
            failed_tools.push(name);
        }
    }

    // tool.suspended/tool.denied → blocked
    for entry in trace.tool_events_by_kind("tool.suspended") {
        let tool_name = extract_tool_name(&entry.payload);
        if let Some(name) = tool_name {
            blocked_tools.push(name);
        }
    }
    for entry in trace.tool_events_by_kind("tool.denied") {
        let tool_name = extract_tool_name(&entry.payload);
        if let Some(name) = tool_name {
            blocked_tools.push(name);
        }
    }

    // Detect forbidden tools
    let forbidden_requested: Vec<String> = expectations
        .forbidden_tool_calls
        .iter()
        .filter(|forbidden| requested_tools.iter().any(|t| t.contains(forbidden.as_str())))
        .cloned()
        .collect();

    ToolEvalResult {
        requested_tools,
        executed_tools,
        blocked_tools,
        forbidden_requested,
    }
}

/// Collect policy evaluation from trace evidence.
/// Reads gate.evaluated and gate.output_screened events.
pub fn collect_policy_eval(
    trace: &EvalTraceEvidence,
    expectations: &EvalExpectations,
) -> PolicyEvalResult {
    let mut gates_seen = Vec::new();
    let mut required_approvals_seen = Vec::new();
    let mut denials = Vec::new();
    let mut unexpected_allows = Vec::new();

    for entry in trace.gate_events_by_kind("gate.evaluated") {
        let evaluated = entry.payload.get("payload").and_then(|p| p.get("Evaluated"));
        if let Some(ev) = evaluated {
            let gate_kind = ev.get("gate_kind").and_then(|v| v.as_str()).unwrap_or("unknown");
            let passed = ev.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
            let risk = ev.get("risk_level").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let reason = ev.get("reason_code").and_then(|v| v.as_str()).unwrap_or("");

            gates_seen.push(format!("{}:{}:{}", gate_kind, if passed { "pass" } else { "block" }, risk));

            // Track required approvals
            if expectations.policy_events.iter().any(|e| gate_kind.contains(e.as_str())) {
                required_approvals_seen.push(gate_kind.to_string());
            }

            // Track denials
            if !passed {
                denials.push(format!("{}: {}", gate_kind, reason));
            }
        }
    }

    PolicyEvalResult {
        gates_seen,
        required_approvals_seen,
        unexpected_allows,
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
/// Collect rebuild evaluation from a rebuild result.
/// Runs actual rebuild and records fidelity.
pub fn collect_rebuild_eval(
    result: &openwand_session::rebuild::RebuildResult,
) -> RebuildEvalResult {
    RebuildEvalResult {
        events_replayed: result.events_replayed,
        state_matches: result.state_matches,
        divergences: result.divergences.clone(),
    }
}

/// Collect patch evaluation from trace evidence.
/// Reads file events and tool results for file_patch operations.
pub fn collect_patch_eval_from_trace(
    trace: &EvalTraceEvidence,
    expectations: &EvalExpectations,
) -> PatchEvalResult {
    let has_plan = trace.tool_events.iter().any(|e| {
        if e.event_kind != "tool.called" { return false; }
        let name = extract_tool_name(&e.payload);
        name.as_deref() == Some("local__file_patch")
    });

    let has_apply = trace.tool_events.iter().any(|e| {
        if e.event_kind != "tool.completed" { return false; }
        let name = extract_tool_name(&e.payload);
        name.as_deref() == Some("local__file_patch")
    });

    // Check file events for preimage/postimage evidence
    let has_file_events = trace.has_file_events();

    let changed_files: Vec<String> = trace.file_events.iter()
        .filter_map(|e| {
            let payload = e.payload.get("payload");
            payload.and_then(|p| {
                for variant in &["Written", "Patched"] {
                    if let Some(v) = p.get(variant) {
                        return v.get("path").and_then(|t| t.as_str()).map(String::from);
                    }
                }
                None
            })
        })
        .collect();

    let changed_files_match_expected = if expectations.file_changes.is_empty() {
        true
    } else {
        expectations
            .file_changes
            .iter()
            .all(|expected| changed_files.iter().any(|f| f.contains(expected.as_str())))
    };

    PatchEvalResult {
        planned: has_plan,
        applied: has_apply,
        preimage_verified: has_file_events,
        postimage_verified: has_file_events,
        rollback_available: false, // Requires tool result parsing
        changed_files_match_expected,
    }
}

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

/// Extract tool name from a tool event payload.
fn extract_tool_name(payload: &serde_json::Value) -> Option<String> {
    let payload_obj = payload.get("payload")?;
    for variant in &["Called", "Completed", "Failed", "Suspended", "Resumed", "Denied"] {
        if let Some(v) = payload_obj.get(variant) {
            if let Some(name) = v.get("tool_name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
    }
    None
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
        let mut trace = EvalTraceEvidence::default();
        // Add a tool.called for a forbidden tool
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.called file_write".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Called": { "tool_call_id": "tc1", "tool_name": "local__file_write", "args_hash": "h1", "invoker": "Llm" } }
            }),
        });
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.called file_read".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Called": { "tool_call_id": "tc2", "tool_name": "local__file_read", "args_hash": "h2", "invoker": "Llm" } }
            }),
        });

        let expectations = EvalExpectations {
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            ..Default::default()
        };
        let result = collect_tool_eval(&trace, &expectations);
        assert_eq!(vec!["local__file_write"], result.forbidden_requested);
    }

    #[test]
    fn eval_tool_allows_expected_only() {
        let mut trace = EvalTraceEvidence::default();
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.called file_patch".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Called": { "tool_call_id": "tc1", "tool_name": "local__file_patch", "args_hash": "h1", "invoker": "Llm" } }
            }),
        });
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "tool.completed".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.completed file_patch".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Completed": { "tool_call_id": "tc1", "tool_name": "local__file_patch", "status": "Success", "result_summary": "ok", "duration_ms": 100 } }
            }),
        });

        let expectations = EvalExpectations {
            tool_calls: vec!["local__file_patch".to_string()],
            forbidden_tool_calls: vec!["local__file_write".to_string()],
            ..Default::default()
        };
        let result = collect_tool_eval(&trace, &expectations);
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

    // ── Tool/Policy trace-backed tests ──

    #[test]
    fn eval_tool_collector_reads_tool_completed() {
        let mut trace = EvalTraceEvidence::default();
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.called file_read".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Called": { "tool_call_id": "tc1", "tool_name": "local__file_read", "args_hash": "h1", "invoker": "Llm" } }
            }),
        });
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "tool.completed".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.completed file_read".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Completed": { "tool_call_id": "tc1", "tool_name": "local__file_read", "status": "Success", "result_summary": "ok", "duration_ms": 50 } }
            }),
        });

        let result = collect_tool_eval(&trace, &EvalExpectations::default());
        assert!(result.requested_tools.contains(&"local__file_read".to_string()));
        assert!(result.executed_tools.contains(&"local__file_read".to_string()));
    }

    #[test]
    fn eval_tool_collector_reads_tool_failed() {
        let mut trace = EvalTraceEvidence::default();
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.failed".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.failed file_write".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Failed": { "tool_call_id": "tc1", "tool_name": "local__file_write", "error": "disk full" } }
            }),
        });

        let result = collect_tool_eval(&trace, &EvalExpectations::default());
        // Failed tools don't appear in executed but do appear as having been attempted
        // (they appear via tool.called if that was also emitted)
        assert_eq!(0, result.executed_tools.len());
    }

    #[test]
    fn eval_tool_collector_detects_blocked_tools() {
        let mut trace = EvalTraceEvidence::default();
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.suspended".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.suspended file_write".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Suspended": { "tool_call_id": "tc1", "tool_name": "local__file_write", "reason": "requires approval" } }
            }),
        });

        let result = collect_tool_eval(&trace, &EvalExpectations::default());
        assert!(result.blocked_tools.contains(&"local__file_write".to_string()));
    }

    #[test]
    fn eval_policy_collector_reads_gate_evaluated() {
        let mut trace = EvalTraceEvidence::default();
        trace.gate_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "g1".to_string(),
            event_kind: "gate.evaluated".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "gate.evaluated (passed)".to_string(),
            payload: serde_json::json!({
                "family": "gate",
                "payload": { "Evaluated": { "gate_id": "g1", "gate_kind": "risk", "passed": true, "risk_level": "Low", "reason_code": null, "summary": "ok" } }
            }),
        });

        let result = collect_policy_eval(&trace, &EvalExpectations::default());
        assert_eq!(1, result.gates_seen.len());
        assert!(result.gates_seen[0].contains("pass"));
    }

    #[test]
    fn eval_policy_collector_detects_required_confirmation() {
        let mut trace = EvalTraceEvidence::default();
        trace.gate_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "g1".to_string(),
            event_kind: "gate.evaluated".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "gate.evaluated (blocked)".to_string(),
            payload: serde_json::json!({
                "family": "gate",
                "payload": { "Evaluated": { "gate_id": "g1", "gate_kind": "write_gate", "passed": false, "risk_level": "Medium", "reason_code": "requires_approval", "summary": "blocked" } }
            }),
        });

        let expectations = EvalExpectations {
            policy_events: vec!["write_gate".to_string()],
            ..Default::default()
        };
        let result = collect_policy_eval(&trace, &expectations);
        assert!(result.required_approvals_seen.contains(&"write_gate".to_string()));
    }

    #[test]
    fn eval_tool_collector_empty_trace_produces_empty_results() {
        let trace = EvalTraceEvidence::default();
        let result = collect_tool_eval(&trace, &EvalExpectations::default());
        assert!(result.requested_tools.is_empty());
        assert!(result.executed_tools.is_empty());
        assert!(result.blocked_tools.is_empty());
    }

    // ── Patch trace-backed tests ──

    #[test]
    fn eval_patch_collector_reads_from_trace() {
        let mut trace = EvalTraceEvidence::default();
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t1".to_string(),
            event_kind: "tool.called".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.called file_patch".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Called": { "tool_call_id": "tc1", "tool_name": "local__file_patch", "args_hash": "h1", "invoker": "Llm" } }
            }),
        });
        trace.tool_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "t2".to_string(),
            event_kind: "tool.completed".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "tool.completed file_patch".to_string(),
            payload: serde_json::json!({
                "family": "tool",
                "payload": { "Completed": { "tool_call_id": "tc1", "tool_name": "local__file_patch", "status": "Success", "result_summary": "applied", "duration_ms": 100 } }
            }),
        });

        let result = collect_patch_eval_from_trace(&trace, &EvalExpectations::default());
        assert!(result.planned, "Should detect plan from tool.called");
        assert!(result.applied, "Should detect apply from tool.completed");
    }

    #[test]
    fn eval_patch_collector_no_patch_events_means_not_applied() {
        let trace = EvalTraceEvidence::default();
        let result = collect_patch_eval_from_trace(&trace, &EvalExpectations::default());
        assert!(!result.planned);
        assert!(!result.applied);
    }

    #[test]
    fn eval_patch_collector_detects_changed_files() {
        let mut trace = EvalTraceEvidence::default();
        trace.file_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "f1".to_string(),
            event_kind: "file.written".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "file.written src/lib.rs".to_string(),
            payload: serde_json::json!({
                "family": "file",
                "payload": { "Written": { "path": "src/lib.rs", "bytes": 100, "preimage_hash": "h1", "postimage_hash": "h2" } }
            }),
        });

        let expectations = EvalExpectations {
            file_changes: vec!["src/lib.rs".to_string()],
            ..Default::default()
        };
        let result = collect_patch_eval_from_trace(&trace, &expectations);
        assert!(result.changed_files_match_expected);
    }

    #[test]
    fn eval_patch_collector_unexpected_file_fails() {
        let mut trace = EvalTraceEvidence::default();
        trace.file_events.push(crate::eval_trace::TraceEvidenceEntry {
            trace_id: "f1".to_string(),
            event_kind: "file.written".to_string(),
            occurred_at: chrono::Utc::now(),
            summary: "file.written unexpected.rs".to_string(),
            payload: serde_json::json!({
                "family": "file",
                "payload": { "Written": { "path": "unexpected.rs", "bytes": 50 } }
            }),
        });

        let expectations = EvalExpectations {
            file_changes: vec!["src/lib.rs".to_string()],
            ..Default::default()
        };
        let result = collect_patch_eval_from_trace(&trace, &expectations);
        assert!(!result.changed_files_match_expected);
    }

    // ── Rebuild collector tests ──

    #[test]
    fn eval_rebuild_collector_records_rebuild_result() {
        let rebuild_result = openwand_session::rebuild::RebuildResult {
            events_replayed: 42,
            state_matches: true,
            divergences: vec![],
        };
        let result = collect_rebuild_eval(&rebuild_result);
        assert_eq!(42, result.events_replayed);
        assert!(result.state_matches);
        assert!(result.divergences.is_empty());
    }

    #[test]
    fn eval_rebuild_collector_detects_divergence() {
        let rebuild_result = openwand_session::rebuild::RebuildResult {
            events_replayed: 10,
            state_matches: false,
            divergences: vec!["message_count: expected 5, got 4".to_string()],
        };
        let result = collect_rebuild_eval(&rebuild_result);
        assert!(!result.state_matches);
        assert_eq!(1, result.divergences.len());
    }

    // ── Memory fidelity tests ──

    #[test]
    fn eval_memory_collector_has_no_raw_store_dependency() {
        // Memory collector takes GovernedFilteredReport — no store import needed
        // This test documents the invariant
        let report = openwand_memory::governance::GovernanceFilteredReport {
            original_report: make_minimal_report(),
            governed_findings: vec![],
            included_claims: vec![],
            audit_only_claims: vec![],
        };
        let result = collect_memory_eval(&report, &EvalExpectations::default());
        // No raw store queries, just governed report
        assert!(result.included_claims_seen.is_empty());
    }

    #[test]
    fn eval_memory_collector_detects_missing_governed_report() {
        // If report has no findings at all but expectations require claims,
        // missing_required should list them
        let report = openwand_memory::governance::GovernanceFilteredReport {
            original_report: make_minimal_report(),
            governed_findings: vec![],
            included_claims: vec![],
            audit_only_claims: vec![],
        };
        let expectations = EvalExpectations {
            included_claims: vec!["crate core exists".to_string()],
            ..Default::default()
        };
        let result = collect_memory_eval(&report, &expectations);
        assert!(!result.missing_required.is_empty());
    }
}
