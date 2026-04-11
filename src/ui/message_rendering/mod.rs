//! Message list rendering logic.
//!
//! Handles visual formatting of messages including:
//! - Multi-line message display (time + sender on first line, text on second)
//! - Sender grouping (consecutive messages from same sender show name only once)
//! - Date separators between messages from different days
//! - Media type indicators

mod content_spans;
mod forward;
mod line_builder;
mod reply;
mod text_utils;

use ratatui::{
    layout::Alignment,
    text::{Line, Span},
};

use crate::domain::message::{ForwardInfo, Message, MessageStatus, ReplyInfo, TextLink};

use super::styles;

use line_builder::build_message_lines;
use text_utils::{effective_sender_name, format_date, format_time, timestamp_to_date};

/// Represents a visual element in the messages list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageListElement {
    /// Date separator line (e.g., "——— 14 Feb 2026 ———").
    DateSeparator(String),
    /// A message with optional sender display.
    Message {
        time: String,
        show_time: bool,
        sender: Option<String>,
        is_outgoing: bool,
        content: String,
        status: MessageStatus,
        /// File metadata line (e.g. "download=yes, size=15.5KB, duration=0:03").
        file_meta: Option<String>,
        /// Reply preview: sender name and text of the replied-to message.
        reply_info: Option<ReplyInfo>,
        forward_info: Option<ForwardInfo>,
        /// Total number of reactions on this message.
        reaction_count: u32,
        /// Hyperlinks embedded in the message text (byte offsets into `Message::text`).
        links: Vec<TextLink>,
        /// Whether the message has been edited.
        is_edited: bool,
    },
}

/// Builds a list of visual elements from messages.
///
/// Groups consecutive messages from the same sender and inserts date separators.
pub fn build_message_list_elements(messages: &[Message]) -> Vec<MessageListElement> {
    let mut elements = Vec::new();
    let mut prev_date: Option<chrono::NaiveDate> = None;
    let mut prev_sender: Option<&str> = None;
    let mut prev_time: Option<String> = None;

    for message in messages {
        let msg_date = timestamp_to_date(message.timestamp_ms);

        // Insert date separator if date changed
        if prev_date != Some(msg_date) {
            elements.push(MessageListElement::DateSeparator(format_date(msg_date)));
            prev_sender = None; // Reset sender grouping on date change
            prev_time = None;
        }

        let sender_name = effective_sender_name(message);
        let time = format_time(message.timestamp_ms);

        // Show sender only if different from previous message
        let show_sender = prev_sender != Some(sender_name);
        let sender = if show_sender {
            Some(sender_name.to_owned())
        } else {
            None
        };

        // Show time only on the first message in a same-sender group,
        // or when HH:MM changes within the group.
        let show_time = show_sender || prev_time.as_deref() != Some(&time);

        let file_meta = if let Some(ci) = &message.call_info {
            Some(crate::domain::message::build_call_metadata_display(
                ci,
                message.is_outgoing,
            ))
        } else {
            message
                .file_info
                .as_ref()
                .map(|fi| crate::domain::message::build_file_metadata_display(message.media, fi))
        };

        elements.push(MessageListElement::Message {
            time: time.clone(),
            show_time,
            sender,
            is_outgoing: message.is_outgoing,
            content: message.display_content(),
            status: message.status,
            file_meta,
            reply_info: message.reply_to.clone(),
            forward_info: message.forward_info.clone(),
            reaction_count: message.reaction_count,
            links: message.links.clone(),
            is_edited: message.is_edited,
        });

        prev_date = Some(msg_date);
        prev_sender = Some(sender_name);
        prev_time = Some(time);
    }

    elements
}

/// Converts a message index to the corresponding element index in the list.
///
/// Since the element list contains both messages and date separators,
/// this function finds the element index for a given message index.
/// Returns `None` if the message index is out of range.
pub fn message_index_to_element_index(
    elements: &[MessageListElement],
    message_index: usize,
) -> Option<usize> {
    let mut msg_count = 0;

    for (elem_idx, element) in elements.iter().enumerate() {
        if matches!(element, MessageListElement::Message { .. }) {
            if msg_count == message_index {
                return Some(elem_idx);
            }
            msg_count += 1;
        }
    }

    None
}

/// Converts a list element to `Text` for the custom `ChatMessageList` widget.
///
/// `max_width` is the available width in terminal columns for wrapping long lines.
/// Pass `0` to disable wrapping.
pub fn element_to_text(
    element: &MessageListElement,
    max_width: usize,
) -> ratatui::text::Text<'static> {
    match element {
        MessageListElement::DateSeparator(date) => {
            let separator = format!("——— {} ———", date);
            let line = Line::from(vec![Span::styled(
                separator,
                styles::date_separator_style(),
            )])
            .alignment(Alignment::Center);
            ratatui::text::Text::from(vec![Line::default(), line, Line::default()])
        }
        MessageListElement::Message {
            time,
            show_time,
            sender,
            is_outgoing,
            content,
            status,
            file_meta,
            reply_info,
            forward_info,
            reaction_count,
            links,
            is_edited,
        } => {
            let lines = build_message_lines(
                time,
                *show_time,
                sender.as_deref(),
                *is_outgoing,
                content,
                *status,
                file_meta.as_deref(),
                reply_info.as_ref(),
                forward_info.as_ref(),
                *reaction_count,
                links,
                max_width,
                *is_edited,
            );
            ratatui::text::Text::from(lines)
        }
    }
}

#[cfg(test)]
mod tests;
