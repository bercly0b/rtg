use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::message_info_state::{
    format_unix_timestamp, MessageInfo, MessageInfoPopupState,
};

use super::{popup_utils, styles};

pub fn render_message_info_popup(frame: &mut Frame<'_>, area: Rect, state: &MessageInfoPopupState) {
    let popup_area = popup_utils::centered_rect(area, 50, 60);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Message Info ")
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_info_lines(state);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn build_info_lines(state: &MessageInfoPopupState) -> Vec<Line<'static>> {
    match state {
        MessageInfoPopupState::Loading { .. } => {
            vec![Line::from(Span::styled(
                "Loading...",
                styles::chat_info_popup_value_style(),
            ))]
        }
        MessageInfoPopupState::Error => {
            vec![Line::from(Span::styled(
                "Failed to load message info",
                styles::command_popup_error_style(),
            ))]
        }
        MessageInfoPopupState::Loaded(info) => build_loaded_lines(info),
    }
}

fn build_loaded_lines(info: &MessageInfo) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut has_content = false;

    if !info.reactions.is_empty() {
        lines.push(Line::from(Span::styled(
            "Reactions",
            styles::chat_info_popup_label_style(),
        )));
        for r in &info.reactions {
            lines.push(Line::from(Span::styled(
                format!("  {} — {}", r.emoji, r.sender_name),
                styles::chat_info_popup_value_style(),
            )));
        }
        has_content = true;
    }

    if !info.viewers.is_empty() {
        if has_content {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "Read by",
            styles::chat_info_popup_label_style(),
        )));
        for v in &info.viewers {
            let date = format_unix_timestamp(v.view_date);
            lines.push(Line::from(Span::styled(
                format!("  {} — {}", v.name, date),
                styles::chat_info_popup_value_style(),
            )));
        }
        has_content = true;
    }

    if let Some(read_date) = info.read_date {
        if has_content {
            lines.push(Line::from(""));
        }
        let date = format_unix_timestamp(read_date);
        lines.push(build_field_line("Read at", &date));
        has_content = true;
    }

    if let Some(edit_date) = info.edit_date {
        if has_content {
            lines.push(Line::from(""));
        }
        let date = format_unix_timestamp(edit_date);
        lines.push(build_field_line("Edited", &date));
        has_content = true;
    }

    if !has_content {
        lines.push(Line::from(Span::styled(
            "No additional info available",
            styles::chat_info_popup_value_style(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press q, Esc or I to close",
        styles::help_popup_footer_style(),
    )));

    lines
}

fn build_field_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), styles::chat_info_popup_label_style()),
        Span::styled(value.to_owned(), styles::chat_info_popup_value_style()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message_info_state::{ReactionDetail, ViewerDetail};

    #[test]
    fn loading_state_shows_loading_text() {
        let state = MessageInfoPopupState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        let lines = build_info_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Loading"));
    }

    #[test]
    fn error_state_shows_error_text() {
        let state = MessageInfoPopupState::Error;
        let lines = build_info_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Failed"));
    }

    #[test]
    fn empty_info_shows_no_info_message() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("No additional info"));
    }

    #[test]
    fn loaded_with_reactions_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![ReactionDetail {
                emoji: "👍".to_owned(),
                sender_name: "Alice".to_owned(),
            }],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Reactions"));
        assert!(lines[1].spans[0].content.contains("Alice"));
    }

    #[test]
    fn loaded_with_viewers_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![ViewerDetail {
                name: "Bob".to_owned(),
                view_date: 1700000000,
            }],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Read by"));
        assert!(lines[1].spans[0].content.contains("Bob"));
    }

    #[test]
    fn loaded_with_edit_date_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: Some(1700000000),
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Edited"));
    }

    #[test]
    fn loaded_with_read_date_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            reactions: vec![],
            viewers: vec![],
            read_date: Some(1700000000),
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Read at"));
    }
}
