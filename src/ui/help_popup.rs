use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
    Frame,
};

use crate::domain::keymap::{HelpGroup, KeyContext};

use super::{popup_utils, styles};

const ROW_INDENT: &str = "  ";

pub fn render_help_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    context: KeyContext,
    groups: &[HelpGroup],
) {
    let title = match context {
        KeyContext::ChatList => " Help — Chat List ",
        KeyContext::ForumTopicList => " Help — Forum Topics ",
        KeyContext::Messages => " Help — Messages ",
        KeyContext::Global => " Help ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let mut lines = build_hotkey_lines(groups);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press q, ? or Esc to close",
        styles::help_popup_footer_style(),
    )));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
}

fn build_hotkey_lines(groups: &[HelpGroup]) -> Vec<Line<'static>> {
    let max_key_width = groups
        .iter()
        .flat_map(|g| &g.entries)
        .map(|e| e.key_label.len())
        .max()
        .unwrap_or(0);

    let mut lines = Vec::new();
    for group in groups {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            group.title,
            styles::help_popup_group_style(),
        )));
        for entry in &group.entries {
            let padded_key = format!("{:<width$}", entry.key_label, width = max_key_width);
            lines.push(Line::from(vec![
                Span::raw(ROW_INDENT),
                Span::styled(padded_key, styles::help_popup_key_style()),
                Span::styled("  ", styles::help_popup_action_style()),
                Span::styled(entry.action_name.clone(), styles::help_popup_action_style()),
            ]));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use ratatui::{
        backend::TestBackend,
        style::{Color, Modifier},
        Terminal,
    };

    use super::*;
    use crate::domain::keymap::{HelpEntry, Keymap};

    fn entry(key_label: &str, action_name: &str) -> HelpEntry {
        HelpEntry {
            key_label: key_label.to_owned(),
            action_name: action_name.to_owned(),
        }
    }

    fn two_groups() -> Vec<HelpGroup> {
        vec![
            HelpGroup {
                title: "Navigation",
                entries: vec![entry("j", "next"), entry("Enter / l", "open")],
            },
            HelpGroup {
                title: "General",
                entries: vec![entry("q", "quit")],
            },
        ]
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn rendered_text(context: KeyContext, groups: &[HelpGroup]) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| render_help_popup(frame, frame.area(), context, groups))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn each_group_has_a_heading_and_indented_rows() {
        let text: Vec<String> = build_hotkey_lines(&two_groups())
            .iter()
            .map(line_text)
            .collect();

        assert_eq!(
            text,
            vec![
                "Navigation",
                "  j          next",
                "  Enter / l  open",
                "",
                "General",
                "  q          quit",
            ]
        );
    }

    #[test]
    fn group_heading_is_bold_yellow() {
        let lines = build_hotkey_lines(&two_groups());

        let heading = lines[0].spans[0].style;
        assert_eq!(heading.fg, Some(Color::Yellow));
        assert!(heading.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn key_is_bold_in_the_terminal_text_colour_and_action_is_gray() {
        let lines = build_hotkey_lines(&two_groups());

        let key = lines[1].spans[1].style;
        assert_eq!(key.fg, None);
        assert!(key.add_modifier.contains(Modifier::BOLD));
        let action = lines[1].spans[3].style;
        assert_eq!(action.fg, Some(Color::Gray));
    }

    #[test]
    fn no_groups_give_no_lines() {
        assert!(build_hotkey_lines(&[]).is_empty());
    }

    #[test]
    fn real_keymap_gives_a_line_per_heading_hotkey_and_gap() {
        let groups = Keymap::default().help_groups(KeyContext::Messages);
        let hotkeys: usize = groups.iter().map(|g| g.entries.len()).sum();

        let lines = build_hotkey_lines(&groups);

        assert_eq!(lines.len(), hotkeys + groups.len() * 2 - 1);
    }

    #[test]
    fn popup_title_names_the_focused_list() {
        let cases = [
            (KeyContext::ChatList, " Help — Chat List "),
            (KeyContext::ForumTopicList, " Help — Forum Topics "),
            (KeyContext::Messages, " Help — Messages "),
        ];
        for (context, title) in cases {
            let text = rendered_text(context, &two_groups());
            assert!(
                text.iter().any(|row| row.contains(title)),
                "{context:?}: {title}"
            );
        }
    }

    #[test]
    fn popup_is_sized_to_groups() {
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal
            .draw(|frame| {
                render_help_popup(frame, frame.area(), KeyContext::ChatList, &two_groups());
            })
            .unwrap();

        let rect = popup_utils::rendered_border_rect(terminal.backend().buffer()).unwrap();
        // The footer is the widest line; six group lines, blank, footer.
        assert_eq!((rect.width, rect.height), (26 + 6, 8 + 4));
    }
}
