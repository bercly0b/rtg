use crate::telegram::tdlib_client::TdLibError;
use crate::usecases::guided_auth::AuthBackendError;
use crate::usecases::list_chats::ListChatsSourceError;

use super::error_mapping::{
    map_list_chats_error, map_password_error, map_request_code_error, map_sign_in_error,
    map_tdlib_error, parse_flood_wait_seconds,
};
use super::messages::{enrich_same_chat_reply_info, reply_sender_name_for_message};

#[test]
fn parse_flood_wait_extracts_seconds() {
    assert_eq!(parse_flood_wait_seconds("flood_wait_67"), Some(67));
    assert_eq!(parse_flood_wait_seconds("FLOOD_WAIT_120"), Some(120));
    assert_eq!(parse_flood_wait_seconds("no flood here"), None);
    assert_eq!(parse_flood_wait_seconds("other error"), None);
}

#[test]
fn map_request_code_error_detects_invalid_phone() {
    let error = TdLibError::Request {
        code: 400,
        message: "PHONE_NUMBER_INVALID".to_owned(),
    };
    assert_eq!(
        map_request_code_error(error),
        AuthBackendError::InvalidPhone
    );
}

#[test]
fn map_sign_in_error_detects_invalid_code() {
    let error = TdLibError::Request {
        code: 400,
        message: "PHONE_CODE_INVALID".to_owned(),
    };
    assert_eq!(map_sign_in_error(error), AuthBackendError::InvalidCode);
}

#[test]
fn map_password_error_detects_wrong_password() {
    let error = TdLibError::Request {
        code: 400,
        message: "PASSWORD_HASH_INVALID".to_owned(),
    };
    assert_eq!(map_password_error(error), AuthBackendError::WrongPassword);
}

#[test]
fn map_flood_wait_in_request_code() {
    let error = TdLibError::Request {
        code: 429,
        message: "FLOOD_WAIT_300".to_owned(),
    };
    assert_eq!(
        map_request_code_error(error),
        AuthBackendError::FloodWait { seconds: 300 }
    );
}

#[test]
fn map_list_chats_error_returns_unavailable_for_generic_error() {
    let error = TdLibError::Request {
        code: 500,
        message: "Internal Server Error".to_owned(),
    };
    assert_eq!(
        map_list_chats_error(error),
        ListChatsSourceError::Unavailable,
    );
}

#[test]
fn map_list_chats_error_returns_unauthorized_for_auth_error() {
    let error = TdLibError::Request {
        code: 401,
        message: "Unauthorized".to_owned(),
    };
    assert_eq!(
        map_list_chats_error(error),
        ListChatsSourceError::Unauthorized,
    );
}

#[test]
fn map_tdlib_error_maps_request_to_transient() {
    let error = TdLibError::Request {
        code: 400,
        message: "BAD_REQUEST".to_owned(),
    };
    assert_eq!(
        map_tdlib_error(error),
        AuthBackendError::Transient {
            code: "AUTH_BACKEND_UNAVAILABLE",
            message: "BAD_REQUEST".to_owned(),
        }
    );
}

// ── enrich_same_chat_reply_info: same-chat fallback for missing reply targets ──

mod reply_enrichment {
    use super::*;
    use crate::domain::message::{Message, MessageMedia, MessageStatus, ReplyInfo};

    fn empty_message_text() -> tdlib_rs::types::FormattedText {
        tdlib_rs::types::FormattedText {
            text: String::new(),
            entities: vec![],
        }
    }

    fn raw_reply_message(
        id: i64,
        chat_id: i64,
        replied_message_id: i64,
    ) -> tdlib_rs::types::Message {
        tdlib_rs::types::Message {
            id,
            sender_id: tdlib_rs::enums::MessageSender::User(tdlib_rs::types::MessageSenderUser {
                user_id: 1,
            }),
            chat_id,
            sending_state: None,
            scheduling_state: None,
            is_outgoing: false,
            is_pinned: false,
            is_from_offline: false,
            can_be_saved: true,
            has_timestamped_media: false,
            is_channel_post: false,
            is_paid_star_suggested_post: false,
            is_paid_ton_suggested_post: false,
            contains_unread_mention: false,
            date: 0,
            edit_date: 0,
            forward_info: None,
            import_info: None,
            interaction_info: None,
            unread_reactions: vec![],
            fact_check: None,
            suggested_post_info: None,
            // TDLib leaves origin/content empty for same-chat replies; the
            // client must resolve them itself.
            reply_to: Some(tdlib_rs::enums::MessageReplyTo::Message(
                tdlib_rs::types::MessageReplyToMessage {
                    chat_id,
                    message_id: replied_message_id,
                    quote: None,
                    checklist_task_id: 0,
                    origin: None,
                    origin_send_date: 0,
                    content: None,
                },
            )),
            topic_id: None,
            self_destruct_type: None,
            self_destruct_in: 0.0,
            auto_delete_in: 0.0,
            via_bot_user_id: 0,
            sender_business_bot_user_id: 0,
            sender_boost_count: 0,
            paid_message_star_count: 0,
            author_signature: String::new(),
            media_album_id: 0,
            effect_id: 0,
            restriction_info: None,
            summary_language_code: String::new(),
            content: tdlib_rs::enums::MessageContent::MessageText(tdlib_rs::types::MessageText {
                text: empty_message_text(),
                link_preview: None,
                link_preview_options: None,
            }),
            reply_markup: None,
        }
    }

    fn domain_message(id: i64, text: &str, reply_to: Option<ReplyInfo>) -> Message {
        Message {
            id,
            sender_name: "Sender".to_owned(),
            text: text.to_owned(),
            timestamp_ms: 0,
            is_outgoing: false,
            media: MessageMedia::None,
            status: MessageStatus::Delivered,
            file_info: None,
            call_info: None,
            reply_to,
            forward_info: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: false,
        }
    }

    fn empty_reply() -> ReplyInfo {
        ReplyInfo {
            sender_name: String::new(),
            text: String::new(),
            is_outgoing: false,
        }
    }

    /// Reproduces the bug: when the reply target is older than the loaded
    /// chat history page, the reply preview ends up with empty fields and
    /// renders as an empty bar in the UI.
    #[test]
    fn missing_reply_target_yields_empty_preview_without_fallback() {
        let raw = vec![raw_reply_message(2, 100, 1)];
        let mut messages = vec![domain_message(2, "reply body", Some(empty_reply()))];

        enrich_same_chat_reply_info(&raw, &mut messages, |_, _| None);

        let reply = messages[0]
            .reply_to
            .as_ref()
            .expect("message should still report it is a reply");
        assert!(reply.sender_name.is_empty());
        assert!(reply.text.is_empty());
    }

    #[test]
    fn external_lookup_fills_missing_reply_target() {
        let raw = vec![raw_reply_message(2, 100, 1)];
        let mut messages = vec![domain_message(2, "reply body", Some(empty_reply()))];

        let mut calls = Vec::new();
        enrich_same_chat_reply_info(&raw, &mut messages, |chat_id, message_id| {
            calls.push((chat_id, message_id));
            Some(("Alice".to_owned(), "original text".to_owned(), true))
        });

        assert_eq!(calls, vec![(100, 1)]);
        let reply = messages[0].reply_to.as_ref().unwrap();
        assert_eq!(reply.sender_name, "Alice");
        assert_eq!(reply.text, "original text");
        assert!(reply.is_outgoing);
    }

    #[test]
    fn batch_match_short_circuits_external_lookup() {
        let mut original_raw = raw_reply_message(1, 100, 0);
        original_raw.reply_to = None;
        let raw = vec![original_raw, raw_reply_message(2, 100, 1)];

        let mut messages = vec![
            domain_message(1, "original message", None),
            domain_message(2, "reply body", Some(empty_reply())),
        ];

        let mut external_called = false;
        enrich_same_chat_reply_info(&raw, &mut messages, |_, _| {
            external_called = true;
            None
        });

        assert!(
            !external_called,
            "external lookup must not be called when target is in batch"
        );
        let reply = messages[1].reply_to.as_ref().unwrap();
        assert_eq!(reply.sender_name, "Sender");
        assert_eq!(reply.text, "original message");
    }

    #[test]
    fn external_lookup_skipped_when_reply_already_populated() {
        // Cross-chat reply: TDLib already gave us sender + text.
        let raw = vec![raw_reply_message(2, 100, 999)];
        let prefilled = ReplyInfo {
            sender_name: "Bob".to_owned(),
            text: "from another chat".to_owned(),
            is_outgoing: false,
        };
        let mut messages = vec![domain_message(2, "reply body", Some(prefilled))];

        let mut external_called = false;
        enrich_same_chat_reply_info(&raw, &mut messages, |_, _| {
            external_called = true;
            None
        });

        assert!(
            !external_called,
            "external lookup must not be called when reply already has data"
        );
        let reply = messages[0].reply_to.as_ref().unwrap();
        assert_eq!(reply.sender_name, "Bob");
        assert_eq!(reply.text, "from another chat");
    }

    #[test]
    fn external_lookup_returning_none_leaves_fields_empty() {
        let raw = vec![raw_reply_message(2, 100, 1)];
        let mut messages = vec![domain_message(2, "reply body", Some(empty_reply()))];

        enrich_same_chat_reply_info(&raw, &mut messages, |_, _| None);

        let reply = messages[0].reply_to.as_ref().unwrap();
        assert!(reply.sender_name.is_empty());
        assert!(reply.text.is_empty());
    }
}

#[test]
fn reply_sender_name_for_outgoing_message_is_you() {
    let message = crate::domain::message::Message {
        id: 1,
        sender_name: "My Real Name".to_owned(),
        text: "hello".to_owned(),
        timestamp_ms: 0,
        is_outgoing: true,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    };

    assert_eq!(reply_sender_name_for_message(&message), "You");
}
