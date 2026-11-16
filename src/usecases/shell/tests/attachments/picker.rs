use super::*;

// ── opening the picker ──

#[test]
fn a_opens_the_file_picker_and_starts_the_file_list_command() {
    let mut o = DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        ShellState::default(),
        None,
        OrchestratorConfig {
            min_display_messages: 1,
            file_list_cmd: "fd . {dirs}".to_owned(),
            attach_dirs: vec!["~/Desktop".to_owned()],
            ..OrchestratorConfig::default()
        },
    );
    inject_chat_list(&mut o, vec![chat(1, "Chat")]);
    press(&mut o, "enter");
    inject_messages(&mut o, 1, vec![message(10, "Hello")]);

    press(&mut o, "a");

    let picker = o.state().file_picker().expect("file picker must be open");
    assert_eq!(picker.listing(), &FileListing::Running);
    assert_eq!(
        o.dispatcher.last_file_listing(),
        Some((
            picker.listing_id(),
            "fd . {dirs}".to_owned(),
            vec!["~/Desktop".to_owned()]
        ))
    );
}

#[test]
fn a_does_nothing_without_an_open_chat() {
    let mut o = orchestrator_with_chats(vec![chat(1, "Chat")]);
    o.state_mut().set_active_pane(ActivePane::Messages);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.last_file_listing(), None);
}

#[test]
fn a_in_a_channel_says_it_is_read_only() {
    let mut o =
        orchestrator_with_open_chat(vec![channel_chat(1, "News")], 1, vec![message(10, "Hello")]);
    assert_eq!(o.state().open_chat().chat_type(), ChatType::Channel);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(
        o.state().active_notification(),
        Some("Channel is read-only")
    );
}

#[test]
fn a_in_a_closed_topic_says_it_is_closed() {
    let mut closed = topic(1, 7, "Releases", 1);
    closed.is_closed = true;
    let mut o = orchestrator_with_chats(vec![forum_chat(1, "Forum")]);
    press(&mut o, "enter");
    inject_forum_topics(&mut o, 1, vec![closed]);
    press(&mut o, "enter");
    inject_topic_messages(&mut o, 1, Some(7), vec![message(10, "Hello")]);

    press(&mut o, "a");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.state().active_notification(), Some("Topic is closed"));
}

// ── listing ──

#[test]
fn listed_files_fill_the_picker_newest_first() {
    let o = picker_listing(vec![entry("/d/old.png", 100), entry("/d/new.png", 200)]);

    let picker = o.state().file_picker().unwrap();
    assert_eq!(picker.visible_len(), 2);
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
}

#[test]
fn files_of_a_previous_picker_are_ignored() {
    let mut o = open_chat();
    press(&mut o, "a");
    let stale_id = o.dispatcher.last_file_listing().unwrap().0;
    press(&mut o, "esc");
    press(&mut o, "a");

    list_files(&mut o, stale_id, vec![entry("/d/stale.png", 100)]);
    complete(
        &mut o,
        BackgroundTaskResult::FileListingFinished {
            listing_id: stale_id,
            error: Some("killed".to_owned()),
        },
    );

    let picker = o.state().file_picker().unwrap();
    assert_eq!(picker.visible_len(), 0);
    assert_eq!(picker.listing(), &FileListing::Running);
}

#[test]
fn failed_listing_keeps_the_picker_open_with_the_reason() {
    let mut o = open_chat();
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;

    complete(
        &mut o,
        BackgroundTaskResult::FileListingFinished {
            listing_id,
            error: Some("command not found: fd".to_owned()),
        },
    );

    assert_eq!(
        o.state().file_picker().unwrap().listing(),
        &FileListing::Failed("command not found: fd".to_owned())
    );
}

// ── keys inside the picker ──

#[test]
fn typing_filters_the_files_and_backspace_widens_the_filter() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200), entry("/d/notes.txt", 100)]);

    type_text(&mut o, "txt");
    assert_eq!(o.state().file_picker().unwrap().query(), "txt");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/notes.txt"));

    press(&mut o, "backspace");
    press(&mut o, "backspace");
    press(&mut o, "backspace");
    assert_eq!(o.state().file_picker().unwrap().visible_len(), 2);
}

#[test]
fn letters_that_are_hotkeys_elsewhere_go_into_the_query() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200)]);

    type_text(&mut o, "jkq");

    assert_eq!(o.state().file_picker().unwrap().query(), "jkq");
    assert!(o.state().is_running());
}

#[test]
fn ctrl_j_and_ctrl_k_move_the_selection() {
    let mut o = picker_listing(vec![entry("/d/new.png", 200), entry("/d/old.png", 100)]);

    press_ctrl(&mut o, "j");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/old.png"));

    press_ctrl(&mut o, "k");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
    assert_eq!(o.state().file_picker().unwrap().query(), "");
}

#[test]
fn arrows_move_the_selection() {
    let mut o = picker_listing(vec![entry("/d/new.png", 200), entry("/d/old.png", 100)]);

    press(&mut o, "down");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/old.png"));

    press(&mut o, "up");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/new.png"));
}

#[test]
fn esc_closes_the_picker_and_stops_the_file_list_command() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200)]);

    press(&mut o, "esc");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.stopped_file_listings(), 1);
    assert_eq!(o.dispatcher.attachment_send_count(), 0);
}

// ── opening the selected file ──

#[test]
fn ctrl_l_opens_the_selected_file_and_keeps_the_picker_as_it_was() {
    let mut o = picker_listing(vec![
        entry("/d/new.png", 300),
        entry("/d/mid.png", 200),
        entry("/d/notes.txt", 100),
    ]);
    type_text(&mut o, "png");
    press_ctrl(&mut o, "j");

    press_ctrl(&mut o, "l");

    assert_eq!(
        o.dispatcher.file_opens(),
        [(DEFAULT_OPEN.to_owned(), "/d/mid.png".to_owned())]
    );
    assert_eq!(o.state().file_picker().unwrap().query(), "png");
    assert_eq!(selected_path(&o).as_deref(), Some("/d/mid.png"));
    assert_eq!(o.dispatcher.stopped_file_listings(), 0);
    assert_eq!(o.dispatcher.attachment_send_count(), 0);
}

#[test]
fn ctrl_l_uses_the_platform_opener_even_when_an_open_handler_matches() {
    let mut o = picker_listing(vec![entry("/d/shot.png", 200)]);
    o.media
        .open_handlers
        .insert("image/*".to_owned(), "true {file_path}".to_owned());

    press_ctrl(&mut o, "l");

    assert_eq!(
        o.dispatcher.file_opens(),
        [(DEFAULT_OPEN.to_owned(), "/d/shot.png".to_owned())]
    );
    assert!(o.state().command_popup().is_none());
}

#[test]
fn ctrl_l_with_nothing_selected_does_nothing() {
    let mut o = picker_listing(vec![]);

    press_ctrl(&mut o, "l");

    assert!(o.dispatcher.file_opens().is_empty());
    assert!(o.state().command_popup().is_none());
    assert_eq!(o.state().file_picker().unwrap().query(), "");
}
