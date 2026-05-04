//! Renders a centered chat info popup overlay showing chat details.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::chat_info_state::{ChatInfo, ChatInfoPopupState};

use super::{popup_utils, styles};

/// Renders the chat info popup as an overlay on top of existing content.
pub fn render_chat_info_popup(frame: &mut Frame<'_>, area: Rect, state: &ChatInfoPopupState) {
    let popup_area = popup_utils::centered_rect(area, 50, 60);

    frame.render_widget(Clear, popup_area);

    let title = format!(" Chat Info — {} ", state.title());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_info_lines(state);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

fn build_info_lines(state: &ChatInfoPopupState) -> Vec<Line<'static>> {
    match state {
        ChatInfoPopupState::Loading { .. } => {
            vec![Line::from(Span::styled(
                "Loading...",
                styles::chat_info_popup_value_style(),
            ))]
        }
        ChatInfoPopupState::Error { .. } => {
            vec![Line::from(Span::styled(
                "Failed to load chat info",
                styles::command_popup_error_style(),
            ))]
        }
        ChatInfoPopupState::Loaded(info) => build_loaded_lines(info),
    }
}

fn build_loaded_lines(info: &ChatInfo) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(build_field_line("Status", &info.status_line));

    if let Some(username) = &info.username {
        lines.push(build_field_line("Username", username));
    }

    if let Some(desc) = &info.description {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description",
            styles::chat_info_popup_label_style(),
        )));
        for text_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                text_line.to_owned(),
                styles::chat_info_popup_value_style(),
            )));
        }
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
    use crate::domain::chat::ChatType;

    #[test]
    fn loading_state_shows_loading_text() {
        let state = ChatInfoPopupState::Loading {
            chat_id: 1,
            title: "Alice".into(),
        };
        let lines = build_info_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Loading"));
    }

    #[test]
    fn error_state_shows_error_text() {
        let state = ChatInfoPopupState::Error {
            title: "Alice".into(),
        };
        let lines = build_info_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Failed"));
    }

    #[test]
    fn loaded_without_description_shows_status_and_footer() {
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Alice".into(),
            chat_type: ChatType::Private,
            status_line: "online".into(),
            username: None,
            description: None,
        });
        let lines = build_info_lines(&state);
        // Status line + empty + footer
        assert_eq!(lines.len(), 3);
        assert!(lines[0].spans[1].content.contains("online"));
    }

    #[test]
    fn loaded_with_description_shows_all_sections() {
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Dev Chat".into(),
            chat_type: ChatType::Group,
            status_line: "42 members".into(),
            username: None,
            description: Some("A developer community".into()),
        });
        let lines = build_info_lines(&state);
        // Status + empty + "Description" label + description text + empty + footer
        assert_eq!(lines.len(), 6);
        assert!(lines[0].spans[1].content.contains("42 members"));
        assert!(lines[2].spans[0].content.contains("Description"));
        assert!(lines[3].spans[0].content.contains("developer"));
    }

    #[test]
    fn multiline_description_creates_multiple_lines() {
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Chat".into(),
            chat_type: ChatType::Channel,
            status_line: "100 subscribers".into(),
            username: None,
            description: Some("Line 1\nLine 2\nLine 3".into()),
        });
        let lines = build_info_lines(&state);
        // Status + empty + "Description" label + 3 desc lines + empty + footer
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn loaded_with_username_shows_username_line() {
        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Alice".into(),
            chat_type: ChatType::Private,
            status_line: "online".into(),
            username: Some("@alice".into()),
            description: None,
        });
        let lines = build_info_lines(&state);
        // Status + Username + empty + footer
        assert_eq!(lines.len(), 4);
        assert!(lines[1].spans[0].content.contains("Username"));
        assert!(lines[1].spans[1].content.contains("@alice"));
    }

    #[test]
    fn build_field_line_formats_correctly() {
        let line = build_field_line("Status", "online");
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "Status: ");
        assert_eq!(line.spans[1].content, "online");
    }

    #[test]
    fn long_description_wraps_and_is_fully_visible() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = ChatInfoPopupState::Loaded(ChatInfo {
            title: "Chat".into(),
            chat_type: ChatType::Group,
            status_line: "5 members".into(),
            username: None,
            description: Some(
                "This is a very long description that exceeds the popup width".into(),
            ),
        });

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_chat_info_popup(frame, area, &state);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                rendered.push_str(buffer[(x, y)].symbol());
            }
        }

        assert!(
            rendered.contains("width"),
            "last word of description should be visible when wrapping is enabled"
        );
    }
}
