use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::chat_folder::ChatFolderPickerState;

use super::{popup_utils, styles};

pub fn render_chat_folder_picker(frame: &mut Frame<'_>, area: Rect, state: &ChatFolderPickerState) {
    let title = " Chat folders ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_lines(state);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
}

fn build_lines(state: &ChatFolderPickerState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = state
        .entries()
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if Some(i) == state.selected_index() {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                styles::chat_info_popup_value_style()
            };
            Line::from(Span::styled(format!("  {}", entry.title), style))
        })
        .collect();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "j/k navigate, Enter select",
        styles::help_popup_footer_style(),
    )));
    lines.push(Line::from(Span::styled(
        "Press q, Esc or f to close",
        styles::help_popup_footer_style(),
    )));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat_folder::{ChatFolder, ChatFolders, ChatListKind, MAIN_CHAT_LIST_TITLE};

    fn picker(active: ChatListKind) -> ChatFolderPickerState {
        let folders = ChatFolders {
            folders: vec![
                ChatFolder {
                    id: 3,
                    title: "Work".to_owned(),
                },
                ChatFolder {
                    id: 7,
                    title: "Family".to_owned(),
                },
            ],
            main_list_position: 0,
        };
        ChatFolderPickerState::new(&folders, active).unwrap()
    }

    fn is_highlighted(line: &Line<'_>) -> bool {
        line.spans[0].style.bg == Some(Color::White)
    }

    #[test]
    fn lists_main_list_and_folders_with_footer() {
        let lines = build_lines(&picker(ChatListKind::Main));

        assert_eq!(lines.len(), 6);
        assert!(lines[0].spans[0].content.contains(MAIN_CHAT_LIST_TITLE));
        assert!(lines[1].spans[0].content.contains("Work"));
        assert!(lines[2].spans[0].content.contains("Family"));
        assert!(lines[4].spans[0].content.contains("j/k navigate"));
        assert!(lines[5].spans[0]
            .content
            .contains("Press q, Esc or f to close"));
    }

    #[test]
    fn highlights_selected_entry_only() {
        let lines = build_lines(&picker(ChatListKind::Folder(3)));

        assert!(!is_highlighted(&lines[0]));
        assert!(is_highlighted(&lines[1]));
        assert!(!is_highlighted(&lines[2]));
    }

    #[test]
    fn popup_is_sized_to_content() {
        use ratatui::{backend::TestBackend, Terminal};

        let state = picker(ChatListKind::Main);
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| render_chat_folder_picker(frame, frame.area(), &state))
            .unwrap();

        let rect = popup_utils::rendered_border_rect(terminal.backend().buffer()).unwrap();
        // The footer is the widest line; three entries, blank, two footer lines.
        assert_eq!((rect.width, rect.height), (26 + 6, 6 + 4));
    }
}
