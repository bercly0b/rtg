use tdlib_rs::enums::MessageContent;

use crate::domain::message::MessageMedia;
use crate::telegram::tdlib_mappers::{
    extract_forward_info, extract_message_media, extract_message_text, map_tdlib_message_to_domain,
};

use super::{make_test_file, make_test_message};

#[test]
fn normalize_preview_text_collapses_whitespace() {
    use crate::telegram::tdlib_mappers::message::normalize_preview_text;

    assert_eq!(
        normalize_preview_text("hello  world"),
        Some("hello world".to_owned())
    );
    assert_eq!(
        normalize_preview_text("  multiple   spaces  "),
        Some("multiple spaces".to_owned())
    );
}

#[test]
fn normalize_preview_text_returns_none_for_empty() {
    use crate::telegram::tdlib_mappers::message::normalize_preview_text;

    assert_eq!(normalize_preview_text(""), None);
    assert_eq!(normalize_preview_text("   "), None);
}

#[test]
fn extract_message_media_identifies_text_as_none() {
    let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
        text: tdlib_rs::types::FormattedText {
            text: "Hello".to_owned(),
            entities: vec![],
        },
        link_preview: None,
        link_preview_options: None,
    });
    assert_eq!(extract_message_media(&content), MessageMedia::None);
}

#[test]
fn extract_message_media_identifies_photo() {
    let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
        photo: tdlib_rs::types::Photo {
            minithumbnail: None,
            sizes: vec![],
            has_stickers: false,
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        show_caption_above_media: false,
        has_spoiler: false,
        is_secret: false,
    });
    assert_eq!(extract_message_media(&content), MessageMedia::Photo);
}

#[test]
fn extract_message_media_identifies_voice() {
    let content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
        voice_note: tdlib_rs::types::VoiceNote {
            duration: 10,
            waveform: String::new(),
            mime_type: "audio/ogg".to_owned(),
            speech_recognition_result: None,
            voice: tdlib_rs::types::File {
                id: 1,
                size: 1000,
                expected_size: 1000,
                local: tdlib_rs::types::LocalFile {
                    path: String::new(),
                    can_be_downloaded: false,
                    can_be_deleted: false,
                    is_downloading_active: false,
                    is_downloading_completed: false,
                    download_offset: 0,
                    downloaded_prefix_size: 0,
                    downloaded_size: 0,
                },
                remote: tdlib_rs::types::RemoteFile {
                    id: String::new(),
                    unique_id: String::new(),
                    is_uploading_active: false,
                    is_uploading_completed: false,
                    uploaded_size: 0,
                },
            },
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        is_listened: false,
    });
    assert_eq!(extract_message_media(&content), MessageMedia::Voice);
}

#[test]
fn extract_message_text_returns_text_from_message_text() {
    let content = MessageContent::MessageText(tdlib_rs::types::MessageText {
        text: tdlib_rs::types::FormattedText {
            text: "Hello, world!".to_owned(),
            entities: vec![],
        },
        link_preview: None,
        link_preview_options: None,
    });
    assert_eq!(extract_message_text(&content), "Hello, world!");
}

#[test]
fn extract_message_text_returns_caption_from_photo() {
    let content = MessageContent::MessagePhoto(tdlib_rs::types::MessagePhoto {
        photo: tdlib_rs::types::Photo {
            minithumbnail: None,
            sizes: vec![],
            has_stickers: false,
        },
        caption: tdlib_rs::types::FormattedText {
            text: "Photo caption".to_owned(),
            entities: vec![],
        },
        show_caption_above_media: false,
        has_spoiler: false,
        is_secret: false,
    });
    assert_eq!(extract_message_text(&content), "Photo caption");
}

#[test]
fn map_tdlib_message_to_domain_creates_correct_message() {
    let td_message = make_test_message(123, "Hello from TDLib", false);
    let message = map_tdlib_message_to_domain(&td_message, "John Doe".to_owned(), None, None);

    assert_eq!(message.id, 123);
    assert_eq!(message.sender_name, "John Doe");
    assert_eq!(message.text, "Hello from TDLib");
    assert!(!message.is_outgoing);
    assert_eq!(message.media, MessageMedia::None);
}

#[test]
fn map_tdlib_message_to_domain_handles_outgoing() {
    let td_message = make_test_message(456, "My message", true);
    let message = map_tdlib_message_to_domain(&td_message, "Me".to_owned(), None, None);

    assert!(message.is_outgoing);
}

#[test]
fn map_tdlib_message_includes_file_info() {
    let mut td_msg = make_test_message(1, "", false);
    td_msg.content = MessageContent::MessageVoiceNote(tdlib_rs::types::MessageVoiceNote {
        voice_note: tdlib_rs::types::VoiceNote {
            duration: 3,
            waveform: String::new(),
            mime_type: "audio/ogg".to_owned(),
            speech_recognition_result: None,
            voice: make_test_file(7, "/tmp/v.ogg", true),
        },
        caption: tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        },
        is_listened: false,
    });

    let msg = map_tdlib_message_to_domain(&td_msg, "User".to_owned(), None, None);
    assert_eq!(msg.media, MessageMedia::Voice);
    let fi = msg.file_info.expect("voice message should have file_info");
    assert_eq!(fi.file_id, 7);
    assert_eq!(fi.local_path, Some("/tmp/v.ogg".to_owned()));
}

// ── reaction count tests ──

#[test]
fn message_without_interaction_info_has_zero_reactions() {
    let td_msg = make_test_message(1, "Hello", false);
    let msg = map_tdlib_message_to_domain(&td_msg, "User".to_owned(), None, None);
    assert_eq!(msg.reaction_count, 0);
}

#[test]
fn message_with_reactions_sums_total_counts() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.interaction_info = Some(tdlib_rs::types::MessageInteractionInfo {
        view_count: 0,
        forward_count: 0,
        reply_info: None,
        reactions: Some(tdlib_rs::types::MessageReactions {
            reactions: vec![
                tdlib_rs::types::MessageReaction {
                    r#type: tdlib_rs::enums::ReactionType::Emoji(
                        tdlib_rs::types::ReactionTypeEmoji {
                            emoji: "\u{1f44d}".to_owned(),
                        },
                    ),
                    total_count: 2,
                    is_chosen: false,
                    used_sender_id: None,
                    recent_sender_ids: vec![],
                },
                tdlib_rs::types::MessageReaction {
                    r#type: tdlib_rs::enums::ReactionType::Emoji(
                        tdlib_rs::types::ReactionTypeEmoji {
                            emoji: "\u{2764}".to_owned(),
                        },
                    ),
                    total_count: 1,
                    is_chosen: false,
                    used_sender_id: None,
                    recent_sender_ids: vec![],
                },
            ],
            are_tags: false,
            paid_reactors: vec![],
            can_get_added_reactions: false,
        }),
    });

    let msg = map_tdlib_message_to_domain(&td_msg, "User".to_owned(), None, None);
    assert_eq!(msg.reaction_count, 3);
}

// ── forward info extraction tests ──

#[test]
fn extract_forward_info_returns_none_when_no_forward() {
    let td_msg = make_test_message(1, "Hello", false);
    let result = extract_forward_info(&td_msg, |_| None, |_| None);
    assert!(result.is_none());
}

#[test]
fn extract_forward_info_from_user_origin() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::User(tdlib_rs::types::MessageOriginUser {
            sender_user_id: 42,
        }),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(
        &td_msg,
        |uid| {
            if uid == 42 {
                Some("Alice".to_owned())
            } else {
                None
            }
        },
        |_| None,
    );

    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "Alice");
}

#[test]
fn extract_forward_info_from_hidden_user() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::HiddenUser(
            tdlib_rs::types::MessageOriginHiddenUser {
                sender_name: "Hidden Person".to_owned(),
            },
        ),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(&td_msg, |_| None, |_| None);
    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "Hidden Person");
}

#[test]
fn extract_forward_info_from_channel_origin() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::Channel(tdlib_rs::types::MessageOriginChannel {
            chat_id: 999,
            message_id: 1,
            author_signature: "fallback".to_owned(),
        }),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(
        &td_msg,
        |_| None,
        |cid| {
            if cid == 999 {
                Some("News Channel".to_owned())
            } else {
                None
            }
        },
    );

    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "News Channel");
}

#[test]
fn extract_forward_info_channel_falls_back_to_signature() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::Channel(tdlib_rs::types::MessageOriginChannel {
            chat_id: 999,
            message_id: 1,
            author_signature: "Author Sig".to_owned(),
        }),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(&td_msg, |_| None, |_| None);
    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "Author Sig");
}

#[test]
fn extract_forward_info_from_chat_origin() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::Chat(tdlib_rs::types::MessageOriginChat {
            sender_chat_id: 555,
            author_signature: "Sig".to_owned(),
        }),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(
        &td_msg,
        |_| None,
        |cid| {
            if cid == 555 {
                Some("Group Chat".to_owned())
            } else {
                None
            }
        },
    );

    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "Group Chat");
}

#[test]
fn extract_forward_info_chat_falls_back_to_signature() {
    let mut td_msg = make_test_message(1, "Hello", false);
    td_msg.forward_info = Some(tdlib_rs::types::MessageForwardInfo {
        origin: tdlib_rs::enums::MessageOrigin::Chat(tdlib_rs::types::MessageOriginChat {
            sender_chat_id: 555,
            author_signature: "Chat Sig".to_owned(),
        }),
        date: 0,
        source: None,
        public_service_announcement_type: String::new(),
    });

    let result = extract_forward_info(&td_msg, |_| None, |_| None);
    let fwd = result.expect("should have forward_info");
    assert_eq!(fwd.sender_name, "Chat Sig");
}
