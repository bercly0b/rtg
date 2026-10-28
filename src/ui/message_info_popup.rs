use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::message_info_state::{
    format_unix_timestamp, MessageAuthor, MessageInfo, MessageInfoPopupState,
};

use super::{popup_utils, styles};

pub fn render_message_info_popup(frame: &mut Frame<'_>, area: Rect, state: &MessageInfoPopupState) {
    let title = " Message Info ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_info_lines(state);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
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

    if let Some(author) = &info.author {
        lines.push(build_field_line("Author", &format_author(author)));
        has_content = true;
    }

    if !info.reactions.is_empty() {
        if has_content {
            lines.push(Line::from(""));
        }
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
    } else if !info.reaction_counts.is_empty() {
        if has_content {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "Reactions",
            styles::chat_info_popup_label_style(),
        )));
        for r in &info.reaction_counts {
            lines.push(Line::from(Span::styled(
                format!("  {} — {}", r.emoji, r.count),
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

fn format_author(author: &MessageAuthor) -> String {
    match &author.username {
        Some(username) => format!("{} @{username}", author.name),
        None => author.name.clone(),
    }
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
    use crate::domain::message_info_state::{ReactionCount, ReactionDetail, ViewerDetail};

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
            author: None,
            reactions: vec![],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("No additional info"));
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn author_with_username_shows_name_and_handle() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: Some(MessageAuthor {
                name: "Alice Smith".to_owned(),
                username: Some("alice".to_owned()),
            }),
            reactions: vec![],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert_eq!(line_text(&lines[0]), "Author: Alice Smith @alice");
        assert_eq!(
            lines[0].spans[0].style,
            styles::chat_info_popup_label_style()
        );
    }

    #[test]
    fn author_without_username_shows_name_only() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: Some(MessageAuthor {
                name: "Alice".to_owned(),
                username: None,
            }),
            reactions: vec![],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert_eq!(line_text(&lines[0]), "Author: Alice");
        assert!(!lines
            .iter()
            .any(|l| line_text(l).contains("No additional info")));
    }

    #[test]
    fn author_is_shown_above_reactions() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: Some(MessageAuthor {
                name: "Alice".to_owned(),
                username: Some("alice".to_owned()),
            }),
            reactions: vec![ReactionDetail {
                emoji: "👍".to_owned(),
                sender_name: "Bob".to_owned(),
            }],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert_eq!(line_text(&lines[0]), "Author: Alice @alice");
        assert_eq!(line_text(&lines[1]), "");
        assert_eq!(line_text(&lines[2]), "Reactions");
        assert!(line_text(&lines[3]).contains("Bob"));
    }

    #[test]
    fn loaded_with_reactions_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: None,
            reactions: vec![ReactionDetail {
                emoji: "👍".to_owned(),
                sender_name: "Alice".to_owned(),
            }],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Reactions"));
        assert!(lines[1].spans[0].content.contains("Alice"));
    }

    #[test]
    fn loaded_with_reaction_counts_shows_counts() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: None,
            reactions: vec![],
            reaction_counts: vec![ReactionCount {
                emoji: "👍".to_owned(),
                count: 42,
            }],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Reactions"));
        assert!(lines[1].spans[0].content.contains("42"));
    }

    #[test]
    fn named_reactions_take_precedence_over_counts() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: None,
            reactions: vec![ReactionDetail {
                emoji: "👍".to_owned(),
                sender_name: "Alice".to_owned(),
            }],
            reaction_counts: vec![ReactionCount {
                emoji: "👍".to_owned(),
                count: 42,
            }],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[1].spans[0].content.contains("Alice"));
        assert!(!lines[1].spans[0].content.contains("42"));
    }

    #[test]
    fn loaded_with_viewers_shows_section() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: None,
            reactions: vec![],
            reaction_counts: vec![],
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
            author: None,
            reactions: vec![],
            reaction_counts: vec![],
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
            author: None,
            reactions: vec![],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: Some(1700000000),
            edit_date: None,
        });
        let lines = build_info_lines(&state);
        assert!(lines[0].spans[0].content.contains("Read at"));
    }

    fn rendered_popup_rect(state: &MessageInfoPopupState, width: u16, height: u16) -> Rect {
        use ratatui::{backend::TestBackend, Terminal};

        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_message_info_popup(frame, frame.area(), state))
            .unwrap();
        popup_utils::rendered_border_rect(terminal.backend().buffer()).unwrap()
    }

    #[test]
    fn loading_popup_is_sized_to_content() {
        let state = MessageInfoPopupState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        let rect = rendered_popup_rect(&state, 100, 40);
        assert_eq!((rect.width, rect.height), (30, 5));
    }

    #[test]
    fn loaded_popup_height_follows_lines() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: Some(MessageAuthor {
                name: "Alice".to_owned(),
                username: None,
            }),
            reactions: vec![ReactionDetail {
                emoji: "👍".to_owned(),
                sender_name: "Bob".to_owned(),
            }],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let rect = rendered_popup_rect(&state, 100, 40);
        // Author, blank, label, reaction, blank, footer.
        assert_eq!(rect.height, 6 + 4);
    }

    #[test]
    fn long_author_wraps_instead_of_being_cut() {
        let state = MessageInfoPopupState::Loaded(MessageInfo {
            author: Some(MessageAuthor {
                name: "Name ".repeat(20).trim_end().to_owned(),
                username: Some("tail".to_owned()),
            }),
            reactions: vec![],
            reaction_counts: vec![],
            viewers: vec![],
            read_date: None,
            edit_date: None,
        });
        let rect = rendered_popup_rect(&state, 60, 30);
        assert_eq!(rect.width, 48);
        assert!(rect.height > 3 + 4);
    }
}
