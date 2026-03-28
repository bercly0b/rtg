use crate::ui::message_rendering::reply::truncate_to_width;
use crate::ui::message_rendering::text_utils::wrap_line;

// ── wrap_line tests ──

#[test]
fn wrap_line_short_text_no_wrapping() {
    let result = wrap_line("hello", 10);
    assert_eq!(result, vec!["hello"]);
}

#[test]
fn wrap_line_exact_fit() {
    let result = wrap_line("12345", 5);
    assert_eq!(result, vec!["12345"]);
}

#[test]
fn wrap_line_splits_long_text() {
    let result = wrap_line("abcdefghij", 5);
    assert_eq!(result, vec!["abcde", "fghij"]);
}

#[test]
fn wrap_line_splits_into_three() {
    let result = wrap_line("abcdefghijklmno", 5);
    assert_eq!(result, vec!["abcde", "fghij", "klmno"]);
}

#[test]
fn wrap_line_handles_remainder() {
    let result = wrap_line("abcdefgh", 5);
    assert_eq!(result, vec!["abcde", "fgh"]);
}

#[test]
fn wrap_line_empty_text() {
    let result = wrap_line("", 10);
    assert_eq!(result, vec![""]);
}

#[test]
fn wrap_line_zero_width_returns_original() {
    let result = wrap_line("hello", 0);
    assert_eq!(result, vec!["hello"]);
}

#[test]
fn wrap_line_unicode_emoji() {
    let result = wrap_line("\u{1f680}ab", 4);
    assert_eq!(result, vec!["\u{1f680}ab"]);
}

#[test]
fn wrap_line_unicode_emoji_wraps_correctly() {
    let result = wrap_line("\u{1f680}a\u{1f680}b", 4);
    assert_eq!(result, vec!["\u{1f680}a", "\u{1f680}b"]);
}

// ── truncate_to_width tests ──

#[test]
fn truncate_short_text_unchanged() {
    assert_eq!(truncate_to_width("hello", 10), "hello");
}

#[test]
fn truncate_exact_fit_unchanged() {
    assert_eq!(truncate_to_width("hello", 5), "hello");
}

#[test]
fn truncate_long_text_adds_ellipsis() {
    let result = truncate_to_width("hello world", 8);
    assert!(
        result.ends_with('\u{2026}'),
        "Should end with ellipsis: '{}'",
        result
    );
    assert!(
        result.len() <= 10,
        "Truncated should be short: '{}'",
        result
    );
    assert!(
        result.starts_with("hello w"),
        "Should keep prefix: '{}'",
        result
    );
}

#[test]
fn truncate_zero_width_returns_empty() {
    assert_eq!(truncate_to_width("hello", 0), "");
}

#[test]
fn truncate_width_one_returns_ellipsis_for_long() {
    let result = truncate_to_width("hello", 1);
    assert_eq!(result, "\u{2026}");
}

#[test]
fn truncate_empty_text() {
    assert_eq!(truncate_to_width("", 10), "");
}

#[test]
fn truncate_unicode_text() {
    let result = truncate_to_width(
        "\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}!",
        8,
    );
    assert!(result.ends_with('\u{2026}'));
}
