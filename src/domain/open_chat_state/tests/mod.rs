mod messages;
mod navigation;
mod state_transitions;

use super::*;
use crate::domain::chat::ChatType;

fn message(id: i64, text: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        forward_info: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}
