//! Rule model: matchers, effects, rule classes.

use openwand_core::mode::ConfirmationLevel;
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::tool_vocab::ToolEffect;
use serde::{Deserialize, Serialize};

use crate::tool::{PolicyToolCall, PolicyToolDescriptor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: PolicyRuleId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub class: RuleClass,
    pub matcher: ToolMatcher,
    pub effect: PolicyEffect,
    pub reason_code: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyRuleId(pub String);

/// Three rule classes. MandatoryDeny cannot be weakened by config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleClass {
    /// Cannot be overridden by user/project config.
    MandatoryDeny,
    /// Can be overridden by user/project config.
    BuiltinDefault,
    /// From user config (global or project).
    UserOverride,
}

/// Compositional matching for tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolMatcher {
    /// Matches any tool call
    Any,

    /// Matches by exact tool name
    ToolName { exact: String },

    /// Matches by tool name prefix
    ToolNamePrefix { prefix: String },

    /// Matches by declared tool effect
    ToolEffect { effect: ToolEffect },

    /// Matches by tool tag
    ToolTag { tag: String },

    /// All matchers must match
    All { matchers: Vec<ToolMatcher> },

    /// At least one matcher must match
    AnyOf { matchers: Vec<ToolMatcher> },

    /// Negation
    Not { matcher: Box<ToolMatcher> },
}

impl ToolMatcher {
    pub fn matches(
        &self,
        call: &PolicyToolCall,
        descriptor: Option<&PolicyToolDescriptor>,
    ) -> bool {
        match self {
            Self::Any => true,
            Self::ToolName { exact } => call.name == *exact,
            Self::ToolNamePrefix { prefix } => call.name.starts_with(prefix),
            Self::ToolEffect { effect } => call.declared_effect == *effect,
            Self::ToolTag { tag } => descriptor
                .map(|d| d.tags.contains(tag))
                .unwrap_or(false),
            Self::All { matchers } => matchers
                .iter()
                .all(|m| m.matches(call, descriptor)),
            Self::AnyOf { matchers } => matchers
                .iter()
                .any(|m| m.matches(call, descriptor)),
            Self::Not { matcher } => !matcher.matches(call, descriptor),
        }
    }
}

/// What a matching rule does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow {
        risk: RiskLevelSnapshot,
        confirmation: ConfirmationLevel,
    },
    Block,
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::ToolCallId;

    fn test_call(name: &str, effect: ToolEffect) -> PolicyToolCall {
        PolicyToolCall {
            id: ToolCallId::new(),
            name: name.into(),
            arguments: serde_json::json!({}),
            declared_effect: effect,
        }
    }

    fn test_desc(name: &str, effect: ToolEffect, tags: &[&str]) -> PolicyToolDescriptor {
        PolicyToolDescriptor {
            name: name.into(),
            source: crate::tool::PolicyToolSource::Local,
            declared_effect: effect,
            risk_hints: vec![],
            tags: tags.iter().map(|t| (*t).into()).collect(),
        }
    }

    #[test]
    fn tool_matcher_exact_name_matches() {
        let call = test_call("read_file", ToolEffect::Read);
        let matcher = ToolMatcher::ToolName {
            exact: "read_file".into(),
        };
        assert!(matcher.matches(&call, None));

        let wrong = ToolMatcher::ToolName {
            exact: "write_file".into(),
        };
        assert!(!wrong.matches(&call, None));
    }

    #[test]
    fn tool_matcher_effect_matches() {
        let call = test_call("read_file", ToolEffect::Read);
        let matcher = ToolMatcher::ToolEffect {
            effect: ToolEffect::Read,
        };
        assert!(matcher.matches(&call, None));

        let wrong = ToolMatcher::ToolEffect {
            effect: ToolEffect::Write,
        };
        assert!(!wrong.matches(&call, None));
    }

    #[test]
    fn tool_matcher_combinators_match() {
        let call = test_call("bash", ToolEffect::Execute);
        let desc = test_desc("bash", ToolEffect::Execute, &["shell", "dangerous"]);

        // All — both must match
        let all = ToolMatcher::All {
            matchers: vec![
                ToolMatcher::ToolEffect {
                    effect: ToolEffect::Execute,
                },
                ToolMatcher::ToolTag {
                    tag: "dangerous".into(),
                },
            ],
        };
        assert!(all.matches(&call, Some(&desc)));

        // AnyOf — one suffices
        let any_of = ToolMatcher::AnyOf {
            matchers: vec![
                ToolMatcher::ToolEffect {
                    effect: ToolEffect::Read,
                },
                ToolMatcher::ToolTag {
                    tag: "shell".into(),
                },
            ],
        };
        assert!(any_of.matches(&call, Some(&desc)));

        // Not
        let not = ToolMatcher::Not {
            matcher: Box::new(ToolMatcher::ToolEffect {
                effect: ToolEffect::Read,
            }),
        };
        assert!(not.matches(&call, None));
    }
}
