mod add_remove;
mod basic_ops;
mod limits;
mod lru_eviction;

use super::*;
use crate::domain::message::{Message, MessageMedia, MessageStatus};

fn msg(id: i64, text: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
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

fn msg_with_ts(id: i64, text: &str, timestamp_ms: i64) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms,
        is_outgoing: false,
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
