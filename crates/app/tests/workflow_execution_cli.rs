//! CLI tests for workflow execution commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}

fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn create_full_chain(dir: &std::path::Path) -> (String, String, String, String, String, String) {
    // Plan
    let out = Command::new(openwand_bin()).args(["task-plan", "create", "--intent", "Execution CLI test", "--output-dir"])
        .arg(dir).arg("--json").output().expect("create plan");
    let plan: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap().to_string();

    // Approve plan
    Command::new(openwand_bin()).args(["task-plan", "review", "approve", "--plan-id", &plan_id,
        "--reviewer", "t", "--rationale", "OK", "--output-dir"]).arg(dir).arg("--json")
        .output().expect("approve plan");

    // Proposal
    let out = Command::new(openwand_bin()).args(["workflow-proposal", "create", "--task-plan-id", &plan_id,
        "--output-dir"]).arg(dir).arg("--json").output().expect("create proposal");
    let proposal: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let proposal_id = proposal["proposal_id"].as_str().unwrap().to_string();
    let proposal_hash = proposal["proposal_hash"].as_str().unwrap().to_string();
    let source_hash = proposal["source_task_plan_hash"].as_str().unwrap().to_string();

    // Approve proposal
    let out = Command::new(openwand_bin()).args(["workflow-proposal", "review", "approve",
        "--proposal-id", &proposal_id, "--reviewer", "t", "--rationale", "OK", "--output-dir"])
        .arg(dir).arg("--json").output().expect("approve proposal");
    let review: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let review_id = review["review_id"].as_str().unwrap().to_string();

    // Readiness
    let out = Command::new(openwand_bin()).args(["workflow-readiness", "evaluate",
        "--proposal-id", &proposal_id, "--review-id", &review_id,
        "--expected-proposal-hash", &proposal_hash, "--expected-source-task-plan-hash", &source_hash,
        "--output-dir"]).arg(dir).arg("--json").output().expect("evaluate readiness");
    let readiness: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let readiness_id = readiness["readiness_id"].as_str().unwrap().to_string();

    (plan_id, proposal_id, review_id, proposal_hash, source_hash, readiness_id)
}

#[test]
fn cli_workflow_execution_execute_outputs_run_id() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wfx_"), "Expected run ID: {}", stdout);
}

#[test]
fn cli_workflow_execution_show_roundtrips_record() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let exec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let exec_id = exec["execution_id"].as_str().unwrap();

    let show = Command::new(openwand_bin()).args(["workflow-execution", "show", exec_id, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    assert_eq!(exec_id, shown["execution_id"].as_str().unwrap());
}

#[test]
fn cli_workflow_execution_latest_by_readiness_returns_latest() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let out = Command::new(openwand_bin()).args(["workflow-execution", "latest",
        "--readiness-id", &readiness_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wfx_"));
}

#[test]
fn cli_workflow_execution_latest_by_proposal_returns_latest() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let out = Command::new(openwand_bin()).args(["workflow-execution", "latest",
        "--proposal-id", &proposal_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wfx_"));
}

#[test]
fn cli_workflow_execution_latest_by_review_returns_latest() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let out = Command::new(openwand_bin()).args(["workflow-execution", "latest",
        "--proposal-review-id", &review_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wfx_"));
}

#[test]
fn cli_workflow_execution_latest_by_task_plan_returns_latest() {
    let dir = temp_dir();
    let (plan_id, proposal_id, review_id, proposal_hash, _, readiness_id) = create_full_chain(dir.path());
    Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("execute");
    let out = Command::new(openwand_bin()).args(["workflow-execution", "latest",
        "--task-plan-id", &plan_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("wfx_"));
}

#[test]
fn cli_workflow_execution_requires_expected_hashes() {
    let dir = temp_dir();
    let (_, proposal_id, review_id, _, _, readiness_id) = create_full_chain(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--output-dir"]).arg(dir.path())
        .output().expect("execute");
    assert!(!out.status.success(), "Should fail without required hashes");
}

#[test]
fn cli_workflow_execution_does_not_expose_shell_git_worker_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-execution", "--help"])
        .output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("shell"));
    assert!(!stdout.contains("git"));
    assert!(!stdout.contains("worker"));
    assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume"));
    assert!(!stdout.contains("run-tool"));
}
