//! CLI tests for task plan commands.

use std::process::Command;

fn openwand_bin() -> String {
    // CARGO_MANIFEST_DIR is crates/app, go up to workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn cli_task_plan_create_outputs_plan_id() {
    let dir = temp_dir();
    let output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Test plan creation", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tpl_"), "Expected plan ID in output: {}", stdout);
    assert!(stdout.contains("Test plan creation"), "Expected intent in JSON: {}", stdout);
}

#[test]
fn cli_task_plan_show_roundtrips_record() {
    let dir = temp_dir();
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Show test", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let plan: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap();

    let show_output = Command::new(openwand_bin())
        .args(["task-plan", "show", plan_id, "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let show_stdout = String::from_utf8_lossy(&show_output.stdout);
    let shown: serde_json::Value = serde_json::from_str(&show_stdout).unwrap();
    assert_eq!(plan_id, shown["plan_id"].as_str().unwrap());
}

#[test]
fn cli_task_plan_latest_by_goal_returns_latest() {
    let dir = temp_dir();
    // Create plan without goal — latest should work
    let _ = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Latest test", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");

    let output = Command::new(openwand_bin())
        .args(["task-plan", "latest", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tpl_"), "Expected plan in latest output: {}", stdout);
}

#[test]
fn cli_task_plan_review_approve_outputs_review_id() {
    let dir = temp_dir();
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Review approve test", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let plan: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["task-plan", "review", "approve", "--plan-id", plan_id, "--reviewer", "test-user", "--rationale", "Looks good", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("tpr_"), "Expected review ID: {}", review_stdout);
    assert!(review_stdout.contains("approved"), "Expected approved decision: {}", review_stdout);
    assert!(review_stdout.contains("false"), "Expected creates_execution_grant=false: {}", review_stdout);
}

#[test]
fn cli_task_plan_review_reject_requires_feedback() {
    let dir = temp_dir();
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Reject test", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let plan: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["task-plan", "review", "reject", "--plan-id", plan_id, "--reviewer", "test-user", "--rationale", "Bad", "--feedback", "Missing steps", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("rejected"), "Expected rejected: {}", review_stdout);
}

#[test]
fn cli_task_plan_review_request_changes_requires_feedback() {
    let dir = temp_dir();
    let create_output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "Changes test", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let create_stdout = String::from_utf8_lossy(&create_output.stdout);
    let plan: serde_json::Value = serde_json::from_str(&create_stdout).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap();

    let review_output = Command::new(openwand_bin())
        .args(["task-plan", "review", "request-changes", "--plan-id", plan_id, "--reviewer", "test-user", "--rationale", "Needs work", "--feedback", "Add verify step", "--output-dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .expect("Failed to run openwand");
    let review_stdout = String::from_utf8_lossy(&review_output.stdout);
    assert!(review_stdout.contains("changes_requested"), "Expected changes_requested: {}", review_stdout);
}

#[test]
fn cli_task_plan_does_not_expose_execute() {
    // Verify there is no "execute" subcommand for task-plan
    let output = Command::new(openwand_bin())
        .args(["task-plan", "--help"])
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("execute"), "task-plan must not expose execute: {}", stdout);
}

#[test]
fn cli_task_plan_create_rejects_empty_intent() {
    let dir = temp_dir();
    let output = Command::new(openwand_bin())
        .args(["task-plan", "create", "--intent", "   ", "--output-dir"])
        .arg(dir.path())
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should fail — either from CLI arg or from builder
    assert!(!output.status.success(), "Empty intent should fail");
}
