# Global search

Global search looks through the whole account and Telegram itself for chats,
contacts and messages that match a query, and shows what was found in a popup
(see "Search" in [GLOSSARY.md](../../GLOSSARY.md)). Quick search (`/`,
rebindable as `quick_search`) is a different thing: it moves the selection
within the current chat list and asks Telegram nothing.

## Opening the popup

`s` in the chat list (rebindable as `global_search`) opens the popup. The key
works only when the chat list has the focus: in the topic list of a forum and
in the messages panel it does nothing.

The popup takes every key while it is open:

- any character goes into the query, `Backspace` erases the last one;
- `Ctrl+J` / `Ctrl+K` and the arrow keys move the selection;
- `Enter` opens the selected row;
- `Esc` closes the popup and leaves the chat list as it was.

## Sections

The query sits on the first line; the results follow in sections, each under
its title. A section that found nothing is left out together with its title.
The selection walks all sections as one list.

| Section | What it holds | Limit |
| --- | --- | --- |
| Chats | Chats and contacts of the account | 10 |
| Global | Public chats the account is not in | 5 |
| Messages | Messages from every chat, newest first | pages of 20 |

- "Chats" joins three answers, each chat once: the chats this device already
  knows, then the chats the server finds, then contacts.
- "Global" leaves out the chats that "Chats" already shows.
- "Messages" searches the main list and the archive, whatever chat list is
  open. It does not depend on the other sections: a chat can be both in
  "Chats" and the source of found messages. Secret chats are not searched.
- "Chats" and "Global" have no "show more". "Messages" loads its next page
  when the selection reaches the last row.

## Rows

- **A chat**: its title and its `@username`, if it has one.
- **A message**: the chat, the sender and the date on one line, the text on
  the next one, cut to the width of the popup. The text is the preview the
  chat list shows for a last message: for media, the caption or a label such
  as "[Photo]". An own message is from "You"; a sender that is the chat itself
  (a channel post, the other side of a private chat) is not repeated. The
  date is the time for today, the day and month for this year, the full date
  otherwise.
- The match is not highlighted: TDLib reports no positions, and the server
  matches word forms.

## When Telegram is asked

- Known chats are searched for on every change of the query: that request
  needs no server.
- The requests that need the server are sent once the user has stopped typing
  for about 300 ms.
- Editing the query drops the results at once, so everything shown belongs to
  the current query. An answer to an earlier query is dropped, and so is one
  that arrives after the popup was closed.
- An empty query shows an empty popup: no recent searches, no top chats.

## States

- "Searching…" stays under the results until every request for the query has
  answered.
- "Nothing found" replaces it when all of them answered and found nothing.
- A failed request leaves the other sections alone and is reported in the
  status bar: "Could not search chats", "Could not search contacts",
  "Could not search public chats" or "Could not search messages". A page of
  messages that failed to load is asked for again the next time the selection
  is moved down from the last row.

## Enter

- **On a chat** of "Chats" or "Global": the chat opens the way a mention does
  (see [mentions.md](mentions.md)). It may be in no chat list; when the active
  list holds it, the selection moves to it. A forum opens as its topic list.
  The account does not join a public chat by opening it.
- **On a message**: its chat opens at the latest messages. A message of a
  forum topic opens that topic at once, not the topic list; `h` then leads to
  the topic list. The topic is titled with the forum's name until the topic
  list arrives and names it; a topic outside the loaded topic list keeps the
  forum's name.

## TDLib integration

TDLib has no single method that searches everything, so each part of the
results is its own request, and the requests run side by side:

| Request | Section | Sent |
| --- | --- | --- |
| `searchChats(query, 10)`, an offline method | Chats | on every change of the query |
| `searchChatsOnServer(query, 10)` | Chats | after the pause |
| `searchContacts(query, 10)` | Chats | after the pause |
| `searchPublicChats(query)` | Global | after the pause |
| `searchMessages(chat_list = null, query, offset, 20)` | Messages | after the pause |

- `searchContacts` returns users. A user is turned into a chat with
  `createPrivateChat(user_id, force = true)`: no server request, and the chat
  joins no chat list.
- `searchMessages` gets no chat list, filter or date range. The next page is
  asked for with the `next_offset` of the page before; an empty `next_offset`
  is the last page.
- The forum topic of a message comes from `message.topic_id`
  (`messageTopicForum`), and only for a chat that is a forum.
- A found chat is opened with the regular `openChat` / `getChatHistory` flow.
  `addRecentlyFoundChat` is not called.

## Out of scope

Search inside the open chat (`searchChatMessages`, RTG-122). Opening a chat at
the found message (RTG-123). Recent searches, top chats, joining a found public
chat, filters by chat type and date.
