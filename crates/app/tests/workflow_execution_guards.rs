//! Guard tests for workflow execution.

#[test] fn workflow_execution_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_run.rs"), include_str!("../../workflow/src/workflow_execution_gate.rs"),
        include_str!("../../workflow/src/workflow_run_lifecycle.rs"), include_str!("../../workflow/src/workflow_run_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor") || l.contains("tool_executor"))); }
}

#[test] fn workflow_execution_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_run.rs"), include_str!("../../workflow/src/workflow_execution_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine") || l.contains("policy_engine"))); }
}

#[test] fn workflow_execution_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_run.rs"), include_str!("../../workflow/src/workflow_run_lifecycle.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore") || l.contains("memory_store"))); }
}

#[test] fn workflow_execution_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_run.rs"), include_str!("../../workflow/src/workflow_execution_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("trace_store") || l.contains("append_event"))); }
}

#[test] fn workflow_execution_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_run.rs"), include_str!("../../workflow/src/workflow_run_lifecycle.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn workflow_execution_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_execution.rs"); assert!(!src.contains("std::process::Command")); assert!(!src.contains("git ")); }

#[test] fn workflow_execution_app_does_not_execute_tools_directly() {
    let src = include_str!("../src/workflow_execution.rs"); let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }

#[test] fn workflow_execution_app_does_not_create_approval_requests_directly() {
    let src = include_str!("../src/workflow_execution.rs"); assert!(!src.contains("approval_request")); assert!(!src.contains("create_approval")); }

#[test] fn workflow_execution_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_execution.rs"); let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("append"))); }

#[test] fn workflow_execution_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_execution.rs"); let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore"))); }

#[test] fn workflow_execution_leaves_trace_count_unchanged() {
    // Patch 2: no "except session path" — Wave 26 does not route to sessions
    let src = include_str!("../src/workflow_execution.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }

#[test] fn workflow_execution_does_not_create_tool_result_directly() {
    let src = include_str!("../src/workflow_execution.rs"); assert!(!src.contains("tool_result")); assert!(!src.contains("ToolResult")); }

#[test] fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let cargo_toml = include_str!("../../workflow/Cargo.toml");
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    let mut found = Vec::new(); let mut in_deps = false;
    for line in cargo_toml.lines() {
        if line.trim() == "[dependencies]" { in_deps = true; continue; }
        if line.starts_with('[') && in_deps { break; }
        if in_deps && line.contains('=') { let dep = line.split('=').next().unwrap().trim(); found.push(dep.to_string()); }
    }
    assert!(found.len() <= 6, "workflow crate has {} deps", found.len());
    for dep in &found { assert!(allowed.contains(&dep.as_str()), "unexpected dep '{}'", dep); }
}
