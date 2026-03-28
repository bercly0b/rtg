mod chat_list;
mod chat_list_item;
mod messages_panel;
mod status_line;
mod text_utils;

use ratatui::text::Line;

use crate::domain::chat::{ChatSummary, ChatType, OutgoingReadStatus};

pub(super) fn line_to_string(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

pub(super) fn chat(
    chat_id: i64,
    title: &str,
    unread_count: u32,
    preview: Option<&str>,
) -> ChatSummary {
    chat_with_pinned(chat_id, title, unread_count, preview, false)
}

pub(super) fn chat_with_pinned(
    chat_id: i64,
    title: &str,
    unread_count: u32,
    preview: Option<&str>,
    is_pinned: bool,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

pub(super) fn group_chat(
    chat_id: i64,
    title: &str,
    preview: Option<&str>,
    sender: Option<&str>,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Group,
        last_message_sender: sender.map(ToOwned::to_owned),
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

pub(super) fn group_chat_outgoing(
    chat_id: i64,
    title: &str,
    preview: Option<&str>,
    sender: Option<&str>,
    is_read: bool,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Group,
        last_message_sender: sender.map(ToOwned::to_owned),
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus {
            is_outgoing: true,
            is_read,
        },
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

pub(super) fn private_chat_online(
    chat_id: i64,
    title: &str,
    preview: Option<&str>,
    is_online: bool,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: Some(is_online),
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

pub(super) fn private_chat_outgoing(
    chat_id: i64,
    title: &str,
    preview: Option<&str>,
    is_read: bool,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus {
            is_outgoing: true,
            is_read,
        },
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

pub(super) fn channel_chat_outgoing(
    chat_id: i64,
    title: &str,
    preview: Option<&str>,
    is_read: bool,
) -> ChatSummary {
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: preview.map(ToOwned::to_owned),
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Channel,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus {
            is_outgoing: true,
            is_read,
        },
        last_message_id: None,
        unread_reaction_count: 0,
    }
}
