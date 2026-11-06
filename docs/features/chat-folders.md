# Chat folders

Press `f` in the chat list to open the folder picker (rebindable as
`show_chat_folders`). It lists "All chats" (the main list) and the user's chat
folders in Telegram's order. `j`/`k` move the cursor, `Enter` switches the chat
list to the selected list, and `q`, `Esc` or `f` close the picker. If the user
has no folders, a "No chat folders" notification is shown instead. The chat list
panel title shows the active folder's name ("Chats" for the main list).

## TDLib integration

- **Folder list**: TDLib has no method that lists folders. It pushes
  `updateChatFolders` at startup and whenever folders change. The update loop
  forwards it as `TdLibUpdate::ChatFolders`, the chat updates monitor maps it to
  `ChatUpdate::ChatFoldersChanged`, and the orchestrator stores it in
  `ShellState`. That way the picker opens from memory, with no request.
  `main_chat_list_position` places "All chats" among the folders.
- **Folder chats**: the folder is a TDLib `chatListFolder`. The chat list fetch
  uses the same `loadChats` + `getChats` path as the main list, called with the
  active `ChatList` instead of `chatListMain`. Refreshes, pagination and
  update-driven reloads all fetch the active list.
- **Switching**: `ChatListState::switch_to` drops the current chats and shows
  `Loading` until the new list arrives. `ChatListLoaded` carries the list it was
  fetched for. A result for a list the user has already switched away from is
  discarded, and the pending refresh then reloads the active list.

## Out of scope

Pinned chats inside a folder, and falling back when the active folder is
deleted, are tracked as separate issues.
