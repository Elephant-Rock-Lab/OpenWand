//! OpenWand Desktop UI — Reusable Visual Components.
//!
//! Pure style builders and Dioxus render wrappers for common visual patterns:
//! cards, status dots, badges, warning banners, error panels, monospace blocks,
//! section headers, and labeled rows.
//!
//! Style builders are always compiled and testable.
//! Dioxus render functions are gated behind `#[cfg(feature = "desktop")]`.
//!
//! This module imports no backend crates, performs no persistence, executes
//! no tools, verifies nothing, certifies nothing, mutates no trace or memory
//! state, resolves no approvals, routes no actions, and creates no workflow
//! records.

use crate::ui::design_tokens::*;

// ── Pure style builders ────────────────────────────────────────────────────

/// Style string for a bordered card container.
pub fn card_style(tone: UiTone, density: UiDensity) -> String {
    format!(
        "border: 1px solid {}; border-radius: {}; padding: {}; margin: {} 0;",
        tone.border(),
        radius::MD,
        density.padding(),
        spacing::SPACE_MD,
    )
}

/// Style string for a status indicator dot.
pub fn status_dot_style(tone: UiTone, size: UiSize) -> String {
    let d = size.dot_size();
    format!(
        "width: {d}; height: {d}; background: {}; border-radius: 50%;",
        tone.dot(),
    )
}

/// Style string for a badge / pill label.
pub fn badge_style(tone: UiTone) -> String {
    format!(
        "padding: {} {}; background: {}; border: 1px solid {}; \
         border-radius: {}; font-size: {}; font-weight: 600; color: {}; display: inline-block;",
        spacing::SPACE_SM,
        spacing::SPACE_LG,
        tone.bg(),
        tone.border(),
        radius::PILL,
        typo::TEXT_SM,
        tone.text(),
    )
}

/// Style string for a warning/error banner.
pub fn banner_style(tone: UiTone) -> String {
    format!(
        "background: {}; border: 1px solid {}; border-radius: {}; \
         padding: {}; margin: {} 0; font-size: {}; color: {};",
        tone.bg(),
        tone.border(),
        radius::MD,
        spacing::SPACE_LG,
        spacing::SPACE_MD,
        typo::TEXT_BASE,
        tone.text(),
    )
}

/// Style string for a monospace code block with scrolling.
pub fn monospace_block_style() -> String {
    format!(
        "font-family: {}; font-size: {}; white-space: pre-wrap; \
         max-height: 80px; overflow-y: auto;",
        typo::FONT_MONO,
        typo::TEXT_SM,
    )
}

/// Style string for a section header with optional count.
pub fn section_header_style(tone: UiTone) -> String {
    format!(
        "padding: {} {} {}; font-size: {}; font-weight: 600; color: {};",
        spacing::SPACE_MD,
        spacing::SPACE_XL,
        spacing::SPACE_SM,
        typo::TEXT_SM,
        tone.text(),
    )
}

/// Style string for a row in a list with a colored indicator.
pub fn labeled_row_style() -> String {
    format!(
        "display: flex; gap: {}; padding: {} 0; font-size: {}; font-family: {};",
        spacing::SPACE_MD,
        spacing::SPACE_SM,
        typo::TEXT_BASE,
        typo::FONT_MONO,
    )
}

/// Style string for a row item label.
pub fn label_style(color: &str) -> String {
    format!("min-width: 100px; color: {color};")
}

/// Style string for a row item value.
pub fn value_style() -> String {
    format!("color: {};", colors::TEXT_SECONDARY)
}

/// Style string for a message bubble (assistant or tool output).
pub fn message_bubble_style(tone: UiTone) -> String {
    format!(
        "margin-bottom: 8px; padding: 10px 14px; background: {}; \
         border: 1px solid {}; border-radius: {};",
        tone.bg(),
        tone.border(),
        radius::LG,
    )
}

/// Style string for the message role label.
pub fn message_role_style() -> String {
    format!(
        "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
        typo::TEXT_SM,
        colors::TEXT_MUTED,
        spacing::SPACE_SM,
    )
}

/// Style string for the message body text.
pub fn message_body_style() -> String {
    format!(
        "font-size: {}; color: {};",
        typo::TEXT_MD,
        colors::TEXT_PRIMARY,
    )
}

/// Style string for the tool output area inside a tool event.
pub fn tool_output_style() -> String {
    format!(
        "font-size: {}; color: #777; margin-top: {}; \
         max-height: 80px; overflow-y: auto; white-space: pre-wrap;",
        typo::TEXT_SM,
        spacing::SPACE_SM,
    )
}

/// Style string for a cursor blink during streaming.
pub fn streaming_cursor_style() -> String {
    format!("color: {};", colors::PRIMARY)
}

// ── Desktop-gated Dioxus wrappers ──────────────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use dioxus::prelude::*;

    /// Bordered card container.
    pub fn card(tone: UiTone, density: UiDensity, children: Element) -> Element {
        let style = super::card_style(tone, density);
        rsx! {
            div { style: "{style}",
                {children}
            }
        }
    }

    /// Status indicator dot.
    pub fn status_dot(tone: UiTone, size: UiSize) -> Element {
        let style = super::status_dot_style(tone, size);
        rsx! {
            div { style: "{style}" }
        }
    }

    /// Colored pill badge.
    pub fn badge(label: &str, tone: UiTone) -> Element {
        let style = super::badge_style(tone);
        rsx! {
            span { style: "{style}", "{label}" }
        }
    }

    /// Warning/error/info banner.
    pub fn banner(tone: UiTone, text: &str) -> Element {
        let style = super::banner_style(tone);
        rsx! {
            div { style: "{style}", "{text}" }
        }
    }

    /// Scrollable monospace block.
    pub fn monospace_block(content: &str) -> Element {
        let style = super::monospace_block_style();
        rsx! {
            div { style: "{style}", "{content}" }
        }
    }

    /// Section header with optional count.
    pub fn section_header(title: &str, tone: UiTone, count: Option<usize>) -> Element {
        let style = super::section_header_style(tone);
        let label = match count {
            Some(n) => format!("{} ({})", title, n),
            None => title.into(),
        };
        rsx! {
            div { style: "{style}", "{label}" }
        }
    }

    /// Labeled row with colored indicator text.
    pub fn labeled_row(label: &str, value: &str, label_color: &str) -> Element {
        let row_style = super::labeled_row_style();
        let label_s = super::label_style(label_color);
        let value_s = super::value_style();
        rsx! {
            div { style: "{row_style}",
                span { style: "{label_s}", "{label}" }
                span { style: "{value_s}", "{value}" }
            }
        }
    }

    /// Message bubble with role label and body.
    pub fn message_bubble(tone: UiTone, role: &str, body: &str, streaming: bool) -> Element {
        let bubble_style = super::message_bubble_style(tone);
        let role_style = super::message_role_style();
        let body_style = super::message_body_style();
        let cursor_style = super::streaming_cursor_style();
        rsx! {
            div { style: "{bubble_style}",
                div { style: "{role_style}", "{role}" }
                div { style: "{body_style}",
                    "{body}"
                    if streaming {
                        span { style: "{cursor_style}", "▍" }
                    }
                }
            }
        }
    }
}

// Re-export desktop components when feature is active.
#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_style_contains_border_radius_padding() {
        let s = card_style(UiTone::Neutral, UiDensity::Comfortable);
        assert!(s.contains("border:"));
        assert!(s.contains(radius::MD));
        assert!(s.contains("padding:"));
    }

    #[test]
    fn card_style_compact_has_smaller_padding() {
        let compact = card_style(UiTone::Success, UiDensity::Compact);
        let comfortable = card_style(UiTone::Success, UiDensity::Comfortable);
        assert_ne!(compact, comfortable);
    }

    #[test]
    fn status_dot_style_is_rounded() {
        let s = status_dot_style(UiTone::Success, UiSize::Sm);
        assert!(s.contains("border-radius: 50%"));
        assert!(s.contains(colors::SUCCESS_DOT));
    }

    #[test]
    fn badge_style_is_pill_shaped() {
        let s = badge_style(UiTone::Primary);
        assert!(s.contains(radius::PILL));
        assert!(s.contains("font-weight: 600"));
    }

    #[test]
    fn banner_style_warning_has_warning_colors() {
        let s = banner_style(UiTone::Warning);
        assert!(s.contains(colors::WARNING_BG));
        assert!(s.contains(colors::WARNING_BORDER));
        assert!(s.contains(colors::WARNING_TEXT));
    }

    #[test]
    fn banner_style_error_has_error_colors() {
        let s = banner_style(UiTone::Error);
        assert!(s.contains(colors::ERROR_BG));
        assert!(s.contains(colors::ERROR_BORDER));
        assert!(s.contains(colors::ERROR_TEXT));
    }

    #[test]
    fn monospace_block_style_has_mono_font() {
        let s = monospace_block_style();
        assert!(s.contains(typo::FONT_MONO));
        assert!(s.contains("overflow-y: auto"));
    }

    #[test]
    fn section_header_style_has_sm_font() {
        let s = section_header_style(UiTone::Success);
        assert!(s.contains(typo::TEXT_SM));
        assert!(s.contains("font-weight: 600"));
    }

    #[test]
    fn labeled_row_style_has_flex_and_gap() {
        let s = labeled_row_style();
        assert!(s.contains("display: flex"));
        assert!(s.contains("gap:"));
    }

    #[test]
    fn message_bubble_style_has_lg_radius() {
        let s = message_bubble_style(UiTone::Neutral);
        assert!(s.contains(radius::LG));
        assert!(s.contains("margin-bottom:"));
    }

    #[test]
    fn message_role_style_has_muted_color() {
        let s = message_role_style();
        assert!(s.contains(colors::TEXT_MUTED));
    }

    #[test]
    fn streaming_cursor_style_has_primary_color() {
        let s = streaming_cursor_style();
        assert!(s.contains(colors::PRIMARY));
    }

    #[test]
    fn all_tones_produce_valid_card_styles() {
        for tone in [
            UiTone::Neutral,
            UiTone::Info,
            UiTone::Success,
            UiTone::Warning,
            UiTone::Error,
            UiTone::Primary,
            UiTone::Danger,
        ] {
            let s = card_style(tone, UiDensity::Comfortable);
            assert!(!s.is_empty(), "card_style({:?}) empty", tone);
            assert!(s.contains("border:"), "card_style({:?}) missing border", tone);
        }
    }

    #[test]
    fn component_styles_contain_no_authority_verbs() {
        let styles = [
            card_style(UiTone::Neutral, UiDensity::Comfortable),
            status_dot_style(UiTone::Success, UiSize::Sm),
            badge_style(UiTone::Primary),
            banner_style(UiTone::Warning),
            monospace_block_style(),
            section_header_style(UiTone::Info),
            labeled_row_style(),
            message_bubble_style(UiTone::Neutral),
            message_role_style(),
            streaming_cursor_style(),
        ];
        for s in &styles {
            let lower = s.to_lowercase();
            assert!(!lower.contains("execute"), "contains 'execute': {s}");
            assert!(!lower.contains("verify"), "contains 'verify': {s}");
            assert!(!lower.contains("certify"), "contains 'certify': {s}");
            assert!(!lower.contains("reconcile"), "contains 'reconcile': {s}");
            assert!(!lower.contains("approve"), "contains 'approve': {s}");
        }
    }
}
