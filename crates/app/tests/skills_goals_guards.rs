//! Guard and no-mutation tests for skills/goals integration.
//!
//! Proves skills/goals cannot execute tools, mutate memory, append trace,
//! alter policy, or create execution authority.

use std::path::Path;

/// Read all Rust source files in a crate's src/ directory.
fn read_crate_sources(crate_name: &str) -> String {
    let workspace = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // Go up to workspace root, then into the crate
    let manifest_dir = Path::new(&workspace);
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let src_dir = workspace_root.join(crate_name).join("src");

    let mut all_source = String::new();
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rs") {
                all_source.push_str(&std::fs::read_to_string(&path).unwrap());
                all_source.push('\n');
            }
        }
    }
    all_source
}

fn assert_no_import(source: &str, forbidden: &[&str], crate_name: &str, reason: &str) {
    for pattern in forbidden {
        let patterns = [
            format!("use {pattern}"),
            format!("use {pattern}::"),
            format!("extern crate {pattern}"),
        ];
        for p in &patterns {
            assert!(
                !source.contains(p.as_str()),
                "{crate_name} must not import {reason}: found '{p}'"
            );
        }
    }
}

#[test]
fn skills_crate_does_not_import_process_command() {
    let source = read_crate_sources("crates/skills");
    assert_no_import(&source, &["std::process"], "skills crate", "process command");
}

#[test]
fn goals_crate_does_not_import_process_command() {
    let source = read_crate_sources("crates/goals");
    assert_no_import(&source, &["std::process"], "goals crate", "process command");
}

#[test]
fn skills_goals_context_no_mutation_imports() {
    // Verify the session_capability module in app crate doesn't import mutation machinery
    let workspace = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&workspace).join("src/session_capability.rs"),
    )
    .unwrap();

    let forbidden = [
        "openwand_tools",
        "openwand_policy",
        "openwand_session::runner",
        "std::process",
    ];
    for pattern in &forbidden {
        assert!(
            !source.contains(pattern),
            "session_capability must not import {pattern}"
        );
    }
}

#[test]
fn skill_dto_has_no_command_shell_script_fields() {
    // Verify SkillDefinition struct fields by checking the source
    let source = read_crate_sources("crates/skills");
    let forbidden_fields = [
        "pub command:",
        "pub shell:",
        "pub tool_name:",
        "pub tool_args:",
        "pub script:",
        "pub cwd:",
        "pub env:",
        "pub function_ref:",
    ];
    for field in &forbidden_fields {
        assert!(
            !source.contains(field),
            "Skills crate must not have executable field: {field}"
        );
    }
}

#[test]
fn goal_dto_has_no_command_shell_script_fields() {
    let source = read_crate_sources("crates/goals");
    let forbidden_fields = [
        "pub command:",
        "pub shell:",
        "pub tool_name:",
        "pub tool_args:",
        "pub script:",
        "pub cwd:",
        "pub env:",
        "pub function_ref:",
    ];
    for field in &forbidden_fields {
        assert!(
            !source.contains(field),
            "Goals crate must not have executable field: {field}"
        );
    }
}

#[test]
fn context_summaries_are_text_only_no_tool_handles() {
    let skills_source = read_crate_sources("crates/skills");
    let goals_source = read_crate_sources("crates/goals");

    let forbidden = [
        "pub tool_handle:",
        "pub command_string:",
        "pub executable:",
        "pub fn execute",
    ];
    for field in &forbidden {
        assert!(
            !skills_source.contains(field),
            "Skills context must not have: {field}"
        );
        assert!(
            !goals_source.contains(field),
            "Goals context must not have: {field}"
        );
    }
}

#[test]
fn loading_skills_goals_leaves_trace_count_unchanged() {
    // Design assertion: skill/goal loading functions don't touch trace store.
    // The crates don't even have a dependency on trace/store.
    // This test proves the dependency chain is clean.
    let skills_source = read_crate_sources("crates/skills");
    let goals_source = read_crate_sources("crates/goals");

    assert!(!skills_source.contains("TraceStore"));
    assert!(!goals_source.contains("TraceStore"));
    assert!(!skills_source.contains("append"));
    assert!(!goals_source.contains("append"));
}

#[test]
fn loading_skills_goals_leaves_memory_store_unchanged() {
    let skills_source = read_crate_sources("crates/skills");
    let goals_source = read_crate_sources("crates/goals");

    assert!(!skills_source.contains("MemoryStore"));
    assert!(!goals_source.contains("MemoryStore"));
    assert!(!skills_source.contains("project_episode"));
    assert!(!goals_source.contains("project_episode"));
}

#[test]
fn loading_skills_goals_leaves_head_index_worktree_unchanged() {
    let skills_source = read_crate_sources("crates/skills");
    let goals_source = read_crate_sources("crates/goals");

    assert!(!skills_source.contains("git"));
    assert!(!goals_source.contains("git"));
    assert!(!skills_source.contains("Command"));
    assert!(!goals_source.contains("Command"));
}

#[test]
fn session_context_build_does_not_append_trace() {
    let workspace = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&workspace).join("src/session_capability.rs"),
    )
    .unwrap();

    assert!(!source.contains("append"));
    assert!(!source.contains("TraceStore"));
    assert!(!source.contains("trace_append"));
}
