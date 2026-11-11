use std::collections::HashMap;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::domain::chat_folder::{ChatFolderPickerState, ChatListKind};

use super::{popup_utils, styles};

pub fn render_chat_folder_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ChatFolderPickerState,
    unread_chat_counts: &HashMap<ChatListKind, u32>,
) {
    let title = " Chat folders ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_lines(state, unread_chat_counts);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
}

const FOOTER: [&str; 2] = ["j/k navigate, Enter select", "Press q, Esc or f to close"];

fn build_lines(
    state: &ChatFolderPickerState,
    unread_chat_counts: &HashMap<ChatListKind, u32>,
) -> Vec<Line<'static>> {
    let rows: Vec<(String, Option<String>)> = state
        .entries()
        .iter()
        .map(|entry| {
            let badge = unread_chat_counts
                .get(&entry.list)
                .filter(|&&unread| unread > 0)
                .map(|unread| format!("[{unread}]"));
            (format!("  {}", entry.title), badge)
        })
        .collect();

    // Badges end at the right edge of the popup content, which is as wide as
    // its widest line: a footer line or a row with a one-space gap.
    let content_width = rows
        .iter()
        .map(|(title, badge)| title.width() + badge.as_ref().map_or(0, |badge| 1 + badge.width()))
        .chain(FOOTER.iter().map(|line| line.width()))
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line<'static>> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (title, badge))| {
            let selected = Some(i) == state.selected_index();
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                styles::chat_info_popup_value_style()
            };
            let Some(badge) = badge else {
                return Line::from(Span::styled(title, style));
            };

            let gap = content_width - title.width() - badge.width();
            let badge_style = if selected {
                style
            } else {
                styles::unread_chat_count_style()
            };
            Line::from(vec![
                Span::styled(title, style),
                Span::styled(format!("{}{badge}", " ".repeat(gap)), badge_style),
            ])
        })
        .collect();

    lines.push(Line::from(""));
    lines.extend(
        FOOTER
            .iter()
            .map(|line| Line::from(Span::styled(*line, styles::help_popup_footer_style()))),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chat_folder::{
        ChatFolder, ChatFolders, ChatListKind, ARCHIVE_CHAT_LIST_TITLE, MAIN_CHAT_LIST_TITLE,
    };

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
        ChatFolderPickerState::new(&folders, active)
    }

    fn no_counts() -> HashMap<ChatListKind, u32> {
        HashMap::new()
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn is_highlighted(line: &Line<'_>) -> bool {
        line.spans[0].style.bg == Some(Color::White)
    }

    #[test]
    fn lists_main_list_folders_and_archive_with_footer() {
        let lines = build_lines(&picker(ChatListKind::Main), &no_counts());

        assert_eq!(lines.len(), 7);
        assert!(lines[0].spans[0].content.contains(MAIN_CHAT_LIST_TITLE));
        assert!(lines[1].spans[0].content.contains("Work"));
        assert!(lines[2].spans[0].content.contains("Family"));
        assert!(lines[3].spans[0].content.contains(ARCHIVE_CHAT_LIST_TITLE));
        assert!(lines[5].spans[0].content.contains("j/k navigate"));
        assert!(lines[6].spans[0]
            .content
            .contains("Press q, Esc or f to close"));
    }

    #[test]
    fn highlights_selected_entry_only() {
        let lines = build_lines(&picker(ChatListKind::Folder(3)), &no_counts());

        assert!(!is_highlighted(&lines[0]));
        assert!(is_highlighted(&lines[1]));
        assert!(!is_highlighted(&lines[2]));
    }

    #[test]
    fn shows_unread_chat_counts_in_a_right_aligned_column() {
        let counts = HashMap::from([
            (ChatListKind::Main, 12),
            (ChatListKind::Folder(3), 0),
            (ChatListKind::Archive, 3),
        ]);

        let lines = build_lines(&picker(ChatListKind::Folder(7)), &counts);

        assert_eq!(line_text(&lines[0]), "  All chats           [12]");
        assert_eq!(line_text(&lines[1]), "  Work");
        assert_eq!(line_text(&lines[2]), "  Family");
        assert_eq!(line_text(&lines[3]), "  Archive              [3]");
    }

    #[test]
    fn long_title_with_a_count_widens_the_rows_past_the_footer() {
        let folders = ChatFolders {
            folders: vec![ChatFolder {
                id: 3,
                title: "A folder with a very long title".to_owned(),
            }],
            main_list_position: 0,
        };
        let state = ChatFolderPickerState::new(&folders, ChatListKind::Main);
        let counts = HashMap::from([(ChatListKind::Folder(3), 5), (ChatListKind::Archive, 43)]);

        let lines = build_lines(&state, &counts);

        assert_eq!(
            line_text(&lines[1]),
            "  A folder with a very long title [5]"
        );
        assert_eq!(
            line_text(&lines[2]),
            "  Archive                        [43]"
        );
    }

    #[test]
    fn unread_chat_count_is_dimmed_and_highlighted_with_its_row() {
        let counts = HashMap::from([(ChatListKind::Main, 2), (ChatListKind::Archive, 3)]);

        let lines = build_lines(&picker(ChatListKind::Main), &counts);

        assert_eq!(lines[0].spans[1].style, lines[0].spans[0].style);
        assert!(is_highlighted(&lines[0]));
        assert_eq!(lines[3].spans[1].style, styles::unread_chat_count_style());
    }

    #[test]
    fn popup_is_sized_to_content() {
        use ratatui::{backend::TestBackend, Terminal};

        let state = picker(ChatListKind::Main);
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| render_chat_folder_picker(frame, frame.area(), &state, &no_counts()))
            .unwrap();

        let rect = popup_utils::rendered_border_rect(terminal.backend().buffer()).unwrap();
        // The footer is the widest line; four entries, blank, two footer lines.
        assert_eq!((rect.width, rect.height), (26 + 6, 7 + 4));
    }
}
