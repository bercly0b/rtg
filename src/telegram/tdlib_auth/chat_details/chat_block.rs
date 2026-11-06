//! The Chat block every info template ends with.

use chrono::{DateTime, Local};
use tdlib_rs::enums::{ChatList, ChatType as TdChatType, NotificationSettingsScope};
use tdlib_rs::types::Chat;

use crate::domain::chat_folder::{ChatFolders, ARCHIVE_CHAT_LIST_TITLE};
use crate::domain::chat_info_state::{ChatBlockInfo, MuteState};

use super::{optional, ChatInfoResolver};

/// TDLib turns any mute longer than this into "forever".
const MAX_TIMED_MUTE_SECONDS: i32 = 366 * 24 * 60 * 60;

pub(super) fn resolve(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    folders: &ChatFolders,
    now: DateTime<Local>,
) -> ChatBlockInfo {
    ChatBlockInfo {
        chat_id: chat.id,
        folders: folder_titles(&chat.chat_lists, folders),
        muted: mute_for(resolver, chat).map(|mute_for| mute_state(mute_for, now)),
        auto_delete_seconds: chat.message_auto_delete_time,
    }
}

/// Folders in the user's order, then the archive.
fn folder_titles(chat_lists: &[ChatList], folders: &ChatFolders) -> Vec<String> {
    let in_folder = |folder_id: i32| {
        chat_lists
            .iter()
            .any(|list| matches!(list, ChatList::Folder(f) if f.chat_folder_id == folder_id))
    };
    let mut titles: Vec<String> = folders
        .folders
        .iter()
        .filter(|folder| in_folder(folder.id))
        .map(|folder| folder.title.clone())
        .collect();
    if chat_lists.contains(&ChatList::Archive) {
        titles.push(ARCHIVE_CHAT_LIST_TITLE.to_owned());
    }
    titles
}

fn mute_for(resolver: &dyn ChatInfoResolver, chat: &Chat) -> Option<i32> {
    let settings = &chat.notification_settings;
    if !settings.use_default_mute_for {
        return Some(settings.mute_for);
    }
    let scope = notification_scope(&chat.r#type);
    optional(
        "getScopeNotificationSettings",
        resolver.get_scope_notification_settings(scope),
    )
    .map(|scope_settings| scope_settings.mute_for)
}

fn notification_scope(td_type: &TdChatType) -> NotificationSettingsScope {
    match td_type {
        TdChatType::Private(_) | TdChatType::Secret(_) => NotificationSettingsScope::PrivateChats,
        TdChatType::Supergroup(sg) if sg.is_channel => NotificationSettingsScope::ChannelChats,
        TdChatType::BasicGroup(_) | TdChatType::Supergroup(_) => {
            NotificationSettingsScope::GroupChats
        }
    }
}

fn mute_state(mute_for: i32, now: DateTime<Local>) -> MuteState {
    if mute_for <= 0 {
        MuteState::Unmuted
    } else if mute_for > MAX_TIMED_MUTE_SECONDS {
        MuteState::Forever
    } else {
        MuteState::Until(now.timestamp() + i64::from(mute_for))
    }
}
