//! Smoke-test wiring regression tests.
//!
//! These verify that the app composition root wires crates correctly.
//! They do NOT connect to real LLM providers — only check tool/policy wiring.

use openwand_tools::composite::CompositeToolExecutor;
use openwand_tools::executor::ToolExecutor;
use openwand_tools::local::batch1_local_tools;

/// Verify that batch1_local_tools exposes the expected tool names.
#[test]
fn app_smoke_wiring_exposes_batch1_local_tools() {
    let tools: CompositeToolExecutor =
        CompositeToolExecutor::local_only(batch1_local_tools());
    let names: Vec<_> = tools
        .available_tools()
        .into_iter()
        .map(|t| t.name)
        .collect();

    assert!(
        names.contains(&"local__file_read".to_string()),
        "expected local__file_read in {:?}",
        names
    );
    assert!(
        names.contains(&"local__file_list".to_string()),
        "expected local__file_list in {:?}",
        names
    );
    assert!(
        names.contains(&"local__file_search".to_string()),
        "expected local__file_search in {:?}",
        names
    );
}

/// Verify that the smoke-test policy profile allows only Read + Search effects.
use openwand_policy::{BuiltinPolicyEngine, PolicyEffect, PolicyRule, PolicyRuleId, ToolMatcher};
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::mode::ConfirmationLevel;
use openwand_core::tool_vocab::ToolEffect;
use openwand_policy::{PolicyEngine, PolicyRequest, PolicyToolCall, PolicyContext};
use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;

fn build_smoke_policy() -> BuiltinPolicyEngine {
    let allow_read = PolicyRule {
        id: PolicyRuleId("smoke-allow-read".into()),
        name: "Allow read-effect tools (smoke)".into(),
        enabled: true,
        priority: 0,
        class: openwand_policy::RuleClass::BuiltinDefault,
        matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Read },
        effect: PolicyEffect::Allow {
            risk: RiskLevelSnapshot::Low,
            confirmation: ConfirmationLevel::Auto,
        },
        reason_code: "smoke_allow_read".into(),
        summary: "Allow read-effect tool calls for smoke testing.".into(),
    };
    let allow_search = PolicyRule {
        id: PolicyRuleId("smoke-allow-search".into()),
        name: "Allow search-effect tools (smoke)".into(),
        enabled: true,
        priority: 0,
        class: openwand_policy::RuleClass::BuiltinDefault,
        matcher: ToolMatcher::ToolEffect { effect: ToolEffect::Search },
        effect: PolicyEffect::Allow {
            risk: RiskLevelSnapshot::Low,
            confirmation: ConfirmationLevel::Auto,
        },
        reason_code: "smoke_allow_search".into(),
        summary: "Allow search-effect tool calls for smoke testing.".into(),
    };
    BuiltinPolicyEngine::new(vec![allow_read, allow_search])
}

fn make_request(effect: ToolEffect) -> PolicyRequest {
    PolicyRequest {
        tool_call: PolicyToolCall {
            id: openwand_core::ToolCallId("tc_test".into()),
            name: "test_tool".into(),
            arguments: serde_json::Value::Null,
            declared_effect: effect,
        },
        mode: InteractionMode::Direct,
        context: PolicyContext {
            working_directory: ".".into(),
            model: "test".into(),
            session_id: SessionId::new(),
            recent_gate_history: vec![],
        },
    }
}

#[tokio::test]
async fn smoke_policy_allows_read_effect() {
    let policy = build_smoke_policy();
    let eval = policy.evaluate_tool_call(make_request(ToolEffect::Read)).await.unwrap();
    assert!(
        matches!(eval.decision, openwand_policy::GateDecision::Allow),
        "Read effect should be allowed, got {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn smoke_policy_allows_search_effect() {
    let policy = build_smoke_policy();
    let eval = policy.evaluate_tool_call(make_request(ToolEffect::Search)).await.unwrap();
    assert!(
        matches!(eval.decision, openwand_policy::GateDecision::Allow),
        "Search effect should be allowed, got {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn smoke_policy_blocks_write_effect() {
    let policy = build_smoke_policy();
    let eval = policy.evaluate_tool_call(make_request(ToolEffect::Write)).await.unwrap();
    assert!(
        matches!(eval.decision, openwand_policy::GateDecision::Block { .. }),
        "Write effect should be blocked, got {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn smoke_policy_blocks_unknown_effect() {
    let policy = build_smoke_policy();
    let eval = policy.evaluate_tool_call(make_request(ToolEffect::Unknown)).await.unwrap();
    assert!(
        matches!(eval.decision, openwand_policy::GateDecision::Block { .. }),
        "Unknown effect should be blocked, got {:?}",
        eval.decision
    );
}

#[tokio::test]
async fn smoke_policy_blocks_delete_effect() {
    let policy = build_smoke_policy();
    let eval = policy.evaluate_tool_call(make_request(ToolEffect::Delete)).await.unwrap();
    assert!(
        matches!(eval.decision, openwand_policy::GateDecision::Block { .. }),
        "Delete effect should be blocked, got {:?}",
        eval.decision
    );
}
