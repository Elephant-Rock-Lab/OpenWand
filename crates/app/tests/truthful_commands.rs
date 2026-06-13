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
fn cli_trace_verify_reports_result_or_fails_on_missing_db() {
    // trace-verify is now REAL (Wave 92B). It should either:
    // - exit 0 (Pass) if trace DB exists and verification passes
    // - exit 1 (operational error) if trace DB doesn't exist
    // - exit 2 (Fail) if trace integrity fails
    // It must NOT claim to be unimplemented.
    let output = Command::new(openwand_bin())
        .args(["trace-verify", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Must NOT say "not yet implemented"
    assert!(
        !combined.contains("not yet implemented"),
        "trace-verify is real now, must not say unimplemented: {}",
        combined
    );

    // Must NOT claim to be a stub
    assert!(
        !combined.contains("planned for a future release"),
        "trace-verify is real now"
    );

    // Exit code should be one of the documented codes
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1 || code == 2 || code == 3 || code == 4,
        "trace-verify should exit with documented code (0-4), got {}",
        code
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
fn cli_trace_verify_does_not_claim_full_immutability() {
    // trace-verify is now REAL. It should print an honest note about
    // what Pass means (chain continuity, not hash recomputation).
    let output = Command::new(openwand_bin())
        .args(["trace-verify", "test-session"])
        .output()
        .expect("Failed to run openwand");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // On Pass, should include the honest note about limitations
    let code = output.status.code().unwrap_or(-1);
    if code == 0 {
        assert!(
            stdout.contains("chain continuity") || stdout.contains("ordering"),
            "on Pass, should mention what was verified: {}",
            stdout
        );
        assert!(
            !stdout.contains("full immutability") && !stdout.contains("cryptographic immutability"),
            "must not claim full cryptographic immutability"
        );
    }

    // Must not claim backend-specific hash correctness
    assert!(
        !combined.contains("hash correctness verified"),
        "must not claim hash correctness verification"
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
