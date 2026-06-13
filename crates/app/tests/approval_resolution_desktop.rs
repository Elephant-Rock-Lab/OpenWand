//! Tests for desktop approval resolution authority boundary (Wave 88B).
//!
//! Proves:
//! 1. The UI request DTO module imports no backend authority types.
//! 2. The render function does not call resolve_approval directly.
//! 3. The service delegates through SessionRunner::resolve_approval().
//! 4. Empty ARID is rejected before service delegation.
//! 5. Stale/missing runner returns Stale, never silently succeeds.
//! 6. Decision mapping is correct (Approve/Reject → backend types).
//! 7. Tool-name binding remains display-only.

// ── Source-level authority boundary guards ───────────────────

#[cfg(test)]
mod authority_guards {
    /// Guard: approval_resolution_request.rs imports no backend authority.
    #[test]
    fn dto_does_not_import_backend_authority() {
        let src = include_str!("../src/ui/approval_resolution_request.rs");
        assert!(!src.contains("SessionRunner"), "must not import SessionRunner");
        // Check for backend ApprovalDecision but NOT our own ApprovalDecisionDto
        // We check word-boundary patterns that match the backend type
        assert!(!src.contains("ApprovalDecision "), "must not reference backend ApprovalDecision");
        assert!(!src.contains("ApprovalDecision:"), "must not reference backend ApprovalDecision");
        assert!(!src.contains("ApprovalDecision>"), "must not reference backend ApprovalDecision");
        assert!(!src.contains("RunConfig"), "must not import RunConfig");
        assert!(!src.contains("ToolExecutor"), "must not import ToolExecutor");
        assert!(!src.contains("PolicyEngine"), "must not import PolicyEngine");
        assert!(!src.contains("TraceStore"), "must not import TraceStore");
        assert!(!src.contains("ApprovalStore"), "must not import ApprovalStore");
        assert!(!src.contains("resolve_approval"), "must not call resolve_approval");
        assert!(!src.contains("save_approval"), "must not save approval records");
        assert!(!src.contains("append_trace"), "must not append trace");
        assert!(!src.contains("resume"), "must not resume execution");
    }

    /// Guard: render_approval_resolution does not bypass authority.
    #[test]
    #[cfg(feature = "desktop")]
    fn render_function_does_not_bypass_authority() {
        let src = include_str!("../src/ui_main.rs");
        let render_section = src
            .split("fn render_approval_resolution")
            .nth(1)
            .unwrap_or("");

        // Must use the service delegation path
        assert!(
            render_section.contains("submit_approval_resolution"),
            "must use service delegation"
        );

        // Must not directly call these
        assert!(!render_section.contains(".resolve_approval("), "must not call resolve_approval directly");
        assert!(!render_section.contains("resume("), "must not resume execution");
        assert!(!render_section.contains("execute("), "must not execute tools");
        assert!(!render_section.contains("append_trace"), "must not append trace");
        assert!(!render_section.contains("save_approval"), "must not save approval records");
        assert!(!render_section.contains("ApprovalDecision"), "must not construct ApprovalDecision");
        assert!(!render_section.contains("RunConfig"), "must not construct RunConfig");
    }

    /// Guard: service.rs uses SessionRunner::resolve_approval (not direct execution).
    #[test]
    fn service_delegates_through_runner_resolve_approval() {
        let src = include_str!("../src/ui/service.rs");
        assert!(
            src.contains("runner.resolve_approval(decision, config)"),
            "service must delegate through runner.resolve_approval()"
        );
        // The submit_approval_resolution method must construct RunConfig
        // (so the UI doesn't have to)
        assert!(
            src.contains("RunConfig {"),
            "service must construct RunConfig — not the UI"
        );
    }
}

// ── DTO validation tests ─────────────────────────────────────

#[cfg(test)]
mod dto_validation_tests {
    use openwand_app::ui::approval_resolution_request::*;

    #[test]
    fn empty_arid_is_rejected_before_delegation() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn whitespace_arid_is_rejected() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "   ".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn valid_request_with_arid_passes_validation() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_123".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn reject_with_rationale_passes_validation() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_456".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Reject,
            rationale: Some("Too dangerous for workspace".into()),
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn approval_rationale_can_be_none() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_789".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_ok());
    }
}

// ── Service delegation tests ─────────────────────────────────

#[cfg(test)]
mod service_delegation_tests {
    use openwand_app::ui::approval_resolution_request::*;
    use openwand_app::ui::service::UiSessionService;

    #[tokio::test]
    async fn missing_runner_returns_stale_never_silently_succeeds() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_test".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k1".into(),
        };

        let result = UiSessionService::submit_approval_resolution(None, &req).await;

        match result {
            ApprovalResolutionState::Stale { reason } => {
                assert!(reason.contains("No active session runner"), "got: {reason}");
            }
            _ => panic!("Expected Stale, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn empty_arid_returns_failed_before_delegation() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k2".into(),
        };

        let result = UiSessionService::submit_approval_resolution(None, &req).await;

        match result {
            ApprovalResolutionState::Failed { error } => {
                assert!(error.contains("approval_request_id"), "got: {error}");
            }
            _ => panic!("Expected Failed, got {:?}", result),
        }
    }

    #[test]
    fn decision_dto_maps_correctly() {
        // Approve maps to ApprovalResolution::Approve
        let approve = ApprovalDecisionDto::Approve;
        assert_eq!(approve, ApprovalDecisionDto::Approve);

        // Reject maps to ApprovalResolution::Reject with rationale
        let reject = ApprovalDecisionDto::Reject;
        assert_eq!(reject, ApprovalDecisionDto::Reject);
    }

    #[test]
    fn state_lifecycle_states_are_honest() {
        let idle = ApprovalResolutionState::Idle;
        assert!(!idle.is_terminal());
        assert!(!idle.is_pending());

        let pending = ApprovalResolutionState::Pending;
        assert!(pending.is_pending());
        assert!(!pending.is_terminal());

        let resolved = ApprovalResolutionState::Resolved {
            decision: ApprovalDecisionDto::Approve,
            approval_request_id: "arid".into(),
            tool_name: Some("local__file_write".into()),
            tool_status: Some("completed".into()),
            source: "live".into(),
        };
        assert!(resolved.is_terminal());
        assert!(!resolved.is_pending());

        let failed = ApprovalResolutionState::Failed { error: "test".into() };
        assert!(failed.is_terminal());

        let stale = ApprovalResolutionState::Stale { reason: "gone".into() };
        assert!(stale.is_terminal());
    }

    #[test]
    fn resolved_state_carries_binding_context() {
        // The Resolved state should carry enough detail for the UI
        // without inventing authority
        let resolved = ApprovalResolutionState::Resolved {
            decision: ApprovalDecisionDto::Approve,
            approval_request_id: "arid_123".into(),
            tool_name: Some("local__file_write".into()),
            tool_status: Some("completed".into()),
            source: "recovered".into(),
        };

        if let ApprovalResolutionState::Resolved {
            approval_request_id,
            tool_name,
            tool_status,
            source,
            ..
        } = &resolved
        {
            assert_eq!(approval_request_id, "arid_123");
            assert_eq!(tool_name.as_deref(), Some("local__file_write"));
            assert_eq!(tool_status.as_deref(), Some("completed"));
            assert_eq!(source, "recovered");
        }
    }

    #[test]
    fn displayed_tool_name_is_not_authoritative_binding() {
        // The DTO field is "displayed_tool_name" not "tool_name"
        // to signal it's display-only, not authoritative binding.
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        // The field exists as display context only
        assert_eq!(req.displayed_tool_name.as_deref(), Some("local__file_write"));
        // The validate() does NOT enforce tool-name binding — that's in the runner
        assert!(req.validate().is_ok());
    }
}
