//! Skill manifest — declarative skill metadata loaded from .openwand/skills.toml.
//!
//! Skills describe reusable capabilities. They are context, not authority.
//! No executable fields: no command, shell, tool_name, tool_args, script,
//! cwd, env, or function_ref.

use serde::{Deserialize, Serialize};

/// Unique identifier for a skill.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Top-level TOML structure for skill manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    #[serde(rename = "skill")]
    pub skills: Vec<SkillDefinition>,
}

/// A single skill definition.
///
/// Free-text fields (description, constraints, inputs, outputs, tags)
/// are supplied as contextual text only and are never parsed by OpenWand
/// as executable commands, tool invocations, scripts, or structured authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub allowed_context: Vec<SkillContextKind>,
}

fn default_true() -> bool {
    true
}

/// Kinds of context a skill may reference.
/// Uses snake_case in TOML. Unknown values are validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillContextKind {
    TraceSummary,
    MemorySummary,
    FileDiffSummary,
    TestOutputSummary,
    GovernanceSummary,
    UserInstruction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_manifest_roundtrips() {
        let manifest = SkillManifest {
            skills: vec![SkillDefinition {
                id: SkillId("rust-test-triage".into()),
                name: "Rust Test Triage".into(),
                description: "Helps interpret failing Rust test output.".into(),
                category: "engineering".into(),
                enabled: true,
                tags: vec!["rust".into(), "tests".into()],
                inputs: vec!["test output".into()],
                outputs: vec!["failure summary".into()],
                constraints: vec!["Must not run commands directly".into()],
                allowed_context: vec![SkillContextKind::TraceSummary, SkillContextKind::TestOutputSummary],
            }],
        };

        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        let restored: SkillManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(1, restored.skills.len());
        assert_eq!("rust-test-triage", restored.skills[0].id.0);
        assert_eq!("Rust Test Triage", restored.skills[0].name);
        assert_eq!(2, restored.skills[0].allowed_context.len());
    }

    #[test]
    fn skill_manifest_loads_from_openwand_skills_toml() {
        let dir = std::env::temp_dir().join("openwand_test_skills_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("skills.toml");

        std::fs::write(
            &path,
            r#"
[[skill]]
id = "rust-test-triage"
name = "Rust Test Triage"
description = "Helps interpret failing Rust test output."
category = "engineering"
enabled = true
tags = ["rust", "tests"]
inputs = ["test output"]
outputs = ["failure summary"]
constraints = ["Must not run commands directly"]
allowed_context = ["trace_summary", "test_output_summary"]
"#,
        )
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let manifest: SkillManifest = toml::from_str(&content).unwrap();
        assert_eq!(1, manifest.skills.len());
        assert_eq!("rust-test-triage", manifest.skills[0].id.0);
        assert_eq!(2, manifest.skills[0].allowed_context.len());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_context_kind_parses_snake_case_values() {
        let kinds: Vec<SkillContextKind> = serde_json::from_str(
            r#"["trace_summary", "memory_summary", "file_diff_summary", "test_output_summary", "governance_summary", "user_instruction"]"#,
        )
        .unwrap();
        assert_eq!(6, kinds.len());
        assert_eq!(SkillContextKind::TraceSummary, kinds[0]);
        assert_eq!(SkillContextKind::UserInstruction, kinds[5]);
    }

    #[test]
    fn skill_definition_has_no_executable_fields() {
        // Structurally verify: SkillDefinition has no command, shell, tool_name,
        // tool_args, script, cwd, env, or function_ref fields.
        // This is a compile-time assertion — if someone adds such a field,
        // this test documents the intent and the source guard will catch it.
        let def = SkillDefinition {
            id: SkillId("test".into()),
            name: "Test".into(),
            description: "Test skill".into(),
            category: "test".into(),
            enabled: true,
            tags: vec![],
            inputs: vec![],
            outputs: vec![],
            constraints: vec![],
            allowed_context: vec![],
        };
        // Verify the only fields are the safe ones
        assert!(!def.name.is_empty());
    }
}
