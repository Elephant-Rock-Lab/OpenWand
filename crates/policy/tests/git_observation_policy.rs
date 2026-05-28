//! Wave 04b policy tests — git observation governance.
//!
//! Proves:
//! - Exact-name observation rules allow git status/diff/log/branch
//! - Generic ToolEffect::Git rule still escalates for non-observation Git tools
//! - Git mutation tool names are NOT accidentally allowed

use openwand_core::mode::{ConfirmationLevel, InteractionMode};
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::{SessionId, ToolCallId};
use openwand_policy::{
    BuiltinPolicyEngine, PolicyEngine,
    PolicyToolCall, PolicyRequest, PolicyContext,
    GateDecision,
};

fn policy_request(name: &str, effect: ToolEffect) -> PolicyRequest {
    PolicyRequest {
        tool_call: PolicyToolCall {
            id: ToolCallId::new(),
            name: name.into(),
            arguments: serde_json::json!({}),
            declared_effect: effect,
        },
        mode: InteractionMode::Conversational,
        context: PolicyContext {
            working_directory: "/tmp".into(),
            model: "test".into(),
            session_id: SessionId::new(),
            recent_gate_history: vec![],
        },
    }
}

#[tokio::test]
async fn policy_git_status_observation_allows() {
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine
        .evaluate_tool_call(policy_request("local__git_status", ToolEffect::Git))
        .await
        .expect("evaluation should succeed");

    // Conversational mode floors Low → Inform
    assert!(
        matches!(result.decision, GateDecision::RequireConfirmation { level: ConfirmationLevel::Inform }),
        "git_status should be Inform in Conversational, got: {:?}",
        result.decision
    );
    assert_eq!(RiskLevelSnapshot::Low, result.risk_level);
}

#[tokio::test]
async fn policy_git_diff_observation_allows() {
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine
        .evaluate_tool_call(policy_request("local__git_diff", ToolEffect::Git))
        .await
        .expect("evaluation should succeed");

    assert!(
        matches!(result.decision, GateDecision::Allow),
        "git_diff should be allowed, got: {:?}",
        result.decision
    );
    assert_eq!(RiskLevelSnapshot::Medium, result.risk_level);
}

#[tokio::test]
async fn policy_git_log_observation_allows() {
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine
        .evaluate_tool_call(policy_request("local__git_log", ToolEffect::Git))
        .await
        .expect("evaluation should succeed");

    // Conversational mode floors Low → Inform
    assert!(
        matches!(result.decision, GateDecision::RequireConfirmation { level: ConfirmationLevel::Inform }),
        "git_log should be Inform in Conversational, got: {:?}",
        result.decision
    );
    assert_eq!(RiskLevelSnapshot::Low, result.risk_level);
}

#[tokio::test]
async fn policy_git_branch_observation_allows() {
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine
        .evaluate_tool_call(policy_request("local__git_branch", ToolEffect::Git))
        .await
        .expect("evaluation should succeed");

    // Conversational mode floors Low → Inform
    assert!(
        matches!(result.decision, GateDecision::RequireConfirmation { level: ConfirmationLevel::Inform }),
        "git_branch should be Inform in Conversational, got: {:?}",
        result.decision
    );
    assert_eq!(RiskLevelSnapshot::Low, result.risk_level);
}

#[tokio::test]
async fn policy_generic_git_remains_conservative() {
    let engine = BuiltinPolicyEngine::batch1();
    // A hypothetical Git tool that is NOT one of the observation tools
    let result = engine
        .evaluate_tool_call(policy_request("local__git_commit", ToolEffect::Git))
        .await
        .expect("evaluation should succeed");

    // Should require escalation
    assert!(
        matches!(
            result.decision,
            GateDecision::RequireConfirmation { .. }
        ),
        "git_commit should require escalation, got: {:?}",
        result.decision
    );
    assert_eq!(RiskLevelSnapshot::High, result.risk_level);
}

#[tokio::test]
async fn policy_git_mutation_name_not_accidentally_allowed() {
    let engine = BuiltinPolicyEngine::batch1();

    // These names should all require escalation
    let mutation_names = ["local__git_add", "local__git_reset", "local__git_checkout", "local__git_push"];
    for name in mutation_names {
        let result = engine
            .evaluate_tool_call(policy_request(name, ToolEffect::Git))
            .await
            .expect("evaluation should succeed");

        assert!(
            matches!(
                result.decision,
                GateDecision::RequireConfirmation { .. }
            ),
            "{} should require escalation, got: {:?}",
            name,
            result.decision
        );
    }
}
