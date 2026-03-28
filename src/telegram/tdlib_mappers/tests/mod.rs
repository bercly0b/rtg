mod chat;
mod file_info;
mod message;
mod text_links;
mod user;

use tdlib_rs::enums::{MessageContent, MessageSender, UserStatus};
use tdlib_rs::types::{Message as TdMessage, User as TdUser};

/// Creates a minimal TdUser for testing.
pub(super) fn make_test_user(first_name: &str, last_name: &str) -> TdUser {
    TdUser {
        id: 1,
        first_name: first_name.to_owned(),
        last_name: last_name.to_owned(),
        usernames: None,
        phone_number: String::new(),
        status: UserStatus::Empty,
        profile_photo: None,
        accent_color_id: 0,
        background_custom_emoji_id: 0,
        upgraded_gift_colors: None,
        profile_accent_color_id: -1,
        profile_background_custom_emoji_id: 0,
        emoji_status: None,
        is_contact: false,
        is_mutual_contact: false,
        is_close_friend: false,
        verification_status: None,
        is_premium: false,
        is_support: false,
        restriction_info: None,
        active_story_state: None,
        restricts_new_chats: false,
        paid_message_star_count: 0,
        have_access: true,
        r#type: tdlib_rs::enums::UserType::Regular,
        language_code: String::new(),
        added_to_attachment_menu: false,
    }
}

/// Creates a minimal TdMessage for testing.
pub(super) fn make_test_message(id: i64, text: &str, is_outgoing: bool) -> TdMessage {
    TdMessage {
        id,
        sender_id: MessageSender::User(tdlib_rs::types::MessageSenderUser { user_id: 1 }),
        chat_id: 100,
        sending_state: None,
        scheduling_state: None,
        is_outgoing,
        is_pinned: false,
        is_from_offline: false,
        can_be_saved: true,
        has_timestamped_media: false,
        is_channel_post: false,
        is_paid_star_suggested_post: false,
        is_paid_ton_suggested_post: false,
        contains_unread_mention: false,
        date: 1609459200, // 2021-01-01 00:00:00 UTC
        edit_date: 0,
        forward_info: None,
        import_info: None,
        interaction_info: None,
        unread_reactions: vec![],
        fact_check: None,
        suggested_post_info: None,
        reply_to: None,
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
        content: MessageContent::MessageText(tdlib_rs::types::MessageText {
            text: tdlib_rs::types::FormattedText {
                text: text.to_owned(),
                entities: vec![],
            },
            link_preview: None,
            link_preview_options: None,
        }),
        reply_markup: None,
    }
}

pub(super) fn make_test_file(id: i32, path: &str, downloaded: bool) -> tdlib_rs::types::File {
    tdlib_rs::types::File {
        id,
        size: 1000,
        expected_size: 1000,
        local: tdlib_rs::types::LocalFile {
            path: path.to_owned(),
            can_be_downloaded: true,
            can_be_deleted: false,
            is_downloading_active: false,
            is_downloading_completed: downloaded,
            download_offset: 0,
            downloaded_prefix_size: 0,
            downloaded_size: if downloaded { 1000 } else { 0 },
        },
        remote: tdlib_rs::types::RemoteFile {
            id: String::new(),
            unique_id: String::new(),
            is_uploading_active: false,
            is_uploading_completed: false,
            uploaded_size: 0,
        },
    }
}

pub(super) fn make_formatted_text(
    text: &str,
    entities: Vec<tdlib_rs::types::TextEntity>,
) -> tdlib_rs::types::FormattedText {
    tdlib_rs::types::FormattedText {
        text: text.to_owned(),
        entities,
    }
}

pub(super) fn make_url_entity(offset: i32, length: i32) -> tdlib_rs::types::TextEntity {
    tdlib_rs::types::TextEntity {
        offset,
        length,
        r#type: tdlib_rs::enums::TextEntityType::Url,
    }
}

pub(super) fn make_text_url_entity(
    offset: i32,
    length: i32,
    url: &str,
) -> tdlib_rs::types::TextEntity {
    tdlib_rs::types::TextEntity {
        offset,
        length,
        r#type: tdlib_rs::enums::TextEntityType::TextUrl(tdlib_rs::types::TextEntityTypeTextUrl {
            url: url.to_owned(),
        }),
    }
}
