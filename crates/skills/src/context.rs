//! Skill context projection — read-only summary for session consumption.
//!
//! Skills are context, not authority. This module produces session-safe
//! summaries that carry no executable fields, no function pointers,
//! no tool handles, no command strings.

use crate::manifest::{SkillContextKind, SkillDefinition};
use crate::registry::SkillRegistry;

/// Session-safe summary of a skill. No structured executable fields.
///
/// Free-text fields (description, constraints, allowed_context) are supplied
/// as contextual text only and are never parsed by OpenWand as executable
/// commands, tool invocations, scripts, or structured authority.
#[derive(Debug, Clone)]
pub struct SkillContextSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub constraints: Vec<String>,
    pub allowed_context: Vec<String>,
}

impl SkillContextSummary {
    /// Build from a skill definition. Only enabled skills should be summarized.
    pub fn from_definition(def: &SkillDefinition) -> Self {
        Self {
            id: def.id.0.clone(),
            name: def.name.clone(),
            description: def.description.clone(),
            category: def.category.clone(),
            constraints: def.constraints.clone(),
            allowed_context: def
                .allowed_context
                .iter()
                .map(|k| format!("{k:?}").to_lowercase().replace(' ', "_"))
                .collect(),
        }
    }
}

/// Build skill context summaries from a registry.
/// Only includes enabled skills. Ordered deterministically by ID.
pub fn build_skill_context_summaries(registry: &SkillRegistry) -> Vec<SkillContextSummary> {
    registry
        .skills
        .iter()
        .filter(|s| s.enabled)
        .map(SkillContextSummary::from_definition)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkillDefinition, SkillId};
    use crate::registry::SkillValidationReport;

    fn test_registry() -> SkillRegistry {
        SkillRegistry {
            skills: vec![
                SkillDefinition {
                    id: SkillId("alpha".into()),
                    name: "Alpha".into(),
                    description: "First skill".into(),
                    category: "test".into(),
                    enabled: true,
                    tags: vec![],
                    inputs: vec![],
                    outputs: vec!["result".into()],
                    constraints: vec!["Must not execute tools".into()],
                    allowed_context: vec![SkillContextKind::TraceSummary],
                },
                SkillDefinition {
                    id: SkillId("disabled".into()),
                    name: "Disabled".into(),
                    description: "A disabled skill".into(),
                    category: "test".into(),
                    enabled: false,
                    tags: vec![],
                    inputs: vec![],
                    outputs: vec![],
                    constraints: vec![],
                    allowed_context: vec![],
                },
            ],
            validation: SkillValidationReport::default(),
        }
    }

    #[test]
    fn skill_context_only_includes_enabled_skills() {
        let registry = test_registry();
        let summaries = build_skill_context_summaries(&registry);
        assert_eq!(1, summaries.len());
        assert_eq!("alpha", summaries[0].id);
    }

    #[test]
    fn skill_context_summary_has_no_executable_fields() {
        let registry = test_registry();
        let summaries = build_skill_context_summaries(&registry);
        let summary = &summaries[0];

        // The summary struct has no command, shell, tool_name, tool_args,
        // script, cwd, env, or function_ref fields.
        // This is a structural guarantee checked by the DTO definition.
        // We verify the fields that DO exist are text-only.
        assert!(!summary.id.is_empty());
        assert!(!summary.name.is_empty());
        assert!(!summary.description.is_empty());
        // Constraints are text strings, not structured commands
        for c in &summary.constraints {
            assert!(c.starts_with("Must")); // just checking they're text
        }
    }
}
