use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::reaction_picker_state::ReactionPickerState;

use super::{popup_utils, styles};

pub fn render_reaction_picker(frame: &mut Frame<'_>, area: Rect, state: &ReactionPickerState) {
    let popup_area = popup_utils::centered_rect(area, 50, 60);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Reactions ")
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let lines = build_lines(state);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, popup_area);
}

fn build_lines(state: &ReactionPickerState) -> Vec<Line<'static>> {
    match state {
        ReactionPickerState::Loading { .. } => {
            vec![Line::from(Span::styled(
                "Loading...",
                styles::chat_info_popup_value_style(),
            ))]
        }
        ReactionPickerState::Error => {
            vec![Line::from(Span::styled(
                "Failed to load reactions",
                styles::command_popup_error_style(),
            ))]
        }
        ReactionPickerState::Ready(data) => {
            let mut lines: Vec<Line<'static>> = data
                .items
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let style = if i == data.selected_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        styles::chat_info_popup_value_style()
                    };
                    let name = r.display_name();
                    let label = if name.is_empty() {
                        format!("  {}", r.emoji)
                    } else {
                        format!("  {}  {}", r.emoji, name)
                    };
                    Line::from(Span::styled(label, style))
                })
                .collect();

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "j/k navigate, Enter select",
                styles::help_popup_footer_style(),
            )));
            lines.push(Line::from(Span::styled(
                "Press q, Esc or R to close",
                styles::help_popup_footer_style(),
            )));
            lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::reaction_picker_state::{AvailableReaction, ReactionPickerData};

    #[test]
    fn loading_state_shows_loading() {
        let state = ReactionPickerState::Loading {
            chat_id: 1,
            message_id: 2,
        };
        let lines = build_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Loading"));
    }

    #[test]
    fn error_state_shows_error() {
        let state = ReactionPickerState::Error;
        let lines = build_lines(&state);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains("Failed"));
    }

    #[test]
    fn ready_state_shows_reactions_with_footer() {
        let reactions = vec![
            AvailableReaction {
                emoji: "👍".into(),
                needs_premium: false,
            },
            AvailableReaction {
                emoji: "❤".into(),
                needs_premium: false,
            },
        ];
        let state = ReactionPickerState::Ready(ReactionPickerData::new(reactions, 1, 2));
        let lines = build_lines(&state);
        assert_eq!(lines.len(), 5);
        assert!(lines[0].spans[0].content.contains("👍"));
        assert!(lines[0].spans[0].content.contains("thumbs_up"));
        assert!(lines[1].spans[0].content.contains("❤"));
        assert!(lines[1].spans[0].content.contains("heart"));
        assert!(lines[3].spans[0].content.contains("j/k navigate"));
        assert!(lines[4].spans[0]
            .content
            .contains("Press q, Esc or R to close"));
    }
}
