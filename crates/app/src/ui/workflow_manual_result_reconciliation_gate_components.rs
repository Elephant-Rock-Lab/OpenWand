//! UI manual reconciliation gate components — desktop-gated render placeholder.

pub fn render_manual_reconciliation_gate_placeholder() -> String {
    "Manual reconciliation gate screen — desktop-gated placeholder.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_placeholder_returns_non_empty() {
        let s = render_manual_reconciliation_gate_placeholder();
        assert!(!s.is_empty());
    }
}
