//! Batch 1 built-in policy rules.

use openwand_core::mode::ConfirmationLevel;
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::tool_vocab::ToolEffect;

use crate::rule::{PolicyEffect, PolicyRule, PolicyRuleId, RuleClass, ToolMatcher};

pub fn batch1_rules() -> Vec<PolicyRule> {
    vec![
        // ── MandatoryDeny ──

        PolicyRule {
            id: PolicyRuleId("mandatory_unknown_effect".into()),
            name: "Block unknown tool effects".into(),
            enabled: true,
            priority: 200,
            class: RuleClass::MandatoryDeny,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Unknown,
            },
            effect: PolicyEffect::Block,
            reason_code: "unknown_tool_effect".into(),
            summary: "Tool effect is unknown; blocked for safety.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("mandatory_policy_change".into()),
            name: "Block policy changes".into(),
            enabled: true,
            priority: 200,
            class: RuleClass::MandatoryDeny,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::PolicyChange,
            },
            effect: PolicyEffect::Block,
            reason_code: "policy_change_blocked".into(),
            summary: "Policy mutations require separate gate.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("mandatory_auth_change".into()),
            name: "Block auth changes".into(),
            enabled: true,
            priority: 200,
            class: RuleClass::MandatoryDeny,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::AuthChange,
            },
            effect: PolicyEffect::Block,
            reason_code: "auth_change_blocked".into(),
            summary: "Auth changes require separate flow.".into(),
        },

        // ── BuiltinDefault: reads ──

        PolicyRule {
            id: PolicyRuleId("allow_read".into()),
            name: "Allow read-only tools".into(),
            enabled: true,
            priority: 100,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Read,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "read_only_tool".into(),
            summary: "Read-only tool call allowed.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("allow_search".into()),
            name: "Allow search tools".into(),
            enabled: true,
            priority: 100,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Search,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Low,
                confirmation: ConfirmationLevel::Auto,
            },
            reason_code: "search_tool".into(),
            summary: "Search tool call allowed.".into(),
        },

        // ── BuiltinDefault: mutation effects ──

        PolicyRule {
            id: PolicyRuleId("confirm_write".into()),
            name: "Write requires approval".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Write,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Medium,
                confirmation: ConfirmationLevel::Approve,
            },
            reason_code: "write_tool".into(),
            summary: "Write tool call requires approval.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_delete".into()),
            name: "Delete requires escalation".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Delete,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::High,
                confirmation: ConfirmationLevel::Escalate,
            },
            reason_code: "delete_tool".into(),
            summary: "Delete tool call requires escalation.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_execute".into()),
            name: "Execute requires escalation".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Execute,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Critical,
                confirmation: ConfirmationLevel::Escalate,
            },
            reason_code: "execute_tool".into(),
            summary: "Execute tool call requires escalation.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_network".into()),
            name: "Network requires approval".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Network,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Medium,
                confirmation: ConfirmationLevel::Approve,
            },
            reason_code: "network_tool".into(),
            summary: "Network tool call requires approval.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_git".into()),
            name: "Git requires escalation".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::Git,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::High,
                confirmation: ConfirmationLevel::Escalate,
            },
            reason_code: "git_tool".into(),
            summary: "Git mutation requires escalation.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_dependency_change".into()),
            name: "Dependency change requires escalation".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::DependencyChange,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Critical,
                confirmation: ConfirmationLevel::Escalate,
            },
            reason_code: "dependency_change_tool".into(),
            summary: "Dependency change requires escalation.".into(),
        },

        PolicyRule {
            id: PolicyRuleId("confirm_persistence_change".into()),
            name: "Persistence change requires escalation".into(),
            enabled: true,
            priority: 90,
            class: RuleClass::BuiltinDefault,
            matcher: ToolMatcher::ToolEffect {
                effect: ToolEffect::PersistenceChange,
            },
            effect: PolicyEffect::Allow {
                risk: RiskLevelSnapshot::Critical,
                confirmation: ConfirmationLevel::Escalate,
            },
            reason_code: "persistence_change_tool".into(),
            summary: "Persistence change requires escalation.".into(),
        },
    ]
}
