//! Authority boundary tests.
//!
//! Proves that filter_tools is defense-in-depth only and
//! evaluate_tool_call remains the mandatory authority boundary.

use openwand_core::mode::InteractionMode;
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::{SessionId, ToolCallId};
use openwand_policy::{
    BuiltinPolicyEngine, PolicyEngine, PolicyToolCall, PolicyToolDescriptor,
    PolicyRequest, PolicyContext, ToolFilterRequest, PolicyToolSource,
};

fn make_request(name: &str, effect: ToolEffect) -> PolicyRequest {
    PolicyRequest {
        tool_call: PolicyToolCall {
            id: ToolCallId::new(),
            name: name.into(),
            arguments: serde_json::json!({}),
            declared_effect: effect,
        },
        mode: InteractionMode::Direct,
        context: PolicyContext {
            working_directory: "/tmp".into(),
            model: "test".into(),
            session_id: SessionId::new(),
            recent_gate_history: vec![],
        },
    }
}

fn ctx() -> PolicyContext {
    PolicyContext {
        working_directory: "/tmp".into(),
        model: "test".into(),
        session_id: SessionId::new(),
        recent_gate_history: vec![],
    }
}

#[tokio::test]
async fn policy_filter_tools_is_not_authority_boundary() {
    // filter_tools removes PolicyChange tools from the prompt surface.
    // But if the LLM still emits a PolicyChange call, evaluate_tool_call MUST block it.
    let engine = BuiltinPolicyEngine::batch1();

    let desc = PolicyToolDescriptor {
        name: "edit_policy".into(),
        source: PolicyToolSource::Local,
        declared_effect: ToolEffect::PolicyChange,
        risk_hints: vec![],
        tags: vec![],
    };

    // filter_tools removes it
    let filtered = engine
        .filter_tools(ToolFilterRequest {
            tools: vec![desc],
            mode: InteractionMode::Direct,
            context: ctx(),
        })
        .await
        .unwrap();
    assert!(filtered.is_empty(), "PolicyChange must be filtered from prompt");

    // But evaluate_tool_call still blocks it if the LLM emits it anyway
    let eval = engine
        .evaluate_tool_call(make_request("edit_policy", ToolEffect::PolicyChange))
        .await
        .unwrap();
    assert!(eval.decision.is_blocked(), "Hidden PolicyChange call must still be blocked");
}

#[tokio::test]
async fn policy_filtered_tool_call_still_evaluates() {
    let engine = BuiltinPolicyEngine::batch1();

    // AuthChange is filtered from prompt
    let filtered = engine
        .filter_tools(ToolFilterRequest {
            tools: vec![PolicyToolDescriptor {
                name: "set_key".into(),
                source: PolicyToolSource::Local,
                declared_effect: ToolEffect::AuthChange,
                risk_hints: vec![],
                tags: vec![],
            }],
            mode: InteractionMode::Direct,
            context: ctx(),
        })
        .await
        .unwrap();
    assert!(filtered.is_empty());

    // LLM emits it anyway → must be blocked by evaluate_tool_call
    let eval = engine
        .evaluate_tool_call(make_request("set_key", ToolEffect::AuthChange))
        .await
        .unwrap();
    assert!(eval.decision.is_blocked());
}

#[tokio::test]
async fn policy_all_mandatory_deny_rules_cannot_be_weakened() {
    let engine = BuiltinPolicyEngine::batch1();

    // Find all MandatoryDeny rules
    let mandatory: Vec<_> = engine
        .rules()
        .iter()
        .filter(|r| r.class == openwand_policy::RuleClass::MandatoryDeny)
        .collect();

    assert!(!mandatory.is_empty(), "Must have at least one MandatoryDeny rule");

    // Each MandatoryDeny rule must produce a Block effect
    for rule in &mandatory {
        assert!(
            matches!(rule.effect, openwand_policy::PolicyEffect::Block),
            "MandatoryDeny rule '{}' must have Block effect",
            rule.id.0
        );
    }

    // Each MandatoryDeny rule must have higher priority than BuiltinDefault rules
    let max_default_priority = engine
        .rules()
        .iter()
        .filter(|r| r.class == openwand_policy::RuleClass::BuiltinDefault)
        .map(|r| r.priority)
        .max()
        .unwrap_or(0);

    for rule in &mandatory {
        assert!(
            rule.priority > max_default_priority,
            "MandatoryDeny rule '{}' (priority {}) must outrank BuiltinDefault max ({})",
            rule.id.0,
            rule.priority,
            max_default_priority
        );
    }
}
