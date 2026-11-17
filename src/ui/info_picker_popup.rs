use ratatui::{layout::Rect, Frame};

use crate::domain::info_picker_state::InfoPickerState;

use super::link_picker_popup::render_picker;

const FOOTER: [&str; 2] = ["j/k navigate, Enter or I open", "Press q or Esc to close"];

pub fn render_info_picker(frame: &mut Frame<'_>, area: Rect, state: &InfoPickerState) {
    render_picker(
        frame,
        area,
        " Info ",
        &labels(state),
        state.selected_index(),
        &FOOTER,
    );
}

fn labels(state: &InfoPickerState) -> Vec<String> {
    state
        .entries()
        .iter()
        .map(|entry| entry.label.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;
    use crate::domain::{chat_info_state::ChatInfoSubject, info_picker_state::InfoPickerEntry};

    fn rendered(state: &InfoPickerState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal
            .draw(|frame| render_info_picker(frame, frame.area(), state))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer
            .content()
            .chunks(usize::from(buffer.area.width))
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn lists_the_sender_and_mentions_under_the_info_title() {
        let state = InfoPickerState::new(vec![
            InfoPickerEntry {
                label: "Bob".to_owned(),
                subject: ChatInfoSubject::User(7),
            },
            InfoPickerEntry {
                label: "@alice".to_owned(),
                subject: ChatInfoSubject::Mention("alice".to_owned()),
            },
        ]);

        let screen = rendered(&state);

        let position = |needle: &str| {
            screen
                .find(needle)
                .unwrap_or_else(|| panic!("{needle:?} is not on screen:\n{screen}"))
        };
        assert!(position(" Info ") < position("Bob"));
        assert!(position("Bob") < position("@alice"));
        assert!(position("@alice") < position("j/k navigate, Enter or I open"));
        position("Press q or Esc to close");
    }
}
