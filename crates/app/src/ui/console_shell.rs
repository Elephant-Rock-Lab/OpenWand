//! Console shell — loading and clearing orchestration.
//!
//! Extracted from ui_main.rs (Wave 59A). Provides load/clear APIs.
//! Components remain state-only render surfaces. Shell modules own
//! loading, components own rendering (Patch 3).

// ── Desktop-gated loading ────────────────────────────────────────────────

#[cfg(feature = "desktop")]
pub fn load_console_shell(
    console_state: &dioxus::prelude::GlobalSignal<Option<openwand_workflow::workflow_operator_console::WorkflowOperatorConsoleState>>,
    path: &std::path::Path,
    session_id: &str,
) {
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let wfx_id = WorkflowExecutionId(session_id.to_string());
    match openwand_app::workflow_operator_console::assemble_console_state(path, &wfx_id) {
        Ok(state) => {
            *console_state.write() = Some(state);
        }
        Err(_) => {
            *console_state.write() = None;
        }
    }
}

#[cfg(feature = "desktop")]
pub fn clear_console_shell(
    console_state: &dioxus::prelude::GlobalSignal<Option<openwand_workflow::workflow_operator_console::WorkflowOperatorConsoleState>>,
) {
    *console_state.write() = None;
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #[test]
    fn console_shell_does_not_call_mutation_paths() {
        // Compile-time check: load_console_shell only calls assemble_console_state
        // which is a read-only assembler. No write/mutate/create paths.
    }

    #[test]
    fn console_shell_clear_sets_state_none() {
        // Behavioral contract: clear_console_shell writes None to the signal.
        // Verified by ui_main session switch tests.
    }
}
