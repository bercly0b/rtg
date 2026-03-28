use super::super::text_utils;

#[test]
fn truncate_to_display_width_fits_ascii() {
    let (text, padding) = text_utils::truncate_to_display_width("hello", 10);
    assert_eq!(text, "hello");
    assert_eq!(padding, 5);
}

#[test]
fn truncate_to_display_width_truncates_with_ellipsis() {
    let (text, padding) = text_utils::truncate_to_display_width("hello world", 8);
    assert_eq!(text, "hello...");
    assert_eq!(padding, 0);
}

#[test]
fn truncate_to_display_width_counts_emoji_as_double_width() {
    let (text, padding) = text_utils::truncate_to_display_width("\u{1F680} hi", 5);
    assert_eq!(text, "\u{1F680} hi");
    assert_eq!(padding, 0);
}

#[test]
fn truncate_to_display_width_truncates_emoji_correctly() {
    let (text, padding) = text_utils::truncate_to_display_width("\u{1F680}\u{1F680}\u{1F680}", 5);
    assert_eq!(text, "\u{1F680}...");
    assert_eq!(padding, 0);
}

#[test]
fn truncate_to_display_width_exact_fit() {
    let (text, padding) = text_utils::truncate_to_display_width("abc", 3);
    assert_eq!(text, "abc");
    assert_eq!(padding, 0);
}

#[test]
fn truncate_to_display_width_zero_width_returns_empty() {
    let (text, padding) = text_utils::truncate_to_display_width("hello", 0);
    assert_eq!(text, "");
    assert_eq!(padding, 0);
}

#[test]
fn truncate_to_display_width_less_than_ellipsis_returns_empty() {
    let (text, padding) = text_utils::truncate_to_display_width("hello", 2);
    assert_eq!(text, "");
    assert_eq!(padding, 2);
}

#[test]
fn format_chat_timestamp_shows_time_for_today() {
    use chrono::Local;

    let now = Local::now();
    let timestamp_ms = now.timestamp_millis();

    let formatted = text_utils::format_chat_timestamp(timestamp_ms);

    assert_eq!(formatted.len(), 5);
    assert!(formatted.contains(':'));
}

#[test]
fn format_chat_timestamp_shows_date_for_past() {
    let timestamp_ms = 1577836800000_i64;

    let formatted = text_utils::format_chat_timestamp(timestamp_ms);

    assert_eq!(formatted.len(), 5);
    assert!(formatted.contains('.'));
}

#[test]
fn format_chat_timestamp_handles_negative_timestamp_gracefully() {
    let formatted = text_utils::format_chat_timestamp(-1000);

    assert_eq!(formatted.len(), 5);
    assert!(formatted.contains('.'));
}

#[test]
fn format_chat_timestamp_handles_extreme_negative_timestamp() {
    let formatted = text_utils::format_chat_timestamp(i64::MIN);

    assert_eq!(formatted, "     ");
}
