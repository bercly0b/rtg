use unicode_width::UnicodeWidthStr;

use crate::domain::local_time::LocalTime;

/// Truncates a string to fit within a given display width.
///
/// Returns `(display_text, padding)`:
/// - If the text fits, returns the original text with remaining padding.
/// - If it doesn't fit, truncates at a character boundary and appends "...".
///
/// Uses Unicode display width so that emoji and wide characters are measured
/// correctly (e.g. 🚀 counts as 2 cells, not 1).
pub(super) fn truncate_to_display_width(text: &str, max_width: usize) -> (String, usize) {
    use unicode_width::UnicodeWidthChar;

    let text_width = text.width();
    if text_width <= max_width {
        return (text.to_owned(), max_width.saturating_sub(text_width));
    }

    if max_width < 3 {
        return (String::new(), max_width);
    }

    let target_width = max_width - 3;
    let mut current_width = 0;
    let mut truncated = String::new();

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > target_width {
            break;
        }
        truncated.push(ch);
        current_width += ch_width;
    }

    (format!("{}...", truncated), 0)
}

pub(super) fn format_chat_timestamp(timestamp_ms: i64) -> String {
    let Some(time) = LocalTime::from_unix_millis(timestamp_ms) else {
        return "     ".to_owned();
    };

    if time.date() == LocalTime::now().date() {
        time.format("%H:%M")
    } else {
        time.format("%d.%m")
    }
}

pub(super) fn normalize_preview_for_chat_row(preview: &str) -> String {
    preview.split_whitespace().collect::<Vec<_>>().join(" ")
}
