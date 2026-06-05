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
    // Should be valid JSON
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
    // Patch 7: console CLI should not expose execution verbs
    assert!(!stdout.contains("\"creates_route\": true"));
    assert!(!stdout.contains("\"executes_tool\": true"));
    assert!(!stdout.contains("\"reconciles_outcome\": true"));
}
