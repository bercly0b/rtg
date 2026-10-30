//! Chat folders and the folder picker popup.

use super::selectable_list::SelectableList;

/// Folder picker title of the main chat list, which TDLib does not name.
pub const MAIN_CHAT_LIST_TITLE: &str = "All chats";

/// Which TDLib chat list the chat list panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatListKind {
    #[default]
    Main,
    Folder(i32),
}

/// A user-defined chat folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatFolder {
    pub id: i32,
    pub title: String,
}

/// The user's chat folders as last pushed by TDLib, in the user's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatFolders {
    pub folders: Vec<ChatFolder>,
    /// Position of the main chat list among the folders, 0-based.
    pub main_list_position: usize,
}

impl ChatFolders {
    pub fn title(&self, folder_id: i32) -> Option<&str> {
        self.folders
            .iter()
            .find(|folder| folder.id == folder_id)
            .map(|folder| folder.title.as_str())
    }
}

/// A row of the folder picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatFolderEntry {
    pub list: ChatListKind,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatFolderPickerState {
    entries: SelectableList<ChatFolderEntry>,
}

impl ChatFolderPickerState {
    /// Lists the main chat list and every folder in the user's order, with
    /// the cursor on `active`. `None` when the user has no chat folders.
    pub fn new(folders: &ChatFolders, active: ChatListKind) -> Option<Self> {
        if folders.folders.is_empty() {
            return None;
        }

        let mut entries: Vec<ChatFolderEntry> = folders
            .folders
            .iter()
            .map(|folder| ChatFolderEntry {
                list: ChatListKind::Folder(folder.id),
                title: folder.title.clone(),
            })
            .collect();
        let main_position = folders.main_list_position.min(entries.len());
        entries.insert(
            main_position,
            ChatFolderEntry {
                list: ChatListKind::Main,
                title: MAIN_CHAT_LIST_TITLE.to_owned(),
            },
        );

        let active_index = entries.iter().position(|entry| entry.list == active);
        let mut list = SelectableList::default();
        list.replace(entries, active_index);
        Some(Self { entries: list })
    }

    pub fn entries(&self) -> &[ChatFolderEntry] {
        self.entries.items()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.entries.selected_index()
    }

    pub fn selected(&self) -> Option<&ChatFolderEntry> {
        self.entries.selected()
    }

    pub fn select_next(&mut self) {
        self.entries.select_next();
    }

    pub fn select_previous(&mut self) {
        self.entries.select_previous();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folders(main_list_position: usize) -> ChatFolders {
        ChatFolders {
            folders: vec![
                ChatFolder {
                    id: 3,
                    title: "Work".to_owned(),
                },
                ChatFolder {
                    id: 7,
                    title: "Family".to_owned(),
                },
            ],
            main_list_position,
        }
    }

    fn lists(picker: &ChatFolderPickerState) -> Vec<ChatListKind> {
        picker.entries().iter().map(|entry| entry.list).collect()
    }

    #[test]
    fn no_picker_without_folders() {
        assert_eq!(
            ChatFolderPickerState::new(&ChatFolders::default(), ChatListKind::Main),
            None
        );
    }

    #[test]
    fn main_list_comes_first_by_default() {
        let picker = ChatFolderPickerState::new(&folders(0), ChatListKind::Main).unwrap();

        assert_eq!(
            lists(&picker),
            vec![
                ChatListKind::Main,
                ChatListKind::Folder(3),
                ChatListKind::Folder(7)
            ]
        );
        assert_eq!(picker.entries()[0].title, MAIN_CHAT_LIST_TITLE);
        assert_eq!(picker.entries()[1].title, "Work");
    }

    #[test]
    fn main_list_is_placed_at_its_position() {
        let picker = ChatFolderPickerState::new(&folders(1), ChatListKind::Main).unwrap();

        assert_eq!(
            lists(&picker),
            vec![
                ChatListKind::Folder(3),
                ChatListKind::Main,
                ChatListKind::Folder(7)
            ]
        );
    }

    #[test]
    fn main_list_position_past_the_end_goes_last() {
        let picker = ChatFolderPickerState::new(&folders(10), ChatListKind::Main).unwrap();

        assert_eq!(lists(&picker).last(), Some(&ChatListKind::Main));
    }

    #[test]
    fn cursor_starts_on_active_list() {
        let picker = ChatFolderPickerState::new(&folders(0), ChatListKind::Folder(7)).unwrap();

        assert_eq!(picker.selected_index(), Some(2));
        assert_eq!(
            picker.selected().map(|entry| entry.list),
            Some(ChatListKind::Folder(7))
        );
    }

    #[test]
    fn cursor_falls_back_to_first_entry_for_unknown_active_folder() {
        let picker = ChatFolderPickerState::new(&folders(1), ChatListKind::Folder(99)).unwrap();

        assert_eq!(picker.selected_index(), Some(0));
    }

    #[test]
    fn cursor_moves_and_clamps() {
        let mut picker = ChatFolderPickerState::new(&folders(0), ChatListKind::Main).unwrap();

        picker.select_previous();
        assert_eq!(picker.selected_index(), Some(0));

        picker.select_next();
        picker.select_next();
        picker.select_next();
        assert_eq!(picker.selected_index(), Some(2));

        picker.select_previous();
        assert_eq!(picker.selected_index(), Some(1));
    }

    #[test]
    fn title_looks_up_folder_by_id() {
        let folders = folders(0);

        assert_eq!(folders.title(7), Some("Family"));
        assert_eq!(folders.title(99), None);
    }
}
