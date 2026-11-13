use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::domain::link_picker_state::LinkPickerState;

use super::{message_rendering::truncate_to_width, popup_utils, styles};

pub fn render_link_picker(frame: &mut Frame<'_>, area: Rect, state: &LinkPickerState) {
    let title = " Open ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(styles::chat_info_popup_border_style())
        .padding(Padding::new(2, 2, 1, 1));

    let max_width = usize::from(popup_utils::max_content_width(area, &block));
    let paragraph = Paragraph::new(build_lines(state, max_width));
    let popup_area = popup_utils::content_sized_rect(area, title, &block, &paragraph);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph.block(block), popup_area);
}

const ROW_INDENT: &str = "  ";
const FOOTER: [&str; 2] = ["j/k navigate, Enter or o open", "Press q or Esc to close"];

/// Rows are cut to `max_width` rather than wrapped, so that every link or
/// mention stays on its own line.
fn build_lines(state: &LinkPickerState, max_width: usize) -> Vec<Line<'static>> {
    let label_width = max_width.saturating_sub(ROW_INDENT.len());
    let mut lines: Vec<Line<'static>> = state
        .targets()
        .iter()
        .enumerate()
        .map(|(i, target)| {
            let style = if Some(i) == state.selected_index() {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                styles::chat_info_popup_value_style()
            };
            let label = truncate_to_width(&target.label(), label_width);
            Line::from(Span::styled(format!("{ROW_INDENT}{label}"), style))
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
    use crate::domain::message::LinkTarget;

    fn picker() -> LinkPickerState {
        LinkPickerState::new(vec![
            LinkTarget::Mention("alice".to_owned()),
            LinkTarget::Url("https://example.com/a/very/long/path".to_owned()),
        ])
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
    fn lists_mentions_and_links_with_footer() {
        let lines = build_lines(&picker(), 80);

        assert_eq!(lines.len(), 5);
        assert_eq!(line_text(&lines[0]), "  @alice");
        assert_eq!(
            line_text(&lines[1]),
            "  https://example.com/a/very/long/path"
        );
        assert_eq!(line_text(&lines[3]), "j/k navigate, Enter or o open");
        assert_eq!(line_text(&lines[4]), "Press q or Esc to close");
    }

    #[test]
    fn highlights_selected_target_only() {
        let mut state = picker();
        state.select_next();

        let lines = build_lines(&state, 80);

        assert!(!is_highlighted(&lines[0]));
        assert!(is_highlighted(&lines[1]));
    }

    #[test]
    fn long_link_is_cut_to_the_popup_width() {
        let lines = build_lines(&picker(), 20);

        assert_eq!(line_text(&lines[0]), "  @alice");
        assert_eq!(line_text(&lines[1]), "  https://example.c…");
    }
}
