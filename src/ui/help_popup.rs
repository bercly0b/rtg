use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::{keymap::HelpEntry, shell_state::ActivePane};

use super::{popup_utils, styles};

pub fn render_help_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    active_pane: ActivePane,
    entries: &[HelpEntry],
) {
    let popup_area = popup_utils::centered_rect(area, 50, 70);

    frame.render_widget(Clear, popup_area);

    let title = match active_pane {
        ActivePane::ChatList => "Help — Chat List",
        ActivePane::Messages | ActivePane::MessageInput => "Help — Messages",
    };

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(styles::help_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let mut lines = build_hotkey_lines(entries);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press q, ? or Esc to close",
        styles::help_popup_footer_style(),
    )));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn build_hotkey_lines(entries: &[HelpEntry]) -> Vec<Line<'static>> {
    let max_key_width = entries.iter().map(|e| e.key_label.len()).max().unwrap_or(0);

    entries
        .iter()
        .map(|entry| {
            let padded_key = format!("{:<width$}", entry.key_label, width = max_key_width);
            Line::from(vec![
                Span::styled(padded_key, styles::help_popup_key_style()),
                Span::styled("  ", styles::help_popup_action_style()),
                Span::styled(entry.action_name.clone(), styles::help_popup_action_style()),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keymap::{KeyContext, Keymap};

    #[test]
    fn build_hotkey_lines_aligns_keys() {
        let entries = vec![
            HelpEntry {
                key_label: "j".to_owned(),
                action_name: "next".to_owned(),
            },
            HelpEntry {
                key_label: "Enter / l".to_owned(),
                action_name: "open".to_owned(),
            },
        ];
        let lines = build_hotkey_lines(&entries);
        assert_eq!(lines.len(), 2);
        let first_key = &lines[0].spans[0];
        assert_eq!(first_key.content.len(), 9);
    }

    #[test]
    fn build_hotkey_lines_empty_input() {
        let lines = build_hotkey_lines(&[]);
        assert!(lines.is_empty());
    }

    #[test]
    fn build_hotkey_lines_single_entry_no_padding_needed() {
        let entries = vec![HelpEntry {
            key_label: "q".to_owned(),
            action_name: "quit".to_owned(),
        }];
        let lines = build_hotkey_lines(&entries);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        assert_eq!(lines[0].spans[0].content, "q");
        assert_eq!(lines[0].spans[1].content, "  ");
        assert_eq!(lines[0].spans[2].content, "quit");
    }

    #[test]
    fn build_hotkey_lines_with_real_keymap() {
        let km = Keymap::default();
        let entries = km.help_entries(KeyContext::ChatList);
        let lines = build_hotkey_lines(&entries);
        assert_eq!(lines.len(), entries.len());
        assert!(!lines.is_empty());
    }

    #[test]
    fn build_hotkey_lines_with_messages_keymap() {
        let km = Keymap::default();
        let entries = km.help_entries(KeyContext::Messages);
        let lines = build_hotkey_lines(&entries);
        assert_eq!(lines.len(), entries.len());
        assert!(!lines.is_empty());
    }
}
