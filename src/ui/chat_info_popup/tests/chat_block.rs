//! The Chat block every template ends with.

use super::*;
use crate::domain::chat_info_state::{ChannelInfo, MemberRole};
use crate::domain::message_info_state::format_unix_timestamp;

fn chat_rows(chat: ChatBlockInfo) -> Vec<String> {
    let channel = ChannelInfo {
        link: None,
        discussion: None,
        signatures: None,
        has_protected_content: false,
        admins: None,
        badges: vec![],
        description: None,
        role: Some(MemberRole::Member),
        joined: None,
    };
    let lines = rendered_lines(&loaded_with_chat(
        "1 subscriber",
        ChatInfoKind::Channel(channel),
        chat,
    ));
    let start = lines.iter().position(|line| line == "Chat").unwrap();
    lines[start + 1..start + 5].to_vec()
}

#[test]
fn chat_block_is_last_before_the_footer() {
    let channel = ChannelInfo {
        link: None,
        discussion: None,
        signatures: None,
        has_protected_content: false,
        admins: None,
        badges: vec![],
        description: None,
        role: None,
        joined: None,
    };
    let lines = rendered_lines(&loaded("1 subscriber", ChatInfoKind::Channel(channel)));

    assert_eq!(lines[lines.len() - 8..], chat_block_and_footer());
}

#[test]
fn chat_block_shows_folders_mute_end_and_auto_delete_period() {
    let until = 1_794_160_800;
    let rows = chat_rows(ChatBlockInfo {
        chat_id: -1_001_234_567_890,
        folders: vec!["Work".into(), "Friends".into(), "Archive".into()],
        muted: Some(MuteState::Until(until)),
        auto_delete_seconds: 7 * 86_400,
    });

    assert_eq!(
        rows,
        vec![
            row("Chat ID", "-1001234567890"),
            row("Folders", "Work, Friends, Archive"),
            row("Muted", &format!("until {}", format_unix_timestamp(until))),
            row("Auto-delete", "1 week"),
        ]
    );
}

#[test]
fn chat_block_muted_forever_and_unknown() {
    let forever = chat_rows(ChatBlockInfo {
        muted: Some(MuteState::Forever),
        auto_delete_seconds: 31 * 86_400,
        ..chat_block()
    });
    let unknown = chat_rows(ChatBlockInfo {
        muted: None,
        auto_delete_seconds: 86_400,
        ..chat_block()
    });

    assert_eq!(
        forever[2..],
        [row("Muted", "yes"), row("Auto-delete", "1 month")]
    );
    assert_eq!(
        unknown[2..],
        [row("Muted", "—"), row("Auto-delete", "1 day")]
    );
}
