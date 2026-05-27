//! Fail-closed semantics tests.
//!
//! Proves that policy failure blocks the tool call, not the session.

use openwand_core::mode::{ConfirmationLevel, InteractionMode};
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::{SessionId, ToolCallId};
use openwand_policy::{
    BuiltinPolicyEngine, PolicyEngine, PolicyError, PolicyEvaluation,
    PolicyToolCall, PolicyRequest, PolicyContext,
};

fn make_request(name: &str, effect: ToolEffect, mode: InteractionMode) -> PolicyRequest {
    PolicyRequest {
        tool_call: PolicyToolCall {
            id: ToolCallId::new(),
            name: name.into(),
            arguments: serde_json::json!({}),
            declared_effect: effect,
        },
        mode,
        context: PolicyContext {
            working_directory: "/tmp".into(),
            model: "test".into(),
            session_id: SessionId::new(),
            recent_gate_history: vec![],
        },
    }
}

#[tokio::test]
async fn policy_fail_closed_on_error() {
    let eval = PolicyEvaluation::fail_closed(PolicyError::Internal("db connection lost".into()));
    assert!(eval.decision.is_blocked());
}

#[tokio::test]
async fn policy_fail_closed_is_critical_escalate() {
    let eval = PolicyEvaluation::fail_closed(PolicyError::RuleEvaluation("timeout".into()));
    assert_eq!(RiskLevelSnapshot::Critical, eval.risk_level);
    assert_eq!(ConfirmationLevel::Escalate, eval.confirmation_level);
}

#[tokio::test]
async fn policy_fail_closed_reason_is_safe() {
    // Internal errors must not leak implementation details
    let eval = PolicyEvaluation::fail_closed(PolicyError::Internal("/etc/shadow".into()));
    assert!(!eval.summary.contains("/etc/shadow"), "Internal error must not leak paths");
    assert!(eval.summary.contains("Internal policy error"));
}

#[tokio::test]
async fn policy_malformed_arguments_block_or_error() {
    let engine = BuiltinPolicyEngine::batch1();

    // Tool call with non-object arguments should still be evaluated
    let req = PolicyRequest {
        tool_call: PolicyToolCall {
            id: ToolCallId::new(),
            name: "bash".into(),
            arguments: serde_json::json!("not an object"),
            declared_effect: ToolEffect::Execute,
        },
        mode: InteractionMode::Direct,
        context: PolicyContext {
            working_directory: "/tmp".into(),
            model: "test".into(),
            session_id: SessionId::new(),
            recent_gate_history: vec![],
        },
    };

    let result = engine.evaluate_tool_call(req).await.unwrap();
    // Execute effect must require escalation regardless of arguments shape
    assert!(
        result.decision.requires_confirmation() || result.decision.is_blocked(),
        "Malformed args on execute should not auto-allow"
    );
}

#[tokio::test]
async fn policy_empty_tool_name_blocks() {
    let empty_engine = BuiltinPolicyEngine::new(vec![]);

    let req = make_request("", ToolEffect::Unknown, InteractionMode::Direct);
    let result = empty_engine.evaluate_tool_call(req).await.unwrap();

    assert!(result.decision.is_blocked(), "Empty tool name must fail closed");

    // Also with batch1 rules — Unknown effect is mandatory-deny
    let engine = BuiltinPolicyEngine::batch1();
    let req = make_request("", ToolEffect::Unknown, InteractionMode::Direct);
    let result = engine.evaluate_tool_call(req).await.unwrap();
    assert!(result.decision.is_blocked());
}

#[tokio::test]
async fn policy_unknown_descriptor_does_not_allow_unknown_effect() {
    // If a descriptor is not registered, the declared_effect on the call
    // is what matters. Unknown effect must always block.
    let engine = BuiltinPolicyEngine::batch1();
    let req = make_request("brand_new_tool", ToolEffect::Unknown, InteractionMode::Direct);
    let result = engine.evaluate_tool_call(req).await.unwrap();

    assert!(result.decision.is_blocked());
    // Blocked via the mandatory_unknown_effect rule (block dominance)
    assert!(
        result.reason_code == "unknown_tool_effect" || result.reason_code == "blocked_by_rule",
        "reason_code should reference the unknown effect rule, got: {}",
        result.reason_code
    );
}
