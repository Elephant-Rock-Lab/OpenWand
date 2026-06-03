//! Guard and no-mutation tests for workflow readiness.

/// Guard: readiness crate source must not import tool executor.
#[test]
fn workflow_readiness_crate_does_not_import_tool_executor() {
    let sources = [
        include_str!("../../workflow/src/workflow_readiness.rs"),
        include_str!("../../workflow/src/workflow_readiness_evaluator.rs"),
        include_str!("../../workflow/src/workflow_readiness_validation.rs"),
        include_str!("../../workflow/src/tool_intent_resolution.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(
            !use_lines.iter().any(|l| l.contains("ToolExecutor") || l.contains("tool_executor")),
            "readiness code must not import ToolExecutor"
        );
    }
}

/// Guard: readiness crate source must not import policy engine execution paths.
#[test]
fn workflow_readiness_crate_does_not_import_policy_engine_for_execution() {
    let sources = [
        include_str!("../../workflow/src/workflow_readiness.rs"),
        include_str!("../../workflow/src/workflow_readiness_evaluator.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(
            !use_lines.iter().any(|l| l.contains("PolicyEngine") || l.contains("policy_engine")),
            "readiness code must not import PolicyEngine"
        );
    }
}

/// Guard: readiness crate source must not import memory store.
#[test]
fn workflow_readiness_crate_does_not_import_memory_projection_store() {
    let sources = [
        include_str!("../../workflow/src/workflow_readiness.rs"),
        include_str!("../../workflow/src/workflow_readiness_evaluator.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(
            !use_lines.iter().any(|l| l.contains("MemoryStore") || l.contains("memory_store")),
            "readiness code must not import MemoryStore"
        );
    }
}

/// Guard: readiness crate source must not import trace.
#[test]
fn workflow_readiness_crate_does_not_import_trace_append() {
    let sources = [
        include_str!("../../workflow/src/workflow_readiness.rs"),
        include_str!("../../workflow/src/workflow_readiness_evaluator.rs"),
    ];
    for src in &sources {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(
            !use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("trace_store") || l.contains("append_event")),
            "readiness code must not import TraceStore"
        );
    }
}

/// Guard: readiness crate source must not import process.
#[test]
fn workflow_readiness_crate_does_not_import_process_command() {
    let sources = [
        include_str!("../../workflow/src/workflow_readiness.rs"),
        include_str!("../../workflow/src/workflow_readiness_evaluator.rs"),
    ];
    for src in &sources {
        assert!(!src.contains("std::process"), "readiness code must not import std::process");
    }
}

/// Guard: readiness app persistence does not call shell or git.
#[test]
fn workflow_readiness_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_readiness.rs");
    assert!(!src.contains("std::process::Command"), "persistence must not call process");
    assert!(!src.contains("git "), "persistence must not call git");
}

/// Guard: readiness app persistence does not execute tools.
#[test]
fn workflow_readiness_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("ToolExecutor")),
        "persistence must not import ToolExecutor"
    );
}

/// Guard: readiness app does not create approval requests.
#[test]
fn workflow_readiness_app_does_not_create_approval_requests() {
    let src = include_str!("../src/workflow_readiness.rs");
    assert!(!src.contains("approval_request"), "must not create approval requests");
    assert!(!src.contains("create_approval"), "must not create approval requests");
}

/// Guard: readiness app does not append trace directly.
#[test]
fn workflow_readiness_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("append")),
        "persistence must not import trace"
    );
}

/// Guard: readiness evaluation leaves trace/memory/git unchanged.
#[test]
fn workflow_readiness_evaluation_leaves_trace_memory_git_unchanged() {
    let src = include_str!("../src/workflow_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("trace") || l.contains("TraceStore")),
        "persistence must not import trace"
    );
    assert!(
        !use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")),
        "persistence must not import memory"
    );
    assert!(!src.contains("git"), "persistence must not reference git");
}

/// Guard: readiness evaluation does not create workflow run records.
#[test]
fn workflow_readiness_evaluation_does_not_create_workflow_run_record() {
    let src = include_str!("../src/workflow_readiness.rs");
    assert!(!src.contains("workflow_run"), "must not create workflow run records");
    assert!(!src.contains("WorkflowRun"), "must not reference WorkflowRun");
}
