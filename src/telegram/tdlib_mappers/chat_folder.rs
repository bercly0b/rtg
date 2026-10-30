use tdlib_rs::enums::ChatList as TdChatList;
use tdlib_rs::types::{ChatFolderInfo, ChatListFolder};

use crate::domain::chat_folder::{ChatFolder, ChatFolders, ChatListKind};

/// Maps the payload of TDLib's `updateChatFolders` to domain chat folders.
pub fn map_chat_folders(folders: &[ChatFolderInfo], main_chat_list_position: i32) -> ChatFolders {
    ChatFolders {
        folders: folders
            .iter()
            .map(|folder| ChatFolder {
                id: folder.id,
                title: folder.name.text.text.clone(),
            })
            .collect(),
        main_list_position: usize::try_from(main_chat_list_position).unwrap_or(0),
    }
}

pub fn map_chat_list_kind(list: ChatListKind) -> TdChatList {
    match list {
        ChatListKind::Main => TdChatList::Main,
        ChatListKind::Folder(chat_folder_id) => {
            TdChatList::Folder(ChatListFolder { chat_folder_id })
        }
    }
}
