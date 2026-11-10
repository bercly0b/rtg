# Chat folders

Press `f` in the chat list to open the folder picker (rebindable as
`show_chat_folders`). It lists "All chats" (the main list) and the user's chat
folders in Telegram's order, then "Archive" as the last row (see
[chat-archive.md](chat-archive.md)). It opens even when the user has no
folders. `j`/`k` move the cursor, `Enter` switches the chat list to the selected
list, and `q`, `Esc` or `f` close the picker. The chat list panel title shows
the active folder's name ("Chats" for the main list).

Every row with unread chats shows its unread chat count as a dimmed `[N]`,
right-aligned in a column: the number of chats in that list with something
unread, muted or not. A row with no unread chats, or whose count TDLib has not
reported yet, shows no badge. The active list shows its count too, and an open
picker follows count changes.

## TDLib integration

- **Folder list**: TDLib has no method that lists folders. It pushes
  `updateChatFolders` at startup and whenever folders change. The update loop
  forwards it as `TdLibUpdate::ChatFolders`, the chat updates monitor maps it to
  `ChatUpdate::ChatFoldersChanged`, and the orchestrator stores it in
  `ShellState`. That way the picker opens from memory, with no request.
  `main_chat_list_position` places "All chats" among the folders.
- **Unread chat counts**: TDLib pushes `updateUnreadChatCount` for a chat list
  at startup and whenever its counts change (only with
  `use_message_database`, which rtg enables). The update loop forwards
  `unread_count` as `TdLibUpdate::UnreadChatCount`, the chat updates monitor
  maps it to `ChatUpdate::UnreadChatCountChanged`, and the orchestrator stores
  it per chat list in `ShellState`. The picker reads the counts at render time,
  so it needs no request and no rebuild.
- **Folder chats**: the folder is a TDLib `chatListFolder`. The chat list fetch
  uses the same `loadChats` + `getChats` path as the main list, called with the
  active `ChatList` instead of `chatListMain`. Refreshes, pagination and
  update-driven reloads all fetch the active list.
- **Switching**: `ChatListState::switch_to` drops the current chats and shows
  `Loading` until the new list arrives. `ChatListLoaded` carries the list it was
  fetched for. A result for a list the user has already switched away from is
  discarded, and the pending refresh then reloads the active list.
- **Pinned chats**: TDLib keeps a separate `ChatPosition` per chat list, each
  with its own `is_pinned`. The chat list fetch passes the requested list down
  to `map_chat_to_summary`, which marks a chat pinned only by its position in
  that list. A folder's "Pinned" section therefore shows the chats pinned in
  the folder, not the ones pinned in the main list.
- **Removed folder**: when an `updateChatFolders` no longer lists the active
  folder (deleted on another device, for example), `ShellState::set_chat_folders`
  switches the chat list to the main list, and the orchestrator reloads it the
  same way as a manual switch. An open picker is rebuilt from the new folders,
  so it never offers a removed folder. Its cursor stays on the same list, or
  moves to the active list if that one is gone too. With no folders left, the
  picker stays open with "All chats" and "Archive". The archive always exists,
  so a folder update never moves the chat list out of it.
