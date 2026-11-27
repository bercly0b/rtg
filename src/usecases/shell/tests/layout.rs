use super::*;
use crate::domain::layout::ColumnLayout;

fn press(o: &mut TestOrchestrator, key: &str) {
    o.handle_event(AppEvent::InputKey(KeyInput::new(key, false)))
        .unwrap();
}

#[test]
fn shift_l_toggles_the_layout_from_the_chat_list() {
    let mut o = make_orchestrator();
    o.state_mut().set_window_width(80);

    press(&mut o, "L");
    assert_eq!(o.state().layout(), ColumnLayout::TwoColumn);

    press(&mut o, "L");
    assert_eq!(o.state().layout(), ColumnLayout::SingleColumn);
}

#[test]
fn shift_l_toggles_the_layout_from_messages() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    o.state_mut().set_window_width(80);

    press(&mut o, "L");

    assert_eq!(o.state().layout(), ColumnLayout::TwoColumn);
    assert_eq!(o.state().active_pane(), ActivePane::Messages);
}

#[test]
fn shift_l_toggles_the_layout_from_the_forum_topic_list() {
    let mut o = orchestrator_with_chats(vec![forum_chat(1, "Forum")]);
    press(&mut o, "l");
    assert_eq!(o.state().key_context(), KeyContext::ForumTopicList);
    o.state_mut().set_window_width(80);

    press(&mut o, "L");

    assert_eq!(o.state().layout(), ColumnLayout::TwoColumn);
}

#[test]
fn shift_l_is_a_letter_in_the_message_input() {
    let mut o = orchestrator_with_open_chat(vec![chat(1, "General")], 1, vec![message(1, "Hi")]);
    o.state_mut().set_window_width(80);
    press(&mut o, "i");

    press(&mut o, "L");

    assert_eq!(o.state().message_input().text(), "L");
    assert_eq!(o.state().layout(), ColumnLayout::SingleColumn);
}
