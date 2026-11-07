//! The Chat block every template ends with.

use tdlib_rs::enums::{ChatList, NotificationSettingsScope};
use tdlib_rs::types::ChatListFolder;

use super::*;
use crate::domain::chat_folder::ChatFolder;
use crate::domain::chat_info_state::{ChatBlockInfo, MuteState};

/// TDLib's ID for supergroup 70: `-100` followed by the supergroup ID.
const CHAT_ID: i64 = -10070;

fn resolver_with_channel(chat: Chat) -> FakeResolver {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(chat.id, chat);
    resolver
        .supergroup_full_infos
        .insert(70, supergroup_full_info(1, ""));
    resolver
}

fn channel_chat() -> Chat {
    supergroup_chat(CHAT_ID, 70, true)
}

fn folder(id: i32, title: &str) -> ChatFolder {
    ChatFolder {
        id,
        title: title.to_owned(),
    }
}

fn in_folder(chat_folder_id: i32) -> ChatList {
    ChatList::Folder(ChatListFolder { chat_folder_id })
}

#[test]
fn chat_block_lists_folders_in_the_users_order_then_the_archive() {
    let mut chat = channel_chat();
    chat.chat_lists = vec![ChatList::Archive, in_folder(7), in_folder(3), in_folder(99)];
    chat.message_auto_delete_time = 86_400;
    let resolver = resolver_with_channel(chat);
    let folders = ChatFolders {
        folders: vec![folder(3, "Work"), folder(5, "Family"), folder(7, "Friends")],
        main_list_position: 0,
    };

    let info = resolve_with_folders(&resolver, CHAT_ID, ChatType::Channel, folders).unwrap();

    assert_eq!(
        info.chat,
        ChatBlockInfo {
            chat_id: CHAT_ID,
            folders: vec!["Work".into(), "Friends".into(), "Archive".into()],
            muted: Some(MuteState::Unmuted),
            auto_delete_seconds: 86_400,
        }
    );
}

#[test]
fn chat_outside_folders_has_none() {
    let mut chat = channel_chat();
    chat.chat_lists = vec![ChatList::Main];
    let resolver = resolver_with_channel(chat);

    let info = resolve(&resolver, CHAT_ID, ChatType::Channel).unwrap();

    assert_eq!(info.chat.folders, Vec::<String>::new());
}

#[test]
fn muted_follows_the_chats_own_setting() {
    let cases = [
        (0, MuteState::Unmuted),
        (3600, MuteState::Until(now().timestamp() + 3600)),
        (i32::MAX - 1_800_000_000, MuteState::Forever),
    ];
    for (mute_for, expected) in cases {
        let mut chat = channel_chat();
        chat.notification_settings.mute_for = mute_for;
        let resolver = resolver_with_channel(chat);

        let info = resolve(&resolver, CHAT_ID, ChatType::Channel).unwrap();

        assert_eq!(info.chat.muted, Some(expected), "mute_for {mute_for}");
        assert!(!resolver.requested("getScopeNotificationSettings"));
    }
}

#[test]
fn muted_falls_back_to_the_settings_of_the_chats_scope() {
    let mut chat = channel_chat();
    chat.notification_settings.use_default_mute_for = true;
    let mut resolver = resolver_with_channel(chat);
    resolver.scope_mute_for = vec![
        (NotificationSettingsScope::PrivateChats, 0),
        (
            NotificationSettingsScope::ChannelChats,
            i32::MAX - 1_800_000_000,
        ),
    ];

    let info = resolve(&resolver, CHAT_ID, ChatType::Channel).unwrap();

    assert_eq!(info.chat.muted, Some(MuteState::Forever));
}

#[test]
fn scope_settings_failure_leaves_muted_unknown() {
    let mut chat = channel_chat();
    chat.notification_settings.use_default_mute_for = true;
    let resolver = resolver_with_channel(chat);

    let info = resolve(&resolver, CHAT_ID, ChatType::Channel).unwrap();

    assert_eq!(info.chat.muted, None);
}
