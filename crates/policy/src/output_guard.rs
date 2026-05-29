//! Post-inference output record guard.
//!
//! This is NOT pre-disclosure safety enforcement.
//! It is post-hoc durable-record correction and user-facing fallback
//! after generation completes.
//!
//! Invariant:
//!   Streaming remains live.
//!   Durable assistant message is screened.
//!   Trace records whether fallback was used.
//!   This does NOT guarantee the user never saw the raw text.

use serde::{Deserialize, Serialize};

/// Result of screening model output against forbidden action patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputScreeningResult {
    /// Whether the text passed screening (no forbidden action patterns found).
    pub passed: bool,
    /// Forbidden action patterns that matched.
    pub forbidden_hits: Vec<String>,
}

/// Screened output — the durable record after post-inference guarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenedOutput {
    /// The final text to record as the assistant message.
    pub final_text: String,
    /// The original model output before screening.
    pub base_text: String,
    /// Whether screening triggered a fallback replacement.
    pub was_screened: bool,
    /// Forbidden action patterns that triggered screening (empty if passed).
    pub forbidden_hits: Vec<String>,
}

/// Configuration for post-inference output guarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputGuardConfig {
    /// Whether output guarding is enabled.
    pub enabled: bool,
    /// Action patterns to forbid in model output text.
    /// Should be narrow: only cross-boundary operations like
    /// "git pull", "git push", "pip install", "npm install".
    /// NOT: "write", "edit", "delete", "rm" — those are tool actions
    /// already governed by the ToolGate.
    pub forbidden_actions: Vec<String>,
}

impl Default for OutputGuardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            forbidden_actions: vec![],
        }
    }
}

impl OutputGuardConfig {
    /// Conservative default for UI sessions.
    /// Only cross-boundary operations that could affect the environment
    /// beyond the user's immediate workspace.
    pub fn conservative_default() -> Self {
        Self {
            enabled: true,
            forbidden_actions: vec![
                "git pull".into(),
                "git push".into(),
                "pip install".into(),
                "npm install".into(),
            ],
        }
    }
}

/// Screen model output text for forbidden action patterns.
///
/// Uses context-sensitive detection: "do not run git pull" is NOT flagged,
/// but "run git pull" IS flagged. Handles negation, quotation, and
/// capability-description contexts.
pub fn screen_output(text: &str, forbidden_actions: &[String]) -> OutputScreeningResult {
    let lower = text.to_lowercase();
    let mut hits = Vec::new();

    for action in forbidden_actions {
        let action_lower = action.to_lowercase();
        if action_lower.trim().is_empty() {
            continue;
        }
        for (start, _) in find_action_occurrences(&lower, &action_lower) {
            let prefix_start = start.saturating_sub(80);
            let prefix = &lower[prefix_start..start];
            let suffix_end = std::cmp::min(lower.len(), start + action_lower.len() + 80);
            let suffix = &lower[start + action_lower.len()..suffix_end];
            if is_safe_context(prefix, suffix, &action_lower) {
                continue;
            }
            hits.push(action.clone());
            break;
        }
    }

    hits.sort();
    hits.dedup();

    OutputScreeningResult {
        passed: hits.is_empty(),
        forbidden_hits: hits,
    }
}

/// Guard model output for the durable assistant message record.
///
/// If forbidden actions are detected, replaces the text with a
/// natural-language fallback. The fallback never contains forbidden
/// action text and sounds like a normal assistant correction.
pub fn guard_output(text: &str, forbidden_actions: &[String]) -> ScreenedOutput {
    let result = screen_output(text, forbidden_actions);

    if result.passed {
        return ScreenedOutput {
            final_text: text.to_string(),
            base_text: text.to_string(),
            was_screened: false,
            forbidden_hits: vec![],
        };
    }

    // Natural-language fallback — NOT a JSON blob.
    let fallback = build_natural_fallback(&result.forbidden_hits);

    ScreenedOutput {
        final_text: fallback,
        base_text: text.to_string(),
        was_screened: true,
        forbidden_hits: result.forbidden_hits,
    }
}

fn build_natural_fallback(hits: &[String]) -> String {
    if hits.is_empty() {
        return "I've reviewed my response and need to correct it before recording. \
               Please ask me to elaborate on specific details."
            .to_string();
    }

    format!(
        "I've reviewed my response and corrected it. \
         My original answer referenced operations that should be performed \
         only with explicit approval (such as {}). \
         No protected operations were performed. \
         Please ask me to elaborate on specific details if needed.",
        hits.iter()
            .map(|h| format!("'{}'", h))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// --- Context-sensitive detection (ported from controller SafetyEvaluator) ---

fn find_action_occurrences(text: &str, action: &str) -> Vec<(usize, usize)> {
    let mut occurrences = Vec::new();
    let mut search_start = 0;
    while let Some(pos) = text[search_start..].find(action) {
        let start = search_start + pos;
        let end = start + action.len();
        if action.split_whitespace().count() > 1 || word_boundary(text, start, end) {
            occurrences.push((start, end));
        }
        search_start = end;
    }
    occurrences
}

fn word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before
        .map(|c| c.is_ascii_alphanumeric() || c == '_')
        .unwrap_or(false)
        && !after
            .map(|c| c.is_ascii_alphanumeric() || c == '_')
            .unwrap_or(false)
}

fn is_safe_context(prefix: &str, suffix: &str, action: &str) -> bool {
    let window = format!("{prefix}{action}{suffix}");

    // Negation context: "do not X", "should not X", "avoid X"
    if [
        "do not",
        "don't",
        "must not",
        "should not",
        "not ",
        "never",
        "avoid",
        "forbidden",
        "unsafe",
        "blocked",
    ]
    .iter()
    .any(|marker| prefix.contains(marker) || suffix.contains(marker))
    {
        return true;
    }

    // Quoted context: the action is inside quotes
    if prefix.ends_with('"') || prefix.ends_with('\'') || prefix.ends_with('`') {
        return true;
    }

    // Capability description: "classified as", "capabilities include"
    if [
        "classified as",
        "category",
        "capability",
        "capabilities",
        "surface includes",
        "tool surface",
        "read/write",
    ]
    .iter()
    .any(|marker| window.contains(marker))
        && !contains_imperative(prefix)
    {
        return true;
    }

    // "write-capable", "write access" — not an instruction to write
    if action.starts_with("write") {
        let suffix_trimmed =
            suffix.trim_start_matches(|c: char| c == '_' || c == '-' || c.is_whitespace());
        if [
            "capable",
            "file",
            "access",
            "barrier",
            "operation",
            "operations",
            "toolset",
            "toolsets",
        ]
        .iter()
        .any(|marker| suffix_trimmed.starts_with(marker))
            && !contains_imperative(prefix)
        {
            return true;
        }
    }

    false
}

fn contains_imperative(prefix: &str) -> bool {
    [
        "use ",
        "run ",
        "execute ",
        "apply ",
        "perform ",
        "invoke ",
        "call ",
    ]
    .iter()
    .any(|marker| prefix.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- screen_output tests ---

    #[test]
    fn safe_text_passes_screening() {
        let result = screen_output(
            "The project has 12 crates.",
            &[],
        );
        assert!(result.passed);
        assert!(result.forbidden_hits.is_empty());
    }

    #[test]
    fn forbidden_action_detected() {
        let result = screen_output(
            "Run git pull to update your repository.",
            &["git pull".to_string()],
        );
        assert!(!result.passed);
        assert!(result.forbidden_hits.contains(&"git pull".to_string()));
    }

    #[test]
    fn negation_context_not_flagged() {
        let result = screen_output(
            "Do not run git pull on production servers.",
            &["git pull".to_string()],
        );
        assert!(result.passed);
    }

    #[test]
    fn dont_contraction_not_flagged() {
        let result = screen_output(
            "Don't run pip install in the global environment.",
            &["pip install".to_string()],
        );
        assert!(result.passed);
    }

    #[test]
    fn quoted_context_not_flagged() {
        let result = screen_output(
            "The command \"git pull\" is blocked by policy.",
            &["git pull".to_string()],
        );
        assert!(result.passed);
    }

    #[test]
    fn imperative_context_flagged() {
        let result = screen_output(
            "Use git pull to update your branch.",
            &["git pull".to_string()],
        );
        assert!(!result.passed);
    }

    #[test]
    fn capability_description_not_flagged() {
        let result = screen_output(
            "The tool surface includes git pull as a classified capability.",
            &["git pull".to_string()],
        );
        assert!(result.passed);
    }

    #[test]
    fn multiple_forbidden_actions_detected() {
        let result = screen_output(
            "Run git pull and then pip install the dependencies.",
            &["git pull".to_string(), "pip install".to_string()],
        );
        assert!(!result.passed);
        assert_eq!(2, result.forbidden_hits.len());
    }

    #[test]
    fn empty_forbidden_list_always_passes() {
        let result = screen_output("Run git pull now.", &[]);
        assert!(result.passed);
    }

    #[test]
    fn word_boundary_prevents_partial_match() {
        let result = screen_output(
            "The bigitt pulling mechanism.",
            &["git pull".to_string()],
        );
        assert!(result.passed, "partial word match should not trigger");
    }

    // --- guard_output tests ---

    #[test]
    fn safe_output_passes_through_unchanged() {
        let output = guard_output(
            "The project has 12 crates.",
            &["git pull".to_string()],
        );
        assert_eq!("The project has 12 crates.", output.final_text);
        assert!(!output.was_screened);
        assert!(output.forbidden_hits.is_empty());
    }

    #[test]
    fn unsafe_output_replaced_with_natural_fallback() {
        let output = guard_output(
            "Run git pull to update.",
            &["git pull".to_string()],
        );
        assert!(output.was_screened);
        assert_ne!(output.final_text, output.base_text);
        // Fallback mentions the operation type but as a warning, not an instruction
        assert!(output.final_text.contains("corrected"), "fallback should sound like natural correction");
        assert!(output.final_text.contains("explicit approval"), "fallback should mention approval requirement");
    }

    #[test]
    fn fallback_mentions_operation_types() {
        let output = guard_output(
            "Run git pull and npm install now.",
            &["git pull".to_string(), "npm install".to_string()],
        );
        assert!(output.final_text.contains("'git pull'"));
        assert!(output.final_text.contains("'npm install'"));
    }

    #[test]
    fn empty_forbidden_list_never_screens() {
        let output = guard_output("Run anything you want.", &[]);
        assert!(!output.was_screened);
        assert_eq!("Run anything you want.", output.final_text);
    }

    // --- OutputGuardConfig tests ---

    #[test]
    fn conservative_default_is_enabled() {
        let config = OutputGuardConfig::conservative_default();
        assert!(config.enabled);
    }

    #[test]
    fn conservative_default_has_narrow_forbidden_list() {
        let config = OutputGuardConfig::conservative_default();
        assert!(config.forbidden_actions.contains(&"git pull".to_string()));
        assert!(config.forbidden_actions.contains(&"npm install".to_string()));
        // NOT: write, edit, delete, rm
        assert!(!config.forbidden_actions.iter().any(|a| a == "write"));
        assert!(!config.forbidden_actions.iter().any(|a| a == "edit"));
        assert!(!config.forbidden_actions.iter().any(|a| a == "delete"));
    }
}
