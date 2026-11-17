# Mentions and the link picker

A mention is an `@username` in the text of a message; a link is an address
that opens in the browser (see "Messages" in [GLOSSARY.md](../../GLOSSARY.md)).
Both are highlighted with the same style, and `o` on a message
(rebindable as `open_link_or_mention`; `open_link_in_browser` is kept as an
alias) opens them.

## What `o` opens

The links and mentions of the selected message are collected in text order,
each one once:

- **none** — the first `http(s)://` word of the text is opened, if any. This
  fallback only applies to a message without link or mention entities;
- **one** — it is opened at once: a link in the browser, a mention as a chat;
- **several** — the link picker lists them. A link is shown by the address it
  leads to, even when the text hides it behind a word; a long address is cut
  to the popup width. `j`/`k` move, `Enter` or `o` open the selected row and
  close the picker, `q`/`Esc` close it.

## Opening the chat behind a mention

- The lookup may need the server, so it runs in the background behind an
  "Opening @name..." notification.
- A late answer is dropped when the user has left the place they asked from:
  another chat or topic is open, or the focus is back on the chat list. Only
  the latest lookup counts.
- The found chat opens like any chat of its kind: a user or a bot as a
  private chat, a group or a channel even without membership, a forum as its
  topic list, the user's own username as Saved Messages.
- When the active chat list holds the chat, the selection moves to it;
  otherwise the list is left as it was and `h` returns to it. A mention opened
  from a forum topic leaves that forum first, so `h` leads to the chat list.
- A mention of the chat that is already open does nothing.
- "@name not found" is shown when nobody holds the username, and
  "Could not open @name" when the lookup failed.

## Chat info of the sender and the mentioned

`I` on a message (`show_chat_info`, the same action and key as in the chat
list) shows the chat info popup, described in
[chat-info-user-data.md](chat-info-user-data.md).

- **Without mentions** the popup is about the sender of the message, as TDLib
  names it: a user, or a chat — a channel for its posts, a group for an
  anonymous admin. A forwarded message shows who forwarded it, not its
  original sender; the current user's own message shows the popup about
  the current user.
- **With at least one mention** the info picker asks first. Its first row is
  the sender, by name, selected by default; then come the mentions in text
  order, each one once. Links are not listed. A mention of the sender is kept
  as its own row. `j`/`k` move, `Enter` or `I` open the popup about the
  selected row, `q`/`Esc` close the picker.
- The popup opens at once in its loading state, titled with the sender's name
  or `@name`, without a notification. An answer that arrives after the popup
  was closed is dropped.
- "@name not found" is shown, and the popup closed, when nobody holds the
  username. A failed request leaves the popup in its error state.

The message info popup moved from `I` to `m` (`show_message_info`); `q`, `Esc`
or `m` close it.

## TDLib integration

- **Mentions** come from `textEntityTypeMention` in the `formattedText` of a
  message or caption, next to `textEntityTypeUrl` and `textEntityTypeTextUrl`.
  RTG does not parse `@` itself, so an address or code with `@` is not a
  mention. `textEntityTypeMentionName` (a user without a username) is not
  handled.
- **Lookup** is `searchPublicChat(username)`: it returns the chat, or an
  error when the username is unoccupied or invalid (`USERNAME_NOT_OCCUPIED`,
  `USERNAME_INVALID`, "Username is invalid") — those mean "not found"; any
  other error is a failed lookup.
- The found chat is opened with the regular `openChat` / `getChatHistory`
  flow; it is not added to any chat list.
- **The sender** comes from `message.sender_id`: `messageSenderUser` or
  `messageSenderChat`. The chat info popup takes a sender chat by its id, a
  sender user through `createPrivateChat(user_id, force = false)` and a
  mention through `searchPublicChat`. `createPrivateChat` returns the
  existing private chat or creates a local record of one; such a chat joins
  no chat list, so the user's chat lists do not change.

## Out of scope

`t.me/...` links open in the browser like any link. Usernames in the chat
info and message info popups are not interactive. A mention without a
username (`textEntityTypeMentionName`) is not offered by the info picker. The
original sender of a forwarded message has no chat info popup. `I` does
nothing in the topic list of a forum, and on a message that is still being
sent optimistically.
