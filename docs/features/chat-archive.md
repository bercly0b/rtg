# Chat archive

The archive is the chat list of chats moved out of the main list (see "Chat
lists" in [GLOSSARY.md](../../GLOSSARY.md)). It is not a folder: every chat is
in exactly one of the main list and the archive.

## Viewing the archive

The folder picker (`f`) always ends with an "Archive" row, even when the
archive is empty (it then opens as an empty list). Picking it shows the
archived chats; the chat list panel title is "Archive". The archive is left
the same way: `f` and another list. `h` does not leave it.

## Archiving and unarchiving

`A` in the chat list (rebindable as `toggle_chat_archive`) archives the
selected chat, or unarchives it when it is archived. There is no confirmation:
the action is undone by pressing `A` again.

- **Direction** follows the chat's own state, not the active list:
  `ChatSummary::is_archived` is `true` when the chat has a position in
  `chatListArchive`. So `A` works the same in the main list, the archive and
  folders.
- **Optimistic UI**: when the chat leaves the active list (archived from the
  main list, unarchived from the archive), its row disappears at once and the
  cursor moves to the next chat. In a folder the row stays and only its
  `is_archived` flips, so a second `A` undoes the first; the update-driven
  reload then decides whether the folder still shows the chat (it depends on
  the folder's `exclude_archived`). A "Chat archived" / "Chat unarchived"
  notification is shown in every case.
- **Failure**: if `addChatToList` fails, a "Failed to archive chat" /
  "Failed to unarchive chat" notification is shown and the active list is
  reloaded, which brings the row back.

## TDLib integration

- **Archive list**: `ChatListKind::Archive` maps to `chatListArchive`. Loading
  (`loadChats` + `getChats`), pagination, update-driven reloads and the Pinned
  section (positions are per list) work as for folders — see
  [chat-folders.md](chat-folders.md).
- **Moving a chat**: archiving is `addChatToList(chat_id, chatListArchive)`,
  unarchiving is `addChatToList(chat_id, chatListMain)`; TDLib removes the chat
  from the other list itself. The call runs on a background thread
  (`TaskDispatcher::dispatch_add_chat_to_list`) and reports only a failure, as
  `BackgroundTaskResult::AddChatToListFailed`. On success TDLib pushes
  `updateChatPosition` for both lists, which updates the cached
  `chat.positions` and reloads the active list.
- **`is_archived`** is derived from the cached `chat.positions` when the chat
  list is built, so it needs no request.

## Out of scope

- Unread counters in the folder picker rows.
- An archive row at the top of the main list, as in the official clients.
- Archive settings (`archiveChatListSettings`).
- Special handling of automatic unarchiving: an unmuted archived chat that gets
  a new message simply disappears from the archive on the next reload.
