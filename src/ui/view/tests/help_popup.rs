use ratatui::{backend::TestBackend, Terminal};

use super::super::render;
use crate::domain::{keymap::Keymap, shell_state::ShellState};

#[test]
fn help_opened_in_the_forum_topic_list_is_titled_forum_topics() {
    let mut state = ShellState::default();
    state.enter_forum(100, "Forum");
    state.show_help();
    let groups = Keymap::default().help_groups(state.key_context());

    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    terminal
        .draw(|frame| render(frame, &mut state, &groups))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains(" Help — Forum Topics "), "{text}");
    assert!(text.contains("select_next_topic"), "{text}");
}
