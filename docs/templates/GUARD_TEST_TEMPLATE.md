# Guard Test Template

Every workflow wave adds a guard test file at `crates/app/tests/workflow_[module]_guards.rs`. Use this template as the starting point.

---

## Standard Guard Test File

```rust
//! Guard tests for workflow [module name].

// --- Crate import guards (check workflow crate source) ---

#[test] fn [module]_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/[module].rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn [module]_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/[module].rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn [module]_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/[module].rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn [module]_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/[module].rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn [module]_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/[module].rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

// --- App behavioral guards (check app crate persistence module) ---

#[test] fn [module]_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/[module].rs");
    assert!(!src.contains("std::process::Command"));
    // Note: check for "git " only in fn bodies, not in type names like invokes_git
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff") || l.contains("git log")));
}

#[test] fn [module]_app_does_not_execute_commands() {
    let src = include_str!("../src/[module].rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn [module]_app_does_not_route_actions() {
    let src = include_str!("../src/[module].rs");
    assert!(!src.contains("route_action"));
    assert!(!src.contains("evaluate_action_route"));
}

#[test] fn [module]_app_does_not_resolve_approvals() {
    let src = include_str!("../src/[module].rs");
    assert!(!src.contains("resolve_approval"));
    assert!(!src.contains("ApprovalDecision"));
}

#[test] fn [module]_app_does_not_reconcile_outcomes() {
    let src = include_str!("../src/[module].rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("evaluate_reconciliation")));
}

#[test] fn [module]_app_does_not_append_trace_directly() {
    let src = include_str!("../src/[module].rs");
    assert!(!src.contains(".append("));
    assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn [module]_app_does_not_write_memory() {
    let src = include_str!("../src/[module].rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn [module]_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/[module].rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

// --- UI surface guard ---

#[test] fn [module]_ui_does_not_expose_execute_route_resolve_reconcile_retry_resume() {
    let src = include_str!("../src/ui/[module]_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("route")));
    assert!(!fn_lines.iter().any(|l| l.contains("resolve")));
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
    assert!(!fn_lines.iter().any(|l| l.contains("retry")));
    assert!(!fn_lines.iter().any(|l| l.contains("resume")));
}

// --- Dependency guard ---

#[test]
fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("workflow").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    let mut dep_count = 0u32;
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" { in_deps = true; continue; }
        if trimmed.starts_with('[') { in_deps = false; continue; }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let name = trimmed.split('=').next().unwrap().trim();
        assert!(allowed.contains(&name), "Unexpected dependency: {}", name);
        dep_count += 1;
    }
    assert_eq!(6, dep_count, "Workflow crate must have exactly 6 dependencies");
}

// --- Wave-specific guards (add per wave) ---
// Examples:
// - Serialized JSON shape guard (no forbidden field names)
// - Artifact metadata-only guard
// - No verification/read guard
```

---

## Guard Count Per Wave

| Category | Typical Count |
|----------|--------------:|
| Crate import guards | 5 |
| App behavioral guards | 7–9 |
| UI surface guard | 1 |
| Dependency guard | 1 |
| Wave-specific guards | 0–3 |
| **Total** | **15–18** |

---

## Gotchas

1. **`Command` in type names:** Don't check full source for `"Command"` — check only `use` lines or `pub fn` lines to avoid false positives from type names like `invokes_git: false` or `std::process::Command`.
2. **`git` in field names:** `invokes_git: false` contains "git" without a space. Check for `"git "` (with trailing space) in function bodies only, or check only `use`/`fn` lines.
3. **Include all related source files:** When a wave adds multiple workflow-crate files (e.g., DTOs + validation), include all of them in the crate import guards.
4. **`process::Command` vs `Command`**: The check should be for `process::Command` specifically, not bare `Command` which appears in many type names.
