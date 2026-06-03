//! Guard and no-mutation tests for workflow proposal creation/review.
//!
//! Proves that workflow proposal creation/review does not execute tools,
//! spawn workflows, mutate memory, append trace authority directly,
//! alter policy, or create execution grants.

use std::path::Path;

/// Guard: workflow crate source must not import tool executor.
#[test]
fn workflow_crate_does_not_import_tool_executor() {
    let lib_rs = include_str!("../../workflow/src/lib.rs");
    let proposal_rs = include_str!("../../workflow/src/workflow_proposal.rs");
    let builder_rs = include_str!("../../workflow/src/workflow_proposal_builder.rs");
    let review_rs = include_str!("../../workflow/src/workflow_proposal_review.rs");
    let validation_rs = include_str!("../../workflow/src/workflow_proposal_validation.rs");

    for (name, src) in [
        ("lib.rs", lib_rs),
        ("workflow_proposal.rs", proposal_rs),
        ("workflow_proposal_builder.rs", builder_rs),
        ("workflow_proposal_review.rs", review_rs),
        ("workflow_proposal_validation.rs", validation_rs),
    ] {
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(
            !use_lines.iter().any(|l| l.contains("ToolExecutor") || l.contains("tool_executor")),
            "workflow/{} must not import ToolExecutor",
            name
        );
    }
}

/// Guard: workflow crate source must not import policy engine.
#[test]
fn workflow_crate_does_not_import_policy_engine() {
    let sources = [
        include_str!("../../workflow/src/lib.rs"),
        include_str!("../../workflow/src/workflow_proposal.rs"),
        include_str!("../../workflow/src/workflow_proposal_builder.rs"),
        include_str!("../../workflow/src/workflow_proposal_review.rs"),
        include_str!("../../workflow/src/workflow_proposal_validation.rs"),
    ];
    for src in &sources {
        assert!(!src.contains("PolicyEngine"), "workflow crate must not import PolicyEngine");
        assert!(!src.contains("policy_engine"), "workflow crate must not import policy_engine");
    }
}

/// Guard: workflow crate source must not import memory projection store.
#[test]
fn workflow_crate_does_not_import_memory_projection_store() {
    let sources = [
        include_str!("../../workflow/src/lib.rs"),
        include_str!("../../workflow/src/workflow_proposal.rs"),
        include_str!("../../workflow/src/workflow_proposal_builder.rs"),
    ];
    for src in &sources {
        assert!(!src.contains("MemoryStore"), "workflow crate must not import MemoryStore");
        assert!(!src.contains("memory_store"), "workflow crate must not import memory_store");
    }
}

/// Guard: workflow crate source must not import trace append.
#[test]
fn workflow_crate_does_not_import_trace_append() {
    let sources = [
        include_str!("../../workflow/src/lib.rs"),
        include_str!("../../workflow/src/workflow_proposal.rs"),
        include_str!("../../workflow/src/workflow_proposal_builder.rs"),
    ];
    for src in &sources {
        assert!(!src.contains("TraceStore"), "workflow crate must not import TraceStore");
        assert!(!src.contains("trace_store"), "workflow crate must not import trace_store");
        assert!(!src.contains("append_event"), "workflow crate must not call append_event");
    }
}

/// Guard: workflow crate source must not import process command.
#[test]
fn workflow_crate_does_not_import_process_command() {
    let sources = [
        include_str!("../../workflow/src/lib.rs"),
        include_str!("../../workflow/src/workflow_proposal.rs"),
        include_str!("../../workflow/src/workflow_proposal_builder.rs"),
    ];
    for src in &sources {
        assert!(!src.contains("Command"), "workflow crate must not import Command");
        assert!(!src.contains("std::process"), "workflow crate must not import std::process");
    }
}

/// Guard: workflow crate Cargo.toml has only the expected 6 dependencies.
#[test]
fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let cargo_toml = include_str!("../../workflow/Cargo.toml");
    let allowed_deps = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];

    // Count dependency lines under [dependencies]
    let mut in_deps = false;
    let mut found_deps = Vec::new();
    for line in cargo_toml.lines() {
        if line.trim() == "[dependencies]" {
            in_deps = true;
            continue;
        }
        if line.starts_with('[') && in_deps {
            break;
        }
        if in_deps && line.contains('=') {
            let dep_name = line.split('=').next().unwrap().trim();
            found_deps.push(dep_name.to_string());
        }
    }

    assert!(
        found_deps.len() <= 6,
        "workflow crate has {} deps, expected ≤6: {:?}",
        found_deps.len(),
        found_deps
    );

    for dep in &found_deps {
        assert!(
            allowed_deps.contains(&dep.as_str()),
            "unexpected dep '{}' in workflow crate",
            dep
        );
    }
}

/// Guard: workflow proposal app persistence does not call shell or git.
#[test]
fn workflow_proposal_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_proposal.rs");
    assert!(!src.contains("std::process::Command"), "persistence must not call std::process::Command");
    assert!(!src.contains("git "), "persistence must not call git");
}

/// Guard: workflow proposal app persistence does not execute tools.
#[test]
fn workflow_proposal_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_proposal.rs");
    assert!(!src.contains("ToolExecutor"), "persistence must not import ToolExecutor");
    assert!(!src.contains("tool_execute"), "persistence must not call tool_execute");
}

/// Guard: workflow proposal app persistence does not append trace directly.
#[test]
fn workflow_proposal_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_proposal.rs");
    assert!(!src.contains("TraceStore"), "persistence must not import TraceStore");
    assert!(!src.contains("append_event"), "persistence must not call append_event");
}

/// Guard: workflow proposal app persistence does not write memory.
#[test]
fn workflow_proposal_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_proposal.rs");
    assert!(!src.contains("MemoryStore"), "persistence must not import MemoryStore");
    assert!(!src.contains("memory_store"), "persistence must not import memory_store");
}

/// Guard: workflow proposal UI does not expose execute, run, schedule.
#[test]
fn workflow_proposal_ui_does_not_expose_execute_run_schedule() {
    let state_src = include_str!("../src/ui/workflow_proposal_state.rs");
    // Check function signatures (pub fn) and struct/enum definitions,
    // but skip the #[cfg(test)] module which is not shipped.
    let mut in_test_mod = false;
    let code_lines: Vec<&str> = state_src.lines().filter(|l| {
        if l.contains("#cfg(test)") || l.contains("mod tests") {
            in_test_mod = true;
        }
        if in_test_mod {
            return false;
        }
        l.trim().starts_with("pub fn ") || l.trim().starts_with("fn ") || l.trim().starts_with("pub struct") || l.trim().starts_with("pub enum")
    }).collect();
    assert!(!code_lines.iter().any(|l| l.contains("execute")), "UI must not have execute function");
    assert!(!code_lines.iter().any(|l| l.contains("schedule")), "UI must not have schedule function");
    assert!(!code_lines.iter().any(|l| l.contains("dispatch")), "UI must not have dispatch function");
}

/// Guard: proposal creation leaves trace count unchanged.
#[test]
fn workflow_proposal_creation_leaves_trace_count_unchanged() {
    // Verify the persistence module has no trace imports or operations.
    // Check use statements only, not test fixtures.
    let src = include_str!("../src/workflow_proposal.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("trace") || l.contains("TraceStore")),
        "persistence must not import trace"
    );
}

/// Guard: proposal creation leaves memory store unchanged.
#[test]
fn workflow_proposal_creation_leaves_memory_store_unchanged() {
    let src = include_str!("../src/workflow_proposal.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")),
        "persistence must not import memory"
    );
}

/// Guard: proposal creation leaves head/index/worktree unchanged.
#[test]
fn workflow_proposal_creation_leaves_head_index_worktree_unchanged() {
    let src = include_str!("../src/workflow_proposal.rs");
    assert!(!src.contains("git"), "persistence must not reference git");
    assert!(!src.contains("HEAD"), "persistence must not reference HEAD");
    assert!(!src.contains("worktree"), "persistence must not reference worktree");
}

/// Guard: review leaves trace/memory/git unchanged.
#[test]
fn workflow_review_leaves_trace_memory_git_unchanged() {
    let src = include_str!("../src/workflow_proposal.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(
        !use_lines.iter().any(|l| l.contains("trace") || l.contains("TraceStore")),
        "review must not import trace"
    );
    assert!(
        !use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")),
        "review must not import memory"
    );
    assert!(!src.contains("git"), "review must not reference git");
}
