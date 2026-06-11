//! Wave 69D: Truthful verification command tests.
//!
//! These tests prove the three placeholder commands (explain, trace-verify,
//! session-rebuild) no longer exit success with fake output. Each exits
//! non-zero with explicit not-implemented messaging.

use std::process::Command;

fn openwand_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    format!(
        "{}/target/debug/openwand{}",
        workspace_root.display(),
        std::env::consts::EXE_SUFFIX
    )
}

#[test]
fn cli_explain_exits_nonzero_with_not_implemented() {
    let output = Command::new(openwand_bin())
        .args(["explain", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "explain should exit non-zero, got exit {:?}",
        output.status.code()
    );
    assert!(
        stderr.contains("not yet implemented"),
        "stderr should say not yet implemented, got: {}",
        stderr
    );
    assert!(
        stderr.contains("explain"),
        "stderr should mention 'explain', got: {}",
        stderr
    );
}

#[test]
fn cli_trace_verify_exits_nonzero_with_not_implemented() {
    let output = Command::new(openwand_bin())
        .args(["trace-verify", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "trace-verify should exit non-zero, got exit {:?}",
        output.status.code()
    );
    assert!(
        stderr.contains("not yet implemented"),
        "stderr should say not yet implemented, got: {}",
        stderr
    );
    assert!(
        stderr.contains("trace-verify"),
        "stderr should mention 'trace-verify', got: {}",
        stderr
    );
}

#[test]
fn cli_session_rebuild_exits_nonzero_with_not_implemented() {
    let output = Command::new(openwand_bin())
        .args(["session-rebuild", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "session-rebuild should exit non-zero, got exit {:?}",
        output.status.code()
    );
    assert!(
        stderr.contains("not yet implemented"),
        "stderr should say not yet implemented, got: {}",
        stderr
    );
    assert!(
        stderr.contains("session-rebuild"),
        "stderr should mention 'session-rebuild', got: {}",
        stderr
    );
}

#[test]
fn cli_explain_does_not_claim_verification() {
    let output = Command::new(openwand_bin())
        .args(["explain", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    // Must not contain fake verification output
    assert!(
        !combined.contains("Trust Explanation"),
        "must not print fake trust explanation banner"
    );
    assert!(
        !combined.contains("Verified"),
        "must not claim verification"
    );
}

#[test]
fn cli_trace_verify_does_not_claim_verification() {
    let output = Command::new(openwand_bin())
        .args(["trace-verify", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !combined.contains("Trace Verification"),
        "must not print fake trace verification banner"
    );
    assert!(
        !combined.contains("verified"),
        "must not claim verification"
    );
}

#[test]
fn cli_session_rebuild_does_not_claim_success() {
    let output = Command::new(openwand_bin())
        .args(["session-rebuild", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !combined.contains("Session Rebuild"),
        "must not print fake session rebuild banner"
    );
    assert!(
        !combined.contains("rebuilt"),
        "must not claim rebuild success"
    );
}
