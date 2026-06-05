//! CLI integration tests for workflow-operator-console.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}
fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

#[test]
fn cli_operator_console_show_outputs_console_state() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "show",
            "--workflow-execution-id", "wfx_test",
            "--output-dir", &out])
        .output().unwrap();
    assert!(result.status.success(), "show failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("wfx_test"));
}

#[test]
fn cli_operator_console_requires_workflow_execution_id() {
    let args: Vec<String> = vec![
        "workflow-operator-console".into(), "show".into(),
    ];
    let result = Command::new(openwand_bin()).args(&args).output().unwrap();
    assert!(!result.status.success(), "should fail without workflow-execution-id");
}

#[test]
fn cli_operator_console_show_json_outputs_json() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "show",
            "--workflow-execution-id", "wfx_json",
            "--output-dir", &out, "--json"])
        .output().unwrap();
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
}

#[test]
fn cli_operator_console_does_not_expose_route_resolve_reconcile_execute() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "show",
            "--workflow-execution-id", "wfx_safe",
            "--output-dir", &out, "--json"])
        .output().unwrap();
    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout).to_lowercase();
    assert!(!stdout.contains("\"creates_route\": true"));
    assert!(!stdout.contains("\"executes_tool\": true"));
    assert!(!stdout.contains("\"reconciles_outcome\": true"));
}

// Wave 48A: New subcommands

#[test]
fn cli_operator_console_summary_outputs_read_only_summary() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "summary",
            "--workflow-execution-id", "wfx_sum",
            "--output-dir", &out])
        .output().unwrap();
    assert!(result.status.success(), "summary failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("Summary"));
    assert!(stdout.contains("wfx_sum"));
}

#[test]
fn cli_operator_console_evidence_lists_grouped_links() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "evidence",
            "--workflow-execution-id", "wfx_ev",
            "--output-dir", &out])
        .output().unwrap();
    assert!(result.status.success(), "evidence failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    // Should show sections
    assert!(stdout.contains("UpstreamSpine") || stdout.contains("upstream_spine") || stdout.contains("present"));
}

#[test]
fn cli_operator_console_explain_outputs_detected_state_explanation() {
    let d = temp_dir();
    let out = d.path().to_string_lossy().to_string();
    let result = Command::new(openwand_bin())
        .args(["workflow-operator-console", "explain",
            "--workflow-execution-id", "wfx_exp",
            "--output-dir", &out])
        .output().unwrap();
    assert!(result.status.success(), "explain failed: {}", String::from_utf8_lossy(&result.stderr));
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("State:") || stdout.contains("inconclusive"));
}

// Patch 6: No action verbs in CLI surface
#[test]
fn cli_operator_console_does_not_expose_action_verbs() {
    let src = include_str!("../src/main.rs");
    // Find the console section
    let console_start = src.find("enum WorkflowOperatorConsoleCommands").unwrap_or(0);
    let console_end = src.find("fn cmd_operator_console").unwrap_or(src.len());
    let console_section = &src[console_start..console_end];
    let forbidden = ["execute", "resolve", "reconcile", "verify", "certify", "trust", "promote", "schedule", "advance"];
    for word in &forbidden {
        assert!(!console_section.to_lowercase().contains(word),
            "Console CLI section contains forbidden action verb: {}", word);
    }
}
