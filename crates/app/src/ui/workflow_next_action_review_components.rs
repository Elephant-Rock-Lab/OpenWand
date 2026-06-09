//! UI next-action review components — desktop-gated placeholder.
//!
//! PLACEHOLDER — Desktop UI rendering will be implemented when the design system
//! is established (see POSTPONE-04). This file exists to maintain module structure.
//! Backend: complete (CLI + persistence + validation + guard tests).
//!
//! Coverage gap closure (Wave 50A, FIX-05, KNOWN_GAPS gap 4).

pub fn render_next_action_review_placeholder() -> String {
    "Next-action review screen — desktop-gated placeholder.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder_is_non_empty() {
        assert!(!render_next_action_review_placeholder().is_empty());
    }
}
