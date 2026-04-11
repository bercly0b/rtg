mod content_spans;
mod element_building;
mod forward;
mod rendering;
mod reply;
mod text_utils;

use crate::domain::message::{ForwardInfo, Message, MessageMedia, MessageStatus, ReplyInfo};

// Note: These timestamps are in UTC. Tests use Local timezone for conversion,
// so the displayed time may vary by timezone. However, the date grouping logic
// (same day vs different day) should work correctly regardless of timezone.
const FEB_14_2026_10AM: i64 = 1771059600000; // 2026-02-14 10:00:00 UTC
const FEB_15_2026_1PM: i64 = 1771156800000; // 2026-02-15 13:00:00 UTC

fn msg(id: i64, sender: &str, text: &str, ts_ms: i64, outgoing: bool) -> Message {
    Message {
        id,
        sender_name: sender.to_owned(),
        text: text.to_owned(),
        timestamp_ms: ts_ms,
        is_outgoing: outgoing,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn msg_with_media(id: i64, sender: &str, text: &str, ts_ms: i64, media: MessageMedia) -> Message {
    Message {
        id,
        sender_name: sender.to_owned(),
        text: text.to_owned(),
        timestamp_ms: ts_ms,
        is_outgoing: false,
        media,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn msg_with_reply(
    id: i64,
    sender: &str,
    text: &str,
    ts_ms: i64,
    reply_sender: &str,
    reply_text: &str,
) -> Message {
    Message {
        id,
        sender_name: sender.to_owned(),
        text: text.to_owned(),
        timestamp_ms: ts_ms,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: Some(ReplyInfo {
            sender_name: reply_sender.to_owned(),
            text: reply_text.to_owned(),
            is_outgoing: false,
        }),
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

fn msg_with_forward(
    id: i64,
    sender: &str,
    text: &str,
    ts_ms: i64,
    forward_sender: &str,
) -> Message {
    Message {
        id,
        sender_name: sender.to_owned(),
        text: text.to_owned(),
        timestamp_ms: ts_ms,
        is_outgoing: false,
        media: MessageMedia::None,
        status: MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: Some(ForwardInfo {
            sender_name: forward_sender.to_owned(),
        }),
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}
