mod call_metadata;
mod display;
mod file_metadata;
mod url;

use super::*;

pub(super) fn msg(text: &str, media: MessageMedia) -> Message {
    Message {
        id: 1,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms: 1000,
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
