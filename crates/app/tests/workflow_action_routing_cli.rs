//! CLI tests for workflow action routing commands.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    format!("{}/target/debug/openwand{}", workspace_root.display(), std::env::consts::EXE_SUFFIX)
}

fn temp_dir() -> tempfile::TempDir { tempfile::tempdir().unwrap() }

fn create_full_chain_with_suspended_run(dir: &std::path::Path) -> (String, String, String, String, String, String) {
    // Plan
    let out = Command::new(openwand_bin()).args(["task-plan", "create", "--intent", "Action route CLI test",
        "--policy-constraints", "No shell", "--output-dir"])
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

    // Execute workflow (creates suspended run with action requests)
    let out = Command::new(openwand_bin()).args(["workflow-execution", "execute",
        "--readiness-id", &readiness_id, "--proposal-id", &proposal_id,
        "--proposal-review-id", &review_id, "--expected-readiness-hash", &proposal_hash,
        "--expected-proposal-hash", &proposal_hash, "--output-dir"]).arg(dir).arg("--json")
        .output().expect("execute");
    let exec: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let exec_id = exec["execution_id"].as_str().unwrap().to_string();

    // Find the first suspended stage with an action request
    let stages = exec["stages"].as_array().unwrap();
    let action_requests = exec["action_requests"].as_array().unwrap();

    let suspended_stage = stages.iter().find(|s| s["status"].as_str() == Some("suspended")).unwrap();
    let stage_id = suspended_stage["stage_id"].as_str().unwrap().to_string();

    let action_req = action_requests.iter().find(|a| a["stage_id"].as_str() == Some(&stage_id)).unwrap();
    let action_request_id = action_req["action_request_id"].as_str().unwrap().to_string();

    (exec_id, readiness_id, proposal_id, review_id, stage_id, action_request_id)
}

#[test]
fn cli_workflow_action_route_outputs_route_id() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--output-dir"]).arg(dir.path()).arg("--json").output().expect("route");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("war_"), "Expected route ID: {}", stdout);
}

#[test]
fn cli_workflow_action_show_roundtrips_record() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--output-dir"]).arg(dir.path()).arg("--json").output().expect("route");
    let route: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let route_id = route["route_id"].as_str().unwrap();

    // Strip the WorkflowActionRouteId tuple wrapper
    let route_id_clean = route_id.trim_start_matches('"').trim_end_matches('"');

    let show = Command::new(openwand_bin()).args(["workflow-action", "show", route_id_clean, "--output-dir"])
        .arg(dir.path()).arg("--json").output().expect("show");
    let shown: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&show.stdout)).unwrap();
    // The route_id is a tuple (String), so compare inner value
    assert_eq!(route_id, shown["route_id"].as_str().unwrap_or(""));
}

#[test]
fn cli_workflow_action_latest_by_workflow_run_returns_latest() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--output-dir"]).arg(dir.path()).arg("--json").output().expect("route");
    let out = Command::new(openwand_bin()).args(["workflow-action", "latest",
        "--workflow-execution-id", &exec_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("war_"));
}

#[test]
fn cli_workflow_action_latest_by_stage_returns_latest() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--output-dir"]).arg(dir.path()).arg("--json").output().expect("route");
    let out = Command::new(openwand_bin()).args(["workflow-action", "latest",
        "--stage-id", &stage_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("war_"));
}

#[test]
fn cli_workflow_action_latest_by_action_request_returns_latest() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--output-dir"]).arg(dir.path()).arg("--json").output().expect("route");
    let out = Command::new(openwand_bin()).args(["workflow-action", "latest",
        "--action-request-id", &action_request_id, "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("war_"));
}

#[test]
fn cli_workflow_action_latest_by_session_returns_latest() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id,
        "--expected-workflow-run-hash", "anyhash", "--expected-action-request-hash", "anyhash",
        "--session-id", "sess_cli_test", "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("route");
    let out = Command::new(openwand_bin()).args(["workflow-action", "latest",
        "--session-id", "sess_cli_test", "--output-dir"]).arg(dir.path()).arg("--json")
        .output().expect("latest");
    assert!(String::from_utf8_lossy(&out.stdout).contains("war_"));
}

#[test]
fn cli_workflow_action_requires_expected_hashes() {
    let dir = temp_dir();
    let (exec_id, readiness_id, proposal_id, _, stage_id, action_request_id) = create_full_chain_with_suspended_run(dir.path());
    let out = Command::new(openwand_bin()).args(["workflow-action", "route",
        "--workflow-execution-id", &exec_id, "--readiness-id", &readiness_id,
        "--proposal-id", &proposal_id, "--stage-id", &stage_id,
        "--action-request-id", &action_request_id, "--output-dir"]).arg(dir.path())
        .output().expect("route");
    assert!(!out.status.success(), "Should fail without required hashes");
}

#[test]
fn cli_workflow_action_does_not_expose_tool_shell_git_retry_resume() {
    let out = Command::new(openwand_bin()).args(["workflow-action", "--help"])
        .output().expect("help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
    assert!(!stdout.contains("execute-tool"));
    assert!(!stdout.contains("run-tool"));
    assert!(!stdout.contains("approve-tool"));
    assert!(!stdout.contains("reject-tool"));
    assert!(!stdout.contains("shell"));
    assert!(!stdout.contains("git"));
    assert!(!stdout.contains("retry"));
    assert!(!stdout.contains("resume"));
}
