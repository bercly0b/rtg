//! Renders a centered help popup overlay showing available hotkeys.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::{
    help_content::{self, HotkeyEntry},
    shell_state::ActivePane,
};

use super::{popup_utils, styles};

/// Renders the help popup as an overlay on top of existing content.
///
/// The popup is centered and sized proportionally to the terminal area.
/// Content depends on which pane is currently active.
pub fn render_help_popup(frame: &mut Frame<'_>, area: Rect, active_pane: ActivePane) {
    let popup_area = popup_utils::centered_rect(area, 50, 70);

    // Erase underlying content under the popup.
    frame.render_widget(Clear, popup_area);

    let (title, entries) = match active_pane {
        ActivePane::ChatList => ("Help — Chat List", help_content::chat_list_hotkeys()),
        ActivePane::Messages => ("Help — Messages", help_content::messages_hotkeys()),
        // MessageInput should not open help, but handle gracefully.
        ActivePane::MessageInput => ("Help — Messages", help_content::messages_hotkeys()),
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

/// Builds styled lines from hotkey entries, aligning keys and action names.
fn build_hotkey_lines(entries: &[HotkeyEntry]) -> Vec<Line<'static>> {
    // Find max key label width for alignment.
    let max_key_width = entries.iter().map(|e| e.key_label.len()).max().unwrap_or(0);

    entries
        .iter()
        .map(|entry| {
            let padded_key = format!("{:<width$}", entry.key_label, width = max_key_width);
            Line::from(vec![
                Span::styled(padded_key, styles::help_popup_key_style()),
                Span::styled("  ", styles::help_popup_action_style()),
                Span::styled(
                    entry.action_name.to_owned(),
                    styles::help_popup_action_style(),
                ),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hotkey_lines_aligns_keys() {
        let entries = &[
            HotkeyEntry {
                key_label: "j",
                action_name: "next",
            },
            HotkeyEntry {
                key_label: "Enter / l",
                action_name: "open",
            },
        ];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), 2);
        // First line key span should be padded to match "Enter / l" width (9 chars)
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
        let entries = &[HotkeyEntry {
            key_label: "q",
            action_name: "quit",
        }];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        // key, separator, action
        assert_eq!(lines[0].spans[0].content, "q");
        assert_eq!(lines[0].spans[1].content, "  ");
        assert_eq!(lines[0].spans[2].content, "quit");
    }

    #[test]
    fn build_hotkey_lines_has_correct_span_count() {
        let entries = &[
            HotkeyEntry {
                key_label: "j",
                action_name: "next",
            },
            HotkeyEntry {
                key_label: "k",
                action_name: "prev",
            },
        ];
        let lines = build_hotkey_lines(entries);
        for line in &lines {
            assert_eq!(line.spans.len(), 3, "each line should have 3 spans");
        }
    }

    #[test]
    fn build_hotkey_lines_uses_correct_styles() {
        let entries = &[HotkeyEntry {
            key_label: "j",
            action_name: "next",
        }];
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines[0].spans[0].style, styles::help_popup_key_style());
        assert_eq!(lines[0].spans[2].style, styles::help_popup_action_style());
    }

    #[test]
    fn build_hotkey_lines_with_real_chat_list_data() {
        let entries = help_content::chat_list_hotkeys();
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), entries.len());
    }

    #[test]
    fn build_hotkey_lines_with_real_messages_data() {
        let entries = help_content::messages_hotkeys();
        let lines = build_hotkey_lines(entries);
        assert_eq!(lines.len(), entries.len());
    }
}
