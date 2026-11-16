use super::*;

// ── sending ──

#[test]
fn enter_sends_the_selected_file_and_shows_it_as_sending() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200), entry("/d/old.pdf", 100)]);
    press_ctrl(&mut o, "j");

    press(&mut o, "enter");

    assert!(o.state().file_picker().is_none());
    assert_eq!(o.dispatcher.stopped_file_listings(), 1);

    let pending = o.state().open_chat().messages().last().unwrap().clone();
    assert_eq!(pending.status, MessageStatus::Sending);
    assert_eq!(pending.media, MessageMedia::Document);
    assert_eq!(pending.text, "");
    assert_eq!(
        sent(&o),
        (
            1,
            None,
            OutgoingAttachment {
                pending_id: pending.id,
                path: "/d/old.pdf".to_owned(),
                size_bytes: 42,
                kind: AttachmentKind::Document,
                caption: String::new(),
                reply_to: None,
            }
        )
    );
}

#[test]
fn enter_with_no_file_selected_sends_nothing() {
    let mut o = picker_listing(Vec::new());

    press(&mut o, "enter");

    assert!(o.state().file_picker().is_some());
    assert_eq!(o.dispatcher.attachment_send_count(), 0);
}

#[test]
fn image_within_the_photo_limits_is_sent_as_a_photo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shot.png");
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    png.extend(b"IHDR");
    png.extend(640u32.to_be_bytes());
    png.extend(480u32.to_be_bytes());
    png.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    std::fs::write(&path, png).unwrap();
    let mut o = picker_listing(vec![entry(path.to_str().unwrap(), 200)]);

    press(&mut o, "enter");

    assert_eq!(
        sent(&o).2.kind,
        AttachmentKind::Photo {
            width: 640,
            height: 480
        }
    );
    assert_eq!(
        o.state().open_chat().messages().last().unwrap().media,
        MessageMedia::Photo
    );
}

#[test]
fn text_in_the_input_goes_as_the_caption_and_the_input_is_cleared() {
    let mut o = open_chat();
    press(&mut o, "i");
    type_text(&mut o, " Look at this ");
    press(&mut o, "esc");
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    assert_eq!(sent(&o).2.caption, "Look at this");
    assert_eq!(o.state().message_input().text(), "");
    assert_eq!(
        o.state().open_chat().messages().last().unwrap().text,
        "Look at this"
    );
}

#[test]
fn reply_target_is_passed_on_with_the_attachment() {
    let mut o = open_chat();
    let reply = ReplyContext {
        message_id: 10,
        sender_name: "User".to_owned(),
        text: "Hello".to_owned(),
    };
    o.state_mut()
        .message_input_mut()
        .set_reply_to(reply.clone());
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    assert_eq!(sent(&o).2.reply_to, Some(reply));
    assert!(o.state().message_input().reply_to().is_none());
    let pending = o.state().open_chat().messages().last().unwrap();
    assert_eq!(pending.reply_to.as_ref().unwrap().text, "Hello");
}

#[test]
fn attachment_goes_into_the_open_forum_topic() {
    let mut o = orchestrator_with_chats(vec![forum_chat(1, "Forum")]);
    press(&mut o, "enter");
    inject_forum_topics(&mut o, 1, vec![topic(1, 7, "Releases", 1)]);
    press(&mut o, "enter");
    inject_topic_messages(&mut o, 1, Some(7), vec![message(10, "Hello")]);
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);

    press(&mut o, "enter");

    let (chat_id, topic_id, _) = sent(&o);
    assert_eq!((chat_id, topic_id), (1, Some(7)));
}

// ── failed send ──

#[test]
fn failed_send_removes_the_pending_message_and_brings_the_caption_back() {
    let mut o = open_chat();
    press(&mut o, "i");
    type_text(&mut o, "Look");
    press(&mut o, "esc");
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    let pending_id = sent(&o).2.pending_id;

    fail_send(&mut o, "SEND_ATTACHMENT_FAILED");

    assert!(o
        .state()
        .open_chat()
        .messages()
        .iter()
        .all(|message| message.id != pending_id));
    assert_eq!(o.state().message_input().text(), "Look");
    assert_eq!(
        o.state().active_notification(),
        Some("Failed to send the file")
    );
}

#[test]
fn failed_send_brings_the_caption_back_even_after_leaving_the_chat() {
    let mut o = orchestrator_with_open_chat(
        vec![chat(1, "Chat"), chat(2, "Other")],
        1,
        vec![message(10, "Hello")],
    );
    press(&mut o, "i");
    type_text(&mut o, "Look");
    press(&mut o, "esc");
    press(&mut o, "a");
    let listing_id = o.dispatcher.last_file_listing().unwrap().0;
    list_files(&mut o, listing_id, vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    press(&mut o, "h");

    fail_send(&mut o, "SEND_ATTACHMENT_FAILED");

    assert_eq!(o.state().message_input().text(), "Look");
}

#[test]
fn file_over_the_upload_limit_is_reported_as_too_large() {
    let mut o = picker_listing(vec![entry("/d/huge.iso", 200)]);
    press(&mut o, "enter");

    fail_send(&mut o, crate::usecases::background::ATTACHMENT_TOO_LARGE);

    assert_eq!(o.state().open_chat().messages().len(), 1);
    assert_eq!(
        o.state().active_notification(),
        Some("File is too large to send")
    );
}

#[test]
fn failed_send_without_a_caption_leaves_a_newer_draft_alone() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    press(&mut o, "i");
    type_text(&mut o, "new draft");

    fail_send(&mut o, "SEND_ATTACHMENT_FAILED");

    assert_eq!(o.state().message_input().text(), "new draft");
}

#[test]
fn successful_send_keeps_the_pending_message_until_the_refresh() {
    let mut o = picker_listing(vec![entry("/d/report.pdf", 200)]);
    press(&mut o, "enter");
    let (chat_id, topic_id, attachment) = sent(&o);

    complete(
        &mut o,
        BackgroundTaskResult::AttachmentSent {
            chat_id,
            topic_id,
            attachment,
            result: Ok(()),
        },
    );

    assert_eq!(o.state().open_chat().messages().len(), 2);
    assert_eq!(o.state().active_notification(), None);
}
