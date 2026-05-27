//! Output normalization for tool results.

pub const MAX_TOOL_OUTPUT_CHARS: usize = 50_000;

/// Normalize raw tool output. Returns (output, was_truncated, original_char_count).
pub fn normalize_output(raw: String) -> (String, bool, Option<usize>) {
    let original_len = raw.chars().count();

    if original_len <= MAX_TOOL_OUTPUT_CHARS {
        return (raw, false, None);
    }

    let truncated: String = raw.chars().take(MAX_TOOL_OUTPUT_CHARS).collect();

    (
        format!(
            "{}\n\n[openwand: output truncated from {} chars to {} chars]",
            truncated, original_len, MAX_TOOL_OUTPUT_CHARS
        ),
        true,
        Some(original_len),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_truncation() {
        let short = "hello".to_string();
        let (output, truncated, orig) = normalize_output(short);
        assert_eq!("hello", output);
        assert!(!truncated);
        assert!(orig.is_none());

        let long: String = "x".repeat(60_000);
        let (output, truncated, orig) = normalize_output(long);
        assert!(truncated);
        assert_eq!(Some(60_000), orig);
        assert!(output.contains("[openwand: output truncated"));
        assert!(output.len() < 60_000);
    }
}
