//! Skill registry — validated read-only collection of available skills.
//!
//! Loads from .openwand/skills.toml. Missing files produce empty registries
//! with warnings, not errors. Skills are context, not authority.

use std::path::Path;

use crate::manifest::{SkillDefinition, SkillManifest};

/// Validated collection of skills.
#[derive(Debug, Clone)]
pub struct SkillRegistry {
    pub skills: Vec<SkillDefinition>,
    pub validation: SkillValidationReport,
}

/// Validation report for skill manifests.
#[derive(Debug, Clone, Default)]
pub struct SkillValidationReport {
    pub errors: Vec<SkillValidationIssue>,
    pub warnings: Vec<SkillValidationIssue>,
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct SkillValidationIssue {
    pub skill_id: String,
    pub message: String,
    pub severity: SkillValidationSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillValidationSeverity {
    Error,
    Warning,
}

/// Load skills from .openwand/skills.toml.
/// Returns empty registry with warning if file is missing.
pub fn load_skill_registry(path: &Path) -> SkillRegistry {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return SkillRegistry {
                skills: Vec::new(),
                validation: SkillValidationReport {
                    errors: Vec::new(),
                    warnings: vec![SkillValidationIssue {
                        skill_id: "_global".into(),
                        message: format!(
                            "Skills manifest not found at '{}' — using empty registry",
                            path.display()
                        ),
                        severity: SkillValidationSeverity::Warning,
                    }],
                },
            };
        }
    };

    let manifest: SkillManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            return SkillRegistry {
                skills: Vec::new(),
                validation: SkillValidationReport {
                    errors: vec![SkillValidationIssue {
                        skill_id: "_global".into(),
                        message: format!("Failed to parse skills manifest: {e}"),
                        severity: SkillValidationSeverity::Error,
                    }],
                    warnings: Vec::new(),
                },
            };
        }
    };

    validate_skill_manifest(manifest)
}

fn validate_skill_manifest(manifest: SkillManifest) -> SkillRegistry {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut valid_skills = Vec::new();

    for skill in manifest.skills {
        let id_str = skill.id.0.as_str();

        // ID must be non-empty
        if id_str.trim().is_empty() {
            errors.push(SkillValidationIssue {
                skill_id: "<empty>".into(),
                message: "Skill ID must not be empty".into(),
                severity: SkillValidationSeverity::Error,
            });
            continue;
        }

        // ID must be unique
        if seen_ids.contains(id_str) {
            errors.push(SkillValidationIssue {
                skill_id: id_str.into(),
                message: format!("Duplicate skill ID: '{id_str}'"),
                severity: SkillValidationSeverity::Error,
            });
            continue;
        }
        seen_ids.insert(id_str.to_string());

        // Name must be non-empty
        if skill.name.trim().is_empty() {
            errors.push(SkillValidationIssue {
                skill_id: id_str.into(),
                message: "Skill name must not be empty".into(),
                severity: SkillValidationSeverity::Error,
            });
            continue;
        }

        // Description must be non-empty
        if skill.description.trim().is_empty() {
            errors.push(SkillValidationIssue {
                skill_id: id_str.into(),
                message: "Skill description must not be empty".into(),
                severity: SkillValidationSeverity::Error,
            });
            continue;
        }

        // Enabled skill with no outputs → warning
        if skill.enabled && skill.outputs.is_empty() {
            warnings.push(SkillValidationIssue {
                skill_id: id_str.into(),
                message: "Enabled skill has no outputs defined".into(),
                severity: SkillValidationSeverity::Warning,
            });
        }

        valid_skills.push(skill);
    }

    // Sort deterministically by ID
    valid_skills.sort_by(|a, b| a.id.0.cmp(&b.id.0));

    SkillRegistry {
        skills: valid_skills,
        validation: SkillValidationReport { errors, warnings },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_skills_toml(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("skills.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        write!(file, "{content}").unwrap();
        path
    }

    #[test]
    fn skill_registry_rejects_duplicate_ids() {
        let dir = std::env::temp_dir().join("openwand_test_skills_dup");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_skills_toml(
            &dir,
            r#"
[[skill]]
id = "dup"
name = "First"
description = "First skill"

[[skill]]
id = "dup"
name = "Second"
description = "Second skill"
"#,
        );
        let registry = load_skill_registry(&path);
        assert!(!registry.validation.errors.is_empty(), "Should report duplicate ID error");
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("Duplicate")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_registry_rejects_empty_name() {
        let dir = std::env::temp_dir().join("openwand_test_skills_empty_name");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_skills_toml(
            &dir,
            r#"
[[skill]]
id = "no-name"
name = ""
description = "Has description"
"#,
        );
        let registry = load_skill_registry(&path);
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("name")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_registry_rejects_empty_id() {
        let dir = std::env::temp_dir().join("openwand_test_skills_empty_id");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_skills_toml(
            &dir,
            r#"
[[skill]]
id = ""
name = "No ID"
description = "Has description"
"#,
        );
        let registry = load_skill_registry(&path);
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("ID")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_registry_warns_enabled_skill_without_outputs() {
        let dir = std::env::temp_dir().join("openwand_test_skills_no_outputs");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_skills_toml(
            &dir,
            r#"
[[skill]]
id = "no-outputs"
name = "No Outputs"
description = "A skill with no outputs"
enabled = true
"#,
        );
        let registry = load_skill_registry(&path);
        assert!(registry.validation.warnings.iter().any(|w| w.message.contains("no outputs")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skill_registry_missing_file_returns_empty_with_warning() {
        let path = std::env::temp_dir().join("nonexistent_skills_dir_xyz/skills.toml");
        let registry = load_skill_registry(&path);
        assert!(registry.skills.is_empty());
        assert!(registry.validation.warnings.iter().any(|w| w.message.contains("not found")));
        assert!(registry.validation.errors.is_empty());
    }

    #[test]
    fn skill_registry_orders_skills_deterministically() {
        let dir = std::env::temp_dir().join("openwand_test_skills_order");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_skills_toml(
            &dir,
            r#"
[[skill]]
id = "zebra"
name = "Zebra"
description = "Z skill"

[[skill]]
id = "alpha"
name = "Alpha"
description = "A skill"
"#,
        );
        let registry = load_skill_registry(&path);
        assert_eq!(2, registry.skills.len());
        assert_eq!("alpha", registry.skills[0].id.0);
        assert_eq!("zebra", registry.skills[1].id.0);
        std::fs::remove_dir_all(&dir).ok();
    }
}
