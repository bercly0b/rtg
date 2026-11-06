use tdlib_rs::enums::ChatList as TdChatList;
use tdlib_rs::types::{
    ChatFolderIcon, ChatFolderInfo, ChatFolderName, ChatListFolder, FormattedText,
};

use crate::domain::chat_folder::{ChatFolder, ChatListKind};
use crate::telegram::tdlib_mappers::{map_chat_folders, map_chat_list_kind};

fn folder_info(id: i32, name: &str) -> ChatFolderInfo {
    ChatFolderInfo {
        id,
        name: ChatFolderName {
            text: FormattedText {
                text: name.to_owned(),
                entities: vec![],
            },
            animate_custom_emoji: false,
        },
        icon: ChatFolderIcon {
            name: "Custom".to_owned(),
        },
        color_id: -1,
        is_shareable: false,
        has_my_invite_links: false,
    }
}

#[test]
fn chat_folders_keep_tdlib_order_and_names() {
    let folders = map_chat_folders(&[folder_info(3, "Work"), folder_info(7, "Family")], 1);

    assert_eq!(
        folders.folders,
        vec![
            ChatFolder {
                id: 3,
                title: "Work".to_owned()
            },
            ChatFolder {
                id: 7,
                title: "Family".to_owned()
            },
        ]
    );
    assert_eq!(folders.main_list_position, 1);
}

#[test]
fn negative_main_list_position_falls_back_to_first() {
    let folders = map_chat_folders(&[folder_info(3, "Work")], -1);

    assert_eq!(folders.main_list_position, 0);
}

#[test]
fn chat_list_kind_maps_to_tdlib_chat_list() {
    assert_eq!(map_chat_list_kind(ChatListKind::Main), TdChatList::Main);
    assert_eq!(
        map_chat_list_kind(ChatListKind::Folder(3)),
        TdChatList::Folder(ChatListFolder { chat_folder_id: 3 })
    );
}
