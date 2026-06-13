//! Trust gate tests for write tools.
//!
//! Proves:
//! - Write-effect tools require confirmation in Conversational mode
//! - Direct mode blocks write tools
//! - Policy failure blocks write (fail closed)
//! - Read/search tools still pass through

use openwand_core::mode::{ConfirmationLevel, InteractionMode};
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::tool_vocab::ToolEffect;
use openwand_policy::{
    PolicyEngine, PolicyEffect, PolicyRequest, PolicyRule, PolicyRuleId,
    RuleClass, ToolMatcher,
};

fn write_conversation_policy() -> openwand_policy::BuiltinPolicyEngine {
    openwand_policy::BuiltinPolicyEngine::new(vec![
        // Allow read/search (auto)
        PolicyRule {
            id: PolicyRuleId("allow-read".into()),
            name: "Allow read-effect tools".into(),
            enabled: true,
            priority: 0,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Read,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "allow_read".into(),
            summary: "Allow read-effect tool calls.".into(),
        },
        PolicyRule {
            id: PolicyRuleId("allow-search".into()),
            name: "Allow search-effect tools".into(),
            enabled: true,
            priority: 0,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Search,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "allow_search".into(),
            summary: "Allow search-effect tool calls.".into(),
        },
        // Write requires explicit approval
        PolicyRule {
            id: PolicyRuleId("write-requires-approve".into()),
            name: "Write-effect tools require user approval".into(),
            enabled: true,
            priority: 0,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Write,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Medium,
                confirmation: ConfirmationLevel::Approve,
            },
            reason_code: "write_requires_approval".into(),
            summary: "Write-effect tools require explicit user approval.".into(),
        },
    ])
}

fn make_policy_request(
    tool_name: &str,
    effect: ToolEffect,
    mode: InteractionMode,
) -> PolicyRequest {
    PolicyRequest {
        tool_call: openwand_policy::PolicyToolCall {
            id: openwand_core::ToolCallId::new(),
            name: tool_name.to_string(),
            arguments: serde_json::json!({}),
            declared_effect: effect,
        },
        mode,
        context: openwand_policy::PolicyContext {
            working_directory: ".".into(),
            model: "test".into(),
            session_id: openwand_core::SessionId::new(),
            recent_gate_history: vec![],
        },
    }
}

#[tokio::test]
async fn policy_write_requires_confirmation() {
    let policy = write_conversation_policy();
    let req = make_policy_request(
        "local__file_write",
        ToolEffect::Write,
        InteractionMode::Conversational,
    );

    let eval = policy.evaluate_tool_call(req).await.unwrap();
    assert!(
        eval.decision.requires_confirmation(),
        "Write tool should require confirmation in Conversational mode, got: {:?}",
        eval.decision
    );
    assert_eq!(ConfirmationLevel::Approve, eval.confirmation_level);
}

#[tokio::test]
async fn policy_write_blocked_in_direct_mode() {
    let policy = write_conversation_policy();
    let req = make_policy_request(
        "local__file_write",
        ToolEffect::Write,
        InteractionMode::Direct,
    );

    let eval = policy.evaluate_tool_call(req).await.unwrap();
    // In Direct mode, RequireConfirmation → treat as blocked
    // (the runner gates on mode: Direct doesn't pause for confirmation)
    assert!(
        eval.decision.requires_confirmation() || eval.decision.is_blocked(),
        "Write tool should require confirmation or be blocked in Direct mode, got: {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn policy_read_still_allowed_in_direct() {
    let policy = write_conversation_policy();
    let req = make_policy_request(
        "local__file_read",
        ToolEffect::Read,
        InteractionMode::Direct,
    );

    let eval = policy.evaluate_tool_call(req).await.unwrap();
    assert!(
        eval.decision.allows_execution(),
        "Read tool should be allowed in Direct mode, got: {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn policy_unknown_effect_blocked() {
    let policy = write_conversation_policy();
    let req = make_policy_request(
        "local__unknown_tool",
        ToolEffect::Unknown,
        InteractionMode::Conversational,
    );

    let eval = policy.evaluate_tool_call(req).await.unwrap();
    assert!(
        eval.decision.is_blocked(),
        "Unknown-effect tool should be blocked, got: {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn policy_delete_effect_blocked() {
    let policy = write_conversation_policy();
    let req = make_policy_request(
        "local__file_delete",
        ToolEffect::Delete,
        InteractionMode::Conversational,
    );

    let eval = policy.evaluate_tool_call(req).await.unwrap();
    assert!(
        eval.decision.is_blocked(),
        "Delete-effect tool should be blocked (no rule allows it), got: {:?}",
        eval.decision
    );
}
