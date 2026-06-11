//! Goal manifest — declarative goal metadata loaded from .openwand/goals.toml.
//!
//! Goals describe intended outcomes. They are context, not authority.
//! No executable fields: no command, shell, tool_name, tool_args, script,
//! cwd, env, or function_ref.

use serde::{Deserialize, Serialize};

/// Unique identifier for a goal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalId(pub String);

impl std::fmt::Display for GoalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Goal status. Uses snake_case in TOML.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GoalStatus {
    #[default]
    Active,
    Paused,
    Completed,
    Archived,
}


/// Top-level TOML structure for goal manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalManifest {
    #[serde(rename = "goal")]
    pub goals: Vec<GoalDefinition>,
}

/// A single goal definition.
///
/// Free-text fields (description, constraints, success_criteria, tags)
/// are supplied as contextual text only and are never parsed by OpenWand
/// as executable commands, tool invocations, scripts, or structured authority.
///
/// linked_skills stores skill IDs as unresolved strings. Cross-registry
/// validation happens in openwand-app, not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDefinition {
    pub id: GoalId,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub status: GoalStatus,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub linked_skills: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_manifest_roundtrips() {
        let manifest = GoalManifest {
            goals: vec![GoalDefinition {
                id: GoalId("ship-governed-agent".into()),
                title: "Ship a governed agent product".into(),
                description: "Turn OpenWand into a usable governed agent.".into(),
                status: GoalStatus::Active,
                priority: 100,
                tags: vec!["product".into(), "governance".into()],
                success_criteria: vec![
                    "User can run a session from UI".into(),
                    "User can approve/reject governed actions".into(),
                ],
                constraints: vec!["Do not bypass policy gates".into()],
                linked_skills: vec!["rust-test-triage".into()],
            }],
        };

        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        let restored: GoalManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(1, restored.goals.len());
        assert_eq!("ship-governed-agent", restored.goals[0].id.0);
        assert_eq!(GoalStatus::Active, restored.goals[0].status);
        assert_eq!(100, restored.goals[0].priority);
        assert_eq!(1, restored.goals[0].linked_skills.len());
    }

    #[test]
    fn goal_manifest_loads_from_openwand_goals_toml() {
        let dir = std::env::temp_dir().join("openwand_test_goals_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("goals.toml");

        std::fs::write(
            &path,
            r#"
[[goal]]
id = "ship-governed-agent"
title = "Ship a governed agent product"
description = "Turn OpenWand into a usable governed agent."
status = "active"
priority = 100
tags = ["product", "governance"]
success_criteria = ["User can run a session from UI"]
constraints = ["Do not bypass policy gates"]
linked_skills = ["rust-test-triage"]
"#,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let manifest: GoalManifest = toml::from_str(&content).unwrap();
        assert_eq!(1, manifest.goals.len());
        assert_eq!("ship-governed-agent", manifest.goals[0].id.0);
        assert_eq!(GoalStatus::Active, manifest.goals[0].status);
        assert_eq!(&["rust-test-triage"], &manifest.goals[0].linked_skills[..]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_definition_has_no_executable_fields() {
        // Structurally verify: GoalDefinition has no command, shell, tool_name,
        // tool_args, script, cwd, env, or function_ref fields.
        let def = GoalDefinition {
            id: GoalId("test".into()),
            title: "Test".into(),
            description: "Test goal".into(),
            status: GoalStatus::Active,
            priority: 50,
            tags: vec![],
            success_criteria: vec![],
            constraints: vec![],
            linked_skills: vec![],
        };
        assert!(!def.title.is_empty());
    }
}
