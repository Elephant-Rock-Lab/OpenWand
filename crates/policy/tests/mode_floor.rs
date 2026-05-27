//! Mode floor and rule-declared confirmation tests.
//!
//! Proves:
//! - Rule-declared confirmation is canonical (not re-derived from risk)
//! - Mode floor can only increase, never decrease confirmation
//! - Mode floor is the only post-rule adjustment

use openwand_core::mode::{ConfirmationLevel, InteractionMode};
use openwand_core::tool_vocab::ToolEffect;
use openwand_core::{SessionId, ToolCallId};
use openwand_policy::{
    BuiltinPolicyEngine, PolicyEngine, PolicyToolCall, PolicyRequest, PolicyContext,
    apply_mode_floor, confirmation_for_risk,
};

fn make_request(effect: ToolEffect, mode: InteractionMode) -> PolicyRequest {
    PolicyRequest {
        tool_call: PolicyToolCall {
            id: ToolCallId::new(),
            name: "test_tool".into(),
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

fn confirmation_rank(level: &ConfirmationLevel) -> u8 {
    match level {
        ConfirmationLevel::Auto => 0,
        ConfirmationLevel::Inform => 1,
        ConfirmationLevel::Approve => 2,
        ConfirmationLevel::Escalate => 3,
    }
}

#[tokio::test]
async fn policy_rule_declared_confirmation_is_preserved() {
    // Write rule declares Approve. In Direct mode (no floor), the final
    // confirmation must be exactly Approve, not re-derived from Medium risk.
    let engine = BuiltinPolicyEngine::batch1();
    let result = engine
        .evaluate_tool_call(make_request(ToolEffect::Write, InteractionMode::Direct))
        .await
        .unwrap();

    assert_eq!(
        ConfirmationLevel::Approve,
        result.confirmation_level,
        "Rule-declared Approve must be preserved in Direct mode"
    );

    // Verify it's NOT the risk-derived value
    // Medium risk → confirmation_for_risk = Inform, but rule says Approve
    let risk_derived = confirmation_for_risk(&openwand_core::risk::RiskLevelSnapshot::Medium);
    assert_ne!(
        risk_derived,
        result.confirmation_level,
        "Rule-declared confirmation must differ from risk-derived default"
    );
}

#[tokio::test]
async fn policy_mode_floor_can_increase_declared_confirmation() {
    // Search in Direct mode: Low risk, Auto confirmation from rule
    let engine = BuiltinPolicyEngine::batch1();

    let direct = engine
        .evaluate_tool_call(make_request(ToolEffect::Search, InteractionMode::Direct))
        .await
        .unwrap();

    // Conversational mode floors Low risk to Inform
    let conv = engine
        .evaluate_tool_call(make_request(ToolEffect::Search, InteractionMode::Conversational))
        .await
        .unwrap();

    assert!(
        confirmation_rank(&conv.confirmation_level) >= confirmation_rank(&direct.confirmation_level),
        "Conversational ({:?}) must be >= Direct ({:?})",
        conv.confirmation_level,
        direct.confirmation_level,
    );
}

#[tokio::test]
async fn policy_mode_floor_never_decreases_declared_confirmation() {
    // Test all modes for a high-risk effect (Delete = Escalate)
    let engine = BuiltinPolicyEngine::batch1();
    let modes = [
        InteractionMode::Direct,
        InteractionMode::Conversational,
        InteractionMode::AutoRouting,
    ];

    let mut results = Vec::new();
    for mode in &modes {
        let result = engine
            .evaluate_tool_call(make_request(ToolEffect::Delete, mode.clone()))
            .await
            .unwrap();
        results.push((mode.clone(), result.confirmation_level.clone()));
    }

    // All modes must produce at least Escalate (rule-declared)
    for (mode, level) in &results {
        assert!(
            confirmation_rank(level) >= confirmation_rank(&ConfirmationLevel::Escalate),
            "Mode {:?} produced {:?}, which is below Escalate",
            mode,
            level,
        );
    }
}

#[test]
fn apply_mode_floor_direct_never_raises() {
    use openwand_core::risk::RiskLevelSnapshot;

    // Direct mode should return base unchanged for all risk levels
    for risk in [
        RiskLevelSnapshot::Low,
        RiskLevelSnapshot::Medium,
        RiskLevelSnapshot::High,
        RiskLevelSnapshot::Critical,
    ] {
        let base = confirmation_for_risk(&risk);
        let result = apply_mode_floor(&InteractionMode::Direct, &risk, &base);
        assert_eq!(
            base, result,
            "Direct mode must not change base confirmation for risk {:?}",
            risk
        );
    }
}

#[test]
fn apply_mode_floor_conversational_raises_low() {
    use openwand_core::risk::RiskLevelSnapshot;

    // Conversational floors Low risk Auto → Inform
    let base = ConfirmationLevel::Auto;
    let result = apply_mode_floor(
        &InteractionMode::Conversational,
        &RiskLevelSnapshot::Low,
        &base,
    );
    assert_eq!(ConfirmationLevel::Inform, result);

    // But should not lower higher risk levels
    let base = ConfirmationLevel::Escalate;
    let result = apply_mode_floor(
        &InteractionMode::Conversational,
        &RiskLevelSnapshot::Critical,
        &base,
    );
    assert_eq!(ConfirmationLevel::Escalate, result);
}
