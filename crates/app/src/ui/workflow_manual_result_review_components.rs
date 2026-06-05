//! UI manual result review components — desktop-gated render placeholder.

// Desktop UI rendering for manual result review.
// Placeholder: actual Dioxus components will be wired when the desktop UI
// gains a review workflow screen.

pub fn render_manual_result_review_placeholder() -> String {
    "Manual result review screen — desktop-gated placeholder.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_placeholder_returns_non_empty() {
        let s = render_manual_result_review_placeholder();
        assert!(!s.is_empty());
    }
}
