//! Deterministic policy evaluation engine.

use async_trait::async_trait;
use openwand_core::mode::ConfirmationLevel;
use openwand_core::risk::RiskLevelSnapshot;

use crate::builtin::batch1_rules;
use crate::decision::{GateDecision, GateFinding, GateFindingResult, PolicyEvaluation};
use crate::engine::PolicyEngine;
use crate::error::PolicyError;
use crate::mapping::{apply_mode_floor, confirmation_for_risk, risk_order};
use crate::request::{PolicyRequest, ToolFilterRequest};
use crate::rule::PolicyEffect;
use crate::tool::PolicyToolDescriptor;

/// Deterministic built-in policy engine for Batch 1.
///
/// Rules are loaded at construction and evaluated synchronously.
/// Evaluation is deterministic: same inputs → same outputs.
/// Fail-closed: any error → block the tool call.
pub struct BuiltinPolicyEngine {
    rules: Vec<crate::rule::PolicyRule>,
}

impl BuiltinPolicyEngine {
    pub fn batch1() -> Self {
        Self {
            rules: batch1_rules(),
        }
    }

    /// Create with custom rules (for testing).
    pub fn new(rules: Vec<crate::rule::PolicyRule>) -> Self {
        Self { rules }
    }

    pub fn rules(&self) -> &[crate::rule::PolicyRule] {
        &self.rules
    }
}

/// Rank confirmation levels for comparison. Higher = more restrictive.
fn confirmation_rank(level: &ConfirmationLevel) -> u8 {
    match level {
        ConfirmationLevel::Auto => 0,
        ConfirmationLevel::Inform => 1,
        ConfirmationLevel::Approve => 2,
        ConfirmationLevel::Escalate => 3,
    }
}

#[async_trait]
impl PolicyEngine for BuiltinPolicyEngine {
    async fn evaluate_tool_call(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyEvaluation, PolicyError> {
        let gate_id = openwand_core::GateId::new();

        // 1. Collect enabled matching rules
        let mut findings = Vec::new();
        let mut matched_rule_ids = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if rule.matcher.matches(&request.tool_call, None) {
                let result = match &rule.effect {
                    PolicyEffect::Allow { risk, .. } => GateFindingResult::Require {
                        risk: risk.clone(),
                    },
                    PolicyEffect::Block => GateFindingResult::Block,
                };

                findings.push(GateFinding {
                    rule_id: Some(rule.id.clone()),
                    result,
                    reason: rule.summary.clone(),
                });
                matched_rule_ids.push(rule.id.clone());
            }
        }

        // 2. No rules matched → fail closed
        if findings.is_empty() {
            return Ok(PolicyEvaluation {
                gate_id,
                decision: GateDecision::Block {
                    reason: "No policy rule matched; defaulting to block.".into(),
                },
                risk_level: RiskLevelSnapshot::Critical,
                confirmation_level: ConfirmationLevel::Escalate,
                findings,
                matched_rules: matched_rule_ids,
                reason_code: "no_matching_rule".into(),
                summary: "No policy rule matched this tool call.".into(),
                rollback_required: false,
                rollback_plan: None,
            });
        }

        // 3. Block dominance: any Block finding → final Block
        let has_block = findings
            .iter()
            .any(|f| matches!(f.result, GateFindingResult::Block));

        if has_block {
            let reasons: Vec<&str> = findings
                .iter()
                .filter(|f| matches!(f.result, GateFindingResult::Block))
                .map(|f| f.reason.as_str())
                .collect();
            let reason_str = reasons.join("; ");
            return Ok(PolicyEvaluation {
                gate_id,
                decision: GateDecision::Block {
                    reason: reason_str.clone(),
                },
                risk_level: RiskLevelSnapshot::Critical,
                confirmation_level: ConfirmationLevel::Escalate,
                findings,
                matched_rules: matched_rule_ids,
                reason_code: "blocked_by_rule".into(),
                summary: format!("Blocked: {}", reason_str),
                rollback_required: false,
                rollback_plan: None,
            });
        }

        // 4. Aggregate maximum risk from all findings
        let max_risk = findings
            .iter()
            .filter_map(|f| match &f.result {
                GateFindingResult::Require { risk } => Some(risk),
                _ => None,
            })
            .max_by(|a, b| risk_order(a).cmp(&risk_order(b)))
            .cloned()
            .unwrap_or(RiskLevelSnapshot::Low);

        // 4b. Get the declared confirmation from the highest-priority matching Allow rule.
        //     Rules declare their own confirmation; we don't re-derive from risk alone.
        let declared_confirmation = self
            .rules
            .iter()
            .filter(|r| r.enabled)
            .filter(|r| r.matcher.matches(&request.tool_call, None))
            .filter_map(|r| match &r.effect {
                PolicyEffect::Allow { confirmation, .. } => Some(confirmation.clone()),
                PolicyEffect::Block => None,
            })
            .max_by(|a, b| confirmation_rank(a).cmp(&confirmation_rank(b)))
            .unwrap_or(confirmation_for_risk(&max_risk));

        // 5. Apply mode floor on top of declared confirmation
        let confirmation = apply_mode_floor(&request.mode, &max_risk, &declared_confirmation);

        // 6. Decision
        let decision = match &confirmation {
            ConfirmationLevel::Auto => GateDecision::Allow,
            other => GateDecision::RequireConfirmation { level: other.clone() },
        };

        let rollback_required = confirmation == ConfirmationLevel::Escalate;

        Ok(PolicyEvaluation {
            gate_id,
            decision,
            risk_level: max_risk,
            confirmation_level: confirmation.clone(),
            findings,
            matched_rules: matched_rule_ids,
            reason_code: "evaluated".into(),
            summary: format!("Policy evaluation: {:?}", confirmation),
            rollback_required,
            rollback_plan: None,
        })
    }

    async fn filter_tools(
        &self,
        request: ToolFilterRequest,
    ) -> Result<Vec<PolicyToolDescriptor>, PolicyError> {
        // Defense-in-depth: remove tools whose declared_effect is blocked.
        // This is NOT the authority boundary — evaluate_tool_call is.
        let allowed: Vec<PolicyToolDescriptor> = request
            .tools
            .into_iter()
            .filter(|tool| {
                // Check if any mandatory-deny rule blocks this effect
                for rule in &self.rules {
                    if !rule.enabled {
                        continue;
                    }
                    // Simple effect check — we don't have a call, just the descriptor
                    let matches = matches!(
                        &rule.matcher,
                        crate::rule::ToolMatcher::ToolEffect { effect } if *effect == tool.declared_effect
                    );
                    if matches && matches!(rule.effect, PolicyEffect::Block) {
                        return false;
                    }
                }
                true
            })
            .collect();

        Ok(allowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{PolicyToolCall, PolicyToolSource};
    use openwand_core::mode::InteractionMode;
    use openwand_core::tool_vocab::ToolEffect;
    use openwand_core::{SessionId, ToolCallId};

    fn make_request(name: &str, effect: ToolEffect, mode: InteractionMode) -> PolicyRequest {
        PolicyRequest {
            tool_call: PolicyToolCall {
                id: ToolCallId::new(),
                name: name.into(),
                arguments: serde_json::json!({}),
                declared_effect: effect,
            },
            mode,
            context: crate::request::PolicyContext {
                working_directory: "/tmp".into(),
                model: "test".into(),
                session_id: SessionId::new(),
                recent_gate_history: vec![],
            },
        }
    }

    fn make_filter_request(
        tools: Vec<PolicyToolDescriptor>,
    ) -> ToolFilterRequest {
        ToolFilterRequest {
            tools,
            mode: InteractionMode::Conversational,
            context: crate::request::PolicyContext {
                working_directory: "/tmp".into(),
                model: "test".into(),
                session_id: SessionId::new(),
                recent_gate_history: vec![],
            },
        }
    }

    fn desc(name: &str, effect: ToolEffect) -> PolicyToolDescriptor {
        PolicyToolDescriptor {
            name: name.into(),
            source: PolicyToolSource::Local,
            declared_effect: effect,
            risk_hints: vec![],
            tags: vec![],
        }
    }

    // ── Rule set ──

    #[test]
    fn builtin_batch1_has_rules() {
        let engine = BuiltinPolicyEngine::batch1();
        assert!(!engine.rules().is_empty(), "batch1 rules must not be empty");
        // Must have at least: unknown block, read allow, search allow
        assert!(engine.rules().iter().any(|r| r.id.0 == "mandatory_unknown_effect"));
        assert!(engine.rules().iter().any(|r| r.id.0 == "allow_read"));
        assert!(engine.rules().iter().any(|r| r.id.0 == "allow_search"));
    }

    // ── Effect-based decisions ──

    #[tokio::test]
    async fn policy_read_allows_auto() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("read_file", ToolEffect::Read, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.allows_execution(), "Read should be allowed");
        assert_eq!(ConfirmationLevel::Auto, result.confirmation_level);
    }

    #[tokio::test]
    async fn policy_search_allows_or_informs() {
        let engine = BuiltinPolicyEngine::batch1();

        // Direct mode: Auto
        let direct = engine
            .evaluate_tool_call(make_request("search", ToolEffect::Search, InteractionMode::Direct))
            .await
            .unwrap();
        assert!(
            direct.decision.allows_execution(),
            "Search should be allowed in Direct mode"
        );

        // Conversational mode: floor raises Auto to Inform
        let conv = engine
            .evaluate_tool_call(make_request("search", ToolEffect::Search, InteractionMode::Conversational))
            .await
            .unwrap();
        assert!(
            conv.decision.allows_execution() || conv.decision.requires_confirmation(),
            "Search in Conversational: allow or require confirmation"
        );
        // Mode floor should raise Low risk from Auto to Inform in Conversational
        assert_ne!(
            ConfirmationLevel::Auto,
            conv.confirmation_level,
            "Conversational mode should floor Low to at least Inform"
        );
    }

    #[tokio::test]
    async fn policy_unknown_blocks() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("mystery", ToolEffect::Unknown, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.is_blocked(), "Unknown effect must be blocked");
        assert_eq!(RiskLevelSnapshot::Critical, result.risk_level);
    }

    #[tokio::test]
    async fn policy_write_requires_confirmation() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("write_file", ToolEffect::Write, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(
            result.decision.requires_confirmation(),
            "Write should require confirmation"
        );
        assert_eq!(ConfirmationLevel::Approve, result.confirmation_level);
    }

    #[tokio::test]
    async fn policy_delete_escalates_or_blocks() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("rm", ToolEffect::Delete, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(
            result.decision.requires_confirmation() || result.decision.is_blocked(),
            "Delete must require escalation or block"
        );
        assert!(
            result.confirmation_level == ConfirmationLevel::Escalate,
            "Delete must escalate"
        );
    }

    #[tokio::test]
    async fn policy_execute_requires_escalate() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("bash", ToolEffect::Execute, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.requires_confirmation());
        assert_eq!(ConfirmationLevel::Escalate, result.confirmation_level);
    }

    #[tokio::test]
    async fn policy_network_requires_approval() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("curl", ToolEffect::Network, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.requires_confirmation());
        assert_eq!(ConfirmationLevel::Approve, result.confirmation_level);
    }

    #[tokio::test]
    async fn policy_dependency_change_escalates() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("cargo_add", ToolEffect::DependencyChange, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.requires_confirmation());
        assert_eq!(ConfirmationLevel::Escalate, result.confirmation_level);
    }

    #[tokio::test]
    async fn policy_policy_change_blocks() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("edit_policy", ToolEffect::PolicyChange, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.is_blocked());
        assert_eq!(RiskLevelSnapshot::Critical, result.risk_level);
    }

    #[tokio::test]
    async fn policy_auth_change_blocks() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .evaluate_tool_call(make_request("set_api_key", ToolEffect::AuthChange, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.is_blocked());
    }

    // ── Edge cases ──

    #[tokio::test]
    async fn policy_no_matching_rule_blocks() {
        // Create a tool with an effect that has no matching rule
        // All effects in core have rules in batch1, so let's use an empty engine
        let empty_engine = BuiltinPolicyEngine { rules: vec![] };
        let result = empty_engine
            .evaluate_tool_call(make_request("read_file", ToolEffect::Read, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.is_blocked(), "No rules must fail closed");
        assert_eq!("no_matching_rule", result.reason_code);
    }

    #[tokio::test]
    async fn policy_block_dominance_wins() {
        // If a tool somehow matches both an allow rule and a block rule,
        // block must win.
        let engine = BuiltinPolicyEngine {
            rules: vec![
                crate::rule::PolicyRule {
                    id: crate::rule::PolicyRuleId("allow".into()),
                    name: "Allow".into(),
                    enabled: true,
                    priority: 50,
                    class: crate::rule::RuleClass::BuiltinDefault,
                    matcher: crate::rule::ToolMatcher::Any,
                    effect: crate::rule::PolicyEffect::Allow {
                        risk: RiskLevelSnapshot::Low,
                        confirmation: ConfirmationLevel::Auto,
                    },
                    reason_code: "allow".into(),
                    summary: "Allow all".into(),
                },
                crate::rule::PolicyRule {
                    id: crate::rule::PolicyRuleId("block".into()),
                    name: "Block".into(),
                    enabled: true,
                    priority: 100,
                    class: crate::rule::RuleClass::MandatoryDeny,
                    matcher: crate::rule::ToolMatcher::Any,
                    effect: crate::rule::PolicyEffect::Block,
                    reason_code: "blocked".into(),
                    summary: "Block all".into(),
                },
            ],
        };

        let result = engine
            .evaluate_tool_call(make_request("anything", ToolEffect::Read, InteractionMode::Direct))
            .await
            .unwrap();

        assert!(result.decision.is_blocked(), "Block must dominate");
    }

    #[tokio::test]
    async fn policy_mode_floor_never_lowers_risk() {
        let engine = BuiltinPolicyEngine::batch1();

        // Write in Direct mode → Approve
        let direct = engine
            .evaluate_tool_call(make_request("write", ToolEffect::Write, InteractionMode::Direct))
            .await
            .unwrap();
        let direct_level = direct.confirmation_level.clone();

        // Write in Conversational mode → should be >= Approve
        let conv = engine
            .evaluate_tool_call(make_request("write", ToolEffect::Write, InteractionMode::Conversational))
            .await
            .unwrap();
        let conv_level = conv.confirmation_level.clone();

        // Conversational must never lower confirmation
        assert!(
            super::confirmation_rank(&conv_level) >= super::confirmation_rank(&direct_level),
            "Conversational ({:?}) must not be lower than Direct ({:?})",
            conv_level,
            direct_level
        );
    }

    // ── Filter tools ──

    #[tokio::test]
    async fn filter_tools_removes_blocked_effects() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .filter_tools(make_filter_request(vec![
                desc("read", ToolEffect::Read),
                desc("unknown_tool", ToolEffect::Unknown),
                desc("edit_policy", ToolEffect::PolicyChange),
                desc("set_key", ToolEffect::AuthChange),
            ]))
            .await
            .unwrap();

        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(vec!["read"], names, "Only read should survive filtering");
    }

    #[tokio::test]
    async fn filter_tools_keeps_read_and_search() {
        let engine = BuiltinPolicyEngine::batch1();
        let result = engine
            .filter_tools(make_filter_request(vec![
                desc("read", ToolEffect::Read),
                desc("search", ToolEffect::Search),
                desc("write", ToolEffect::Write),
            ]))
            .await
            .unwrap();

        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert!(
            names.contains(&"read"),
            "Read must survive filtering"
        );
        assert!(
            names.contains(&"search"),
            "Search must survive filtering"
        );
        assert!(
            names.contains(&"write"),
            "Write should survive filtering (it requires confirmation, not blocked)"
        );
    }

    /// Helper: rank confirmation levels for comparison.
    /// Higher = more restrictive.
    fn _confirmation_rank(level: &ConfirmationLevel) -> u8 {
        super::confirmation_rank(level)
    }
}
