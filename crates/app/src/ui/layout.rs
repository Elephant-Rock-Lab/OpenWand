//! OpenWand Desktop UI — Layout Primitives.
//!
//! Pure style builders and Dioxus render wrappers for structural layout:
//! sidebars, main columns, headers, status bars, and empty states.
//!
//! Style builders are always compiled and testable.
//! Dioxus render functions are gated behind `#[cfg(feature = "desktop")]`.
//!
//! This module imports no backend crates, performs no persistence, executes
//! no tools, verifies nothing, certifies nothing, mutates no trace or memory
//! state, resolves no approvals, routes no actions, and creates no workflow
//! records.

use crate::ui::design_tokens::*;

/// Which sidebar side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarSide {
    Left,
    Right,
}

// ── Pure style builders ────────────────────────────────────────────────────

/// Style string for a sidebar panel.
pub fn sidebar_style(side: SidebarSide) -> String {
    let width = match side {
        SidebarSide::Left => layout_dims::SIDEBAR_LEFT_WIDTH,
        SidebarSide::Right => layout_dims::SIDEBAR_RIGHT_WIDTH,
    };
    let border_prop = match side {
        SidebarSide::Left => "border-right",
        SidebarSide::Right => "border-left",
    };
    format!(
        "width: {width}; min-width: {width}; background: {}; {border_prop}: 1px solid {}; \
         display: flex; flex-direction: column;",
        colors::SURFACE_SIDEBAR,
        colors::BORDER_DEFAULT,
    )
}

/// Style string for a sidebar header section.
pub fn sidebar_header_style() -> String {
    format!(
        "padding: {} {}; border-bottom: 1px solid {}; \
         display: flex; justify-content: space-between; align-items: center;",
        "12px 16px",
        spacing::SPACE_XL,
        colors::BORDER_DEFAULT,
    )
}

/// Style string for the main content column.
pub fn main_column_style() -> String {
    "flex: 1; display: flex; flex-direction: column; min-width: 0;".into()
}

/// Style string for a header bar.
pub fn header_bar_style() -> String {
    format!(
        "padding: {} {}; border-bottom: 1px solid {}; \
         display: flex; justify-content: space-between; align-items: center;",
        spacing::SPACE_LG,
        spacing::SPACE_2XL,
        colors::BORDER_LIGHT,
    )
}

/// Style string for the bottom status bar.
pub fn status_bar_style() -> String {
    format!(
        "padding: {} {}; background: {}; border-top: 1px solid {}; \
         font-size: {}; color: {};",
        spacing::SPACE_SM,
        spacing::SPACE_XL,
        colors::SURFACE_INSET,
        colors::BORDER_DEFAULT,
        typo::TEXT_SM,
        colors::TEXT_MUTED,
    )
}

/// Style string for a centered empty state message.
pub fn empty_state_style() -> String {
    format!(
        "flex: 1; display: flex; align-items: center; justify-content: center; \
         color: {}; font-size: {};",
        colors::TEXT_PLACEHOLDER,
        typo::TEXT_XL,
    )
}

/// Style string for a scrollable content area.
pub fn scroll_area_style() -> String {
    format!(
        "flex: 1; overflow-y: auto; padding: {} {}; background: {};",
        spacing::SPACE_LG,
        spacing::SPACE_2XL,
        colors::SURFACE_BASE,
    )
}

/// Style string for the input area at the bottom.
pub fn input_area_style() -> String {
    format!(
        "padding: {} {}; border-top: 1px solid {}; background: {}; \
         display: flex; gap: {}; align-items: flex-end;",
        spacing::SPACE_SM,
        spacing::SPACE_2XL,
        colors::BORDER_LIGHT,
        colors::SURFACE_RAISED,
        spacing::SPACE_MD,
    )
}

/// Style string for a section title.
pub fn section_title_style() -> String {
    format!(
        "font-weight: 600; font-size: {};",
        typo::TEXT_LG,
    )
}

/// Style string for a session list item.
pub fn session_item_style(selected: bool) -> String {
    let bg = if selected {
        colors::SURFACE_SELECTED
    } else {
        "transparent"
    };
    format!(
        "padding: {} {}; cursor: pointer; background: {}; \
         border-bottom: 1px solid {};",
        "10px 16px",
        spacing::SPACE_XL,
        bg,
        colors::BORDER_LIGHT,
    )
}

/// Style string for a button.
pub fn button_style(tone: UiTone, disabled: bool) -> String {
    let (bg, text) = if disabled {
        (colors::DISABLED_BG, "#fff")
    } else {
        (tone.border(), "#fff")
    };
    let cursor = if disabled { "not-allowed" } else { "pointer" };
    format!(
        "padding: {} {}; font-size: {}; background: {bg}; color: {text}; \
         border: none; border-radius: {}; cursor: {cursor};",
        spacing::SPACE_SM,
        spacing::SPACE_LG,
        typo::TEXT_MD,
        radius::SM,
    )
}

/// Style string for a textarea input.
pub fn textarea_style(disabled: bool) -> String {
    let cursor = if disabled { "not-allowed" } else { "auto" };
    format!(
        "flex: 1; padding: {} {}; font-size: {}; border: 1px solid {}; \
         border-radius: {}; resize: none; font-family: {}; \
         min-height: 36px; max-height: 120px; cursor: {cursor};",
        spacing::SPACE_MD,
        spacing::SPACE_LG,
        typo::TEXT_MD,
        colors::BORDER_DEFAULT,
        radius::MD,
        typo::FONT_BODY,
    )
}

// ── Desktop-gated Dioxus wrappers ──────────────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    

    

    // Desktop render functions will be wired in future waves that
    // migrate ui_main.rs to use layout primitives.
}
// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_left_style_has_correct_width() {
        let s = sidebar_style(SidebarSide::Left);
        assert!(s.contains("260px"), "left sidebar width: {s}");
        assert!(s.contains("border-right"), "left sidebar border: {s}");
        assert!(s.contains(colors::SURFACE_SIDEBAR), "sidebar bg: {s}");
    }

    #[test]
    fn sidebar_right_style_has_correct_width() {
        let s = sidebar_style(SidebarSide::Right);
        assert!(s.contains("240px"), "right sidebar width: {s}");
        assert!(s.contains("border-left"), "right sidebar border: {s}");
    }

    #[test]
    fn main_column_style_is_flex_column() {
        let s = main_column_style();
        assert!(s.contains("flex: 1"));
        assert!(s.contains("min-width: 0"));
    }

    #[test]
    fn status_bar_style_contains_layout_tokens() {
        let s = status_bar_style();
        assert!(s.contains(colors::SURFACE_INSET));
        assert!(s.contains(typo::TEXT_SM));
        assert!(s.contains(colors::TEXT_MUTED));
    }

    #[test]
    fn empty_state_style_is_centered() {
        let s = empty_state_style();
        assert!(s.contains("align-items: center"));
        assert!(s.contains("justify-content: center"));
        assert!(s.contains(colors::TEXT_PLACEHOLDER));
    }

    #[test]
    fn scroll_area_style_has_overflow() {
        let s = scroll_area_style();
        assert!(s.contains("overflow-y: auto"));
        assert!(s.contains(colors::SURFACE_BASE));
    }

    #[test]
    fn input_area_style_has_gap() {
        let s = input_area_style();
        assert!(s.contains("gap:"));
        assert!(s.contains(colors::SURFACE_RAISED));
    }

    #[test]
    fn session_item_style_selected_vs_unselected() {
        let sel = session_item_style(true);
        let unsel = session_item_style(false);
        assert!(sel.contains(colors::SURFACE_SELECTED));
        assert!(unsel.contains("transparent"));
    }

    #[test]
    fn button_style_disabled_has_not_allowed() {
        let s = button_style(UiTone::Primary, true);
        assert!(s.contains("not-allowed"));
        assert!(s.contains(colors::DISABLED_BG));
    }

    #[test]
    fn button_style_primary_has_primary_color() {
        let s = button_style(UiTone::Primary, false);
        assert!(s.contains(colors::PRIMARY));
        assert!(s.contains("pointer"));
    }

    #[test]
    fn textarea_style_contains_font_family() {
        let s = textarea_style(false);
        assert!(s.contains(typo::FONT_BODY));
        assert!(s.contains(typo::TEXT_MD));
    }

    #[test]
    fn layout_builders_contain_no_authority_verbs() {
        let styles = [
            sidebar_style(SidebarSide::Left),
            sidebar_style(SidebarSide::Right),
            main_column_style(),
            header_bar_style(),
            status_bar_style(),
            empty_state_style(),
            scroll_area_style(),
            input_area_style(),
            session_item_style(true),
            button_style(UiTone::Primary, false),
            textarea_style(false),
        ];
        for s in &styles {
            let lower = s.to_lowercase();
            assert!(!lower.contains("execute"), "style contains 'execute': {s}");
            assert!(!lower.contains("verify"), "style contains 'verify': {s}");
            assert!(!lower.contains("certify"), "style contains 'certify': {s}");
            assert!(!lower.contains("reconcile"), "style contains 'reconcile': {s}");
        }
    }
}
