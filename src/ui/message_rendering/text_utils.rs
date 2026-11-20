//! Text utility functions for message rendering.
//!
//! Pure helpers with no ratatui dependency: wrapping, date/time formatting,
//! sender name resolution.

use crate::domain::{
    local_time::{LocalDate, LocalTime},
    message::Message,
};

/// Wraps a text line to fit within `max_width` terminal columns.
///
/// Uses character-level breaking with Unicode width awareness.
/// Returns at least one element (possibly empty string for empty input).
pub(super) fn wrap_line(text: &str, max_width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 || text.is_empty() {
        return vec![text.to_owned()];
    }

    let text_width: usize = text
        .chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    if text_width <= max_width {
        return vec![text.to_owned()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_w > max_width && !current.is_empty() {
            result.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_w;
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

pub(super) fn effective_sender_name(message: &Message) -> &str {
    if message.is_outgoing {
        "You"
    } else {
        &message.sender_name
    }
}

pub(super) fn timestamp_to_date(timestamp_ms: i64) -> LocalDate {
    LocalTime::from_unix_millis(timestamp_ms)
        .unwrap_or_else(LocalTime::now)
        .date()
}

pub(super) fn format_date(date: LocalDate) -> String {
    date.format("%-d %b %Y")
}

pub(super) fn format_time(timestamp_ms: i64) -> String {
    LocalTime::from_unix_millis(timestamp_ms)
        .map_or_else(|| "??:??".to_owned(), |time| time.format("%H:%M"))
}
