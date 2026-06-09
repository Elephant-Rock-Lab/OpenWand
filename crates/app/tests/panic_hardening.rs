//! Wave 51A panic hardening tests — FIX-06.
//!
//! Proves: try_new returns client on standard platforms,
//! production callers use try_new (grep guard),
//! and no unchecked expect remains in production paths.

use openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient;

#[test]
fn try_new_returns_client_on_standard_platform() {
    let result = OpenAiCompatibleClient::try_new();
    assert!(result.is_ok(), "try_new should succeed on standard platforms with TLS available");
}

#[test]
fn production_callers_use_try_new_grep_guard() {
    let src = include_str!("../src/session_runtime.rs");
    // Both build_session_runtime functions must use try_new, not new()
    assert!(
        src.contains("OpenAiCompatibleClient::try_new()"),
        "session_runtime.rs must use try_new() for production LLM client construction"
    );
    // Must NOT contain OpenAiCompatibleClient::new() (test-only constructor)
    assert!(
        !src.contains("OpenAiCompatibleClient::new()"),
        "session_runtime.rs must NOT use new() — use try_new() in production paths"
    );
}

#[test]
fn ui_main_uses_try_new_grep_guard() {
    let src = include_str!("../src/ui_main.rs");
    assert!(
        src.contains("OpenAiCompatibleClient::try_new()"),
        "ui_main.rs must use try_new() for production LLM client construction"
    );
}

#[test]
fn no_unwrap_on_production_mutex_in_run_bridge() {
    let src = include_str!("../src/ui/run_bridge.rs");
    // run_bridge.rs must use unwrap_or_else for mutex, not raw unwrap
    assert!(
        !src.contains(".lock().unwrap()"),
        "run_bridge.rs must not contain .lock().unwrap() — use unwrap_or_else"
    );
    assert!(
        src.contains("unwrap_or_else"),
        "run_bridge.rs must use unwrap_or_else for mutex recovery"
    );
}

#[test]
fn no_unwrap_on_production_mutex_in_service() {
    let src = include_str!("../src/ui/service.rs");
    assert!(
        !src.contains(".lock().unwrap()"),
        "service.rs must not contain .lock().unwrap() — use unwrap_or_else"
    );
}

#[test]
fn no_expect_on_trace_store_in_handle_send() {
    let src = include_str!("../src/ui_main.rs");
    // The handle_send function should not have .expect() on store opens
    // (the init_service hook still has them — those are startup-time, acceptable)
    let handle_send_start = src.find("async fn handle_send").unwrap_or(0);
    let handle_send_section = &src[handle_send_start..];
    assert!(
        !handle_send_section.contains(".expect("),
        "handle_send must not contain .expect() on store operations — use error propagation"
    );
}
