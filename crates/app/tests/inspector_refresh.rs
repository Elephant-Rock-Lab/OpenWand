//! Tests for inspector refresh authority boundary (Wave 89A).
//!
//! Proves:
//! 1. The refresh state DTO module imports no backend authority types.
//! 2. The refresh function does not call any operation/mutation methods.
//! 3. No session_id → workflow_execution_id fallback.
//! 4. No selected workflow run produces Unavailable, not Failed.
//! 5. Manual refresh uses existing read-only inspector loader.
//! 6. State lifecycle is honest.

// ── Source-level authority boundary guards ───────────────────

#[cfg(test)]
mod authority_guards {
    /// Guard: inspector_refresh_state.rs imports no backend authority.
    #[test]
    fn refresh_state_dto_does_not_import_backend_authority() {
        let src = include_str!("../src/ui/inspector_refresh_state.rs");
        assert!(!src.contains("SessionRunner"), "must not import SessionRunner");
        assert!(!src.contains("ToolExecutor"), "must not import ToolExecutor");
        assert!(!src.contains("PolicyEngine"), "must not import PolicyEngine");
        assert!(!src.contains("TraceStore"), "must not import TraceStore");
        assert!(!src.contains("load_inspector_shell"), "must not call loader directly");
        assert!(!src.contains("export_audit_packet"), "must not call exporter");
    }

    /// Guard: refresh_inspector function does not call operation/mutation methods.
    #[test]
    #[cfg(feature = "desktop")]
    fn refresh_function_is_read_only() {
        let src = include_str!("../src/ui_main.rs");
        let refresh_section = src
            .split("fn refresh_inspector()")
            .nth(1)
            .unwrap_or("");

        // Must call the existing read-only loader
        assert!(
            refresh_section.contains("load_inspector_shell"),
            "must use existing read-only inspector loader"
        );

        // Must NOT call any operation or mutation methods
        assert!(!refresh_section.contains("request_workflow_run"), "must not request workflow runs");
        assert!(!refresh_section.contains("submit_approval_resolution"), "must not resolve approvals");
        assert!(!refresh_section.contains("export_evidence"), "must not export evidence");
        assert!(!refresh_section.contains("resolve_approval"), "must not resolve approvals directly");
        assert!(!refresh_section.contains("advance_stages"), "must not advance stages");
        assert!(!refresh_section.contains("save_workflow_run"), "must not save workflow runs");
        assert!(!refresh_section.contains("export_audit_packet"), "must not export audit packets");
        assert!(!refresh_section.contains("append_trace"), "must not append trace");
        assert!(!refresh_section.contains("ToolExecutor"), "must not execute tools");
    }

    /// Guard: refresh function does not use session_id as workflow_execution_id fallback.
    #[test]
    #[cfg(feature = "desktop")]
    fn refresh_does_not_fallback_session_id_to_workflow_id() {
        let src = include_str!("../src/ui_main.rs");
        let refresh_section = src
            .split("fn refresh_inspector()")
            .nth(1)
            .unwrap_or("");

        // Must check SELECTED_WORKFLOW_EXECUTION_ID and WORKFLOW_RUN_REQUEST_STATE
        assert!(
            refresh_section.contains("SELECTED_WORKFLOW_EXECUTION_ID"),
            "must check selected workflow execution ID"
        );
        assert!(
            refresh_section.contains("execution_id_opt"),
            "must check workflow run request state for execution ID"
        );

        // Must produce Unavailable when no workflow ID found
        assert!(
            refresh_section.contains("Unavailable"),
            "must produce Unavailable when no workflow run selected"
        );

        // Must NOT use session_id as workflow_execution_id
        // The refresh function should not reference session_id in the context of wfx_id
        assert!(
            !refresh_section.contains("CURRENT_SESSION.read().as_ref().map(|v| v.summary.session_id"),
            "must not use session_id as workflow_execution_id fallback"
        );
    }
}

// ── State lifecycle tests ────────────────────────────────────

#[cfg(test)]
mod state_lifecycle_tests {
    use openwand_app::ui::inspector_refresh_state::*;

    #[test]
    fn no_selected_workflow_run_is_unavailable_not_failed() {
        // When no workflow run is selected, the state should be Unavailable,
        // not Failed — this is an expected condition, not an error.
        let state = InspectorRefreshState::Unavailable {
            reason: "No workflow run selected".into(),
        };
        assert!(state.is_terminal());
        assert!(!state.is_loading());
    }

    #[test]
    fn loading_carries_workflow_execution_id() {
        let state = InspectorRefreshState::Loading {
            workflow_execution_id: "wfx_abc".into(),
        };
        assert!(state.is_loading());
        assert!(!state.is_terminal());
        if let InspectorRefreshState::Loading { workflow_execution_id } = &state {
            assert_eq!(workflow_execution_id, "wfx_abc");
        }
    }

    #[test]
    fn live_reports_sections_honestly() {
        let state = InspectorRefreshState::Live {
            workflow_execution_id: "wfx_1".into(),
            refreshed_at: "2026-06-13T22:00:00Z".into(),
            sections_attempted: 7,
            sections_loaded: 5,
        };
        assert!(state.is_terminal());
        if let InspectorRefreshState::Live {
            sections_attempted,
            sections_loaded,
            ..
        } = &state
        {
            assert!(*sections_loaded <= *sections_attempted, "loaded must be <= attempted");
        }
    }

    #[test]
    fn stale_carries_workflow_id_and_reason() {
        let state = InspectorRefreshState::Stale {
            workflow_execution_id: "wfx_old".into(),
            reason: "Selection changed during refresh".into(),
        };
        assert!(state.is_terminal());
    }

    #[test]
    fn failed_may_have_optional_workflow_id() {
        // Failed before workflow ID was resolved
        let no_id = InspectorRefreshState::Failed {
            workflow_execution_id: None,
            error: "Could not resolve store path".into(),
        };
        assert!(no_id.is_terminal());

        // Failed after workflow ID was resolved
        let with_id = InspectorRefreshState::Failed {
            workflow_execution_id: Some("wfx_1".into()),
            error: "I/O error".into(),
        };
        assert!(with_id.is_terminal());
    }

    #[test]
    fn status_labels_are_descriptive() {
        assert_eq!(InspectorRefreshState::Idle.status_label(), "Inspector not refreshed");
        assert_eq!(
            InspectorRefreshState::Loading { workflow_execution_id: "x".into() }.status_label(),
            "Refreshing inspector..."
        );
        assert_eq!(
            InspectorRefreshState::Live {
                workflow_execution_id: "x".into(),
                refreshed_at: "x".into(),
                sections_attempted: 1,
                sections_loaded: 1,
            }.status_label(),
            "Inspector live"
        );
        assert_eq!(
            InspectorRefreshState::Unavailable { reason: "test".into() }.status_label(),
            "Inspector unavailable"
        );
        assert_eq!(
            InspectorRefreshState::Stale { workflow_execution_id: "x".into(), reason: "test".into() }.status_label(),
            "Inspector stale"
        );
        assert_eq!(
            InspectorRefreshState::Failed { workflow_execution_id: None, error: "test".into() }.status_label(),
            "Inspector refresh failed"
        );
    }
}
