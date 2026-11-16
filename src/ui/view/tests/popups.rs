use ratatui::{backend::TestBackend, Terminal};

use super::super::render;
use crate::domain::{command_popup_state::CommandPopupKind, shell_state::ShellState};

#[test]
fn command_popup_is_drawn_over_the_file_picker() {
    let mut state = ShellState::default();
    state.open_file_picker(1, None);
    state.open_command_popup("Playing", CommandPopupKind::Playback);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    terminal
        .draw(|frame| render(frame, &mut state, &[]))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
    assert!(rendered.contains("Playing"), "{rendered}");
}
