//! Type mappers from TDLib types to RTG domain types.
//!
//! This module provides conversion functions that map TDLib's rich type system
//! to RTG's simplified domain types for UI rendering.

mod chat;
mod file_info;
mod message;
mod text_links;
mod user;

// Re-exports consumed by sibling modules (`tdlib_auth`, `tdlib_client`, `chat_updates`)
// via `super::tdlib_mappers::*` paths.
#[allow(unused_imports)]
pub use chat::{map_chat_to_summary, map_chat_type};
#[allow(unused_imports)]
pub use file_info::extract_file_info;
#[allow(unused_imports)]
pub use message::{
    extract_message_media, extract_message_preview, extract_message_text, extract_reply_info,
    map_tdlib_message_to_domain, sum_reaction_counts,
};
#[allow(unused_imports)]
pub use user::{
    format_user_name, get_private_chat_user_id, get_sender_user_id, is_user_online,
    map_user_status_to_subtitle,
};

#[cfg(test)]
mod tests;
