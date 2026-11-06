# Chat info: what TDLib exposes about a chat

Research notes for extending the chat info popup (`I` in the chat list). They
list what TDLib can return about the other side of a chat, what each field
costs to fetch, and what is worth showing. The [Design](#design) section at
the end records what was decided. Nothing here is implemented yet.

Sources: the TDLib 1.8.61 schema bundled with `tdlib-rs` 1.3.0 (`tl/api.tl`)
and the TDLib sources (`MessagesManager.cpp`, `DialogActionBar.cpp`,
`UserManager.cpp`, `DialogId.cpp`). Nothing was checked on a live account.

## What the popup requests today

`resolve_chat_info` (`src/telegram/tdlib_auth/chat_details/mod.rs`) calls:

| Chat | Requests | Shown |
|------|----------|-------|
| Private / secret | `getChat`, `getUser`, `getUserFullInfo` | name, status or "bot", first active `@username`, bio |
| Basic group | `getChat`, `getBasicGroupFullInfo` | name, member count, description |
| Supergroup / channel | `getChat`, `getSupergroupFullInfo` | name, member / subscriber count, description |

`getChat` and `getUser` are offline methods: TDLib answers them from memory.
`getUserFullInfo` and `get*FullInfo` are answered from cache when TDLib has a
fresh copy, otherwise they go to the server. Everything these calls return is
already in our hands, but the popup reads only a few fields.

## Account info: the card native clients show for a stranger

When someone who is not a contact starts a chat, the mobile clients show a card
with the country of their phone number, when they joined Telegram, and when
they last changed their name or photo. In TDLib this is
`chat.action_bar = chatActionBarReportAddBlock { can_unarchive, account_info }`:

| `accountInfo` field | Meaning |
|---------------------|---------|
| `registration_month`, `registration_year` | When the account was registered; `0` if unknown |
| `phone_number_country_code` | ISO 3166-1 alpha-2 code of the phone number's country (`"RU"`); empty if unknown. The number itself is not given |
| `last_name_change_date` | Unix time of the last name change; `0` if unknown |
| `last_photo_change_date` | Unix time of the last photo change; `0` if unknown |

How it behaves:

- **Where it comes from.** The server sends it as peer settings in the
  `users.getFullUser` response. TDLib asks for it when it first loads a chat
  (`fix_new_dialog`), and again after `getUserFullInfo` reloads the user. It
  then pushes `updateChatActionBar` and keeps the value in `chat.action_bar`.
  We don't need `openChat` for it.
- **Who gets it.** Only `chatActionBarReportAddBlock`, which TDLib builds only
  for a private chat where the other user can be blocked, reported and added
  to contacts: a non-contact who wrote first. For any other action bar TDLib
  drops `account_info`. A secret chat reuses the action bar of the private
  chat with the same user, but gets `account_info` only while it is archived.
- **Why it disappears.** The action bar goes back to `null` when the server
  stops sending these peer settings (the native client hides the card once the
  conversation is under way), when the chat is reported as spam, the user is
  blocked or added to contacts, or after `removeChatActionBar`. No other TDLib
  method returns `accountInfo`, so once the bar is gone the data cannot be
  fetched again.
- **Cost.** None. The popup already calls `getChat`. We only have to read
  `chat.action_bar`.
- **Country name.** `getCountries` returns `countryInfo { country_code, name,
  english_name }`, so `"RU"` can be shown as "Russia". The list does not
  change while the app runs, so one request per session is enough. Showing
  the bare code, or a flag emoji built from it, needs no request.

## Private chats: other fields

| Data | Source | Already fetched | Notes |
|------|--------|-----------------|-------|
| User ID | `chatTypePrivate.user_id` / `user.id` | yes | For a private chat the chat ID is the same number. A secret chat has its own chat ID |
| Phone number | `user.phone_number` | yes | Empty unless the user shares it with us (contacts, privacy settings) |
| Contact status | `user.is_contact`, `is_mutual_contact`, `is_close_friend` | yes | "Not a contact" is the key fact for a stranger |
| Verified / scam / fake | `user.verification_status` | yes | `is_verified`, `is_scam`, `is_fake`; `null` if none |
| Premium, support | `user.is_premium`, `user.is_support` | yes | |
| Account type | `user.type` | yes | Regular, bot, deleted, unknown. Today only "bot" is shown |
| All usernames | `user.usernames` | yes | We show the first active one. There may be more, plus collectible ones (Fragment) |
| Restricts new chats | `user.restricts_new_chats`, `user.paid_message_star_count` | yes | Messaging this user may be blocked or paid |
| Birthdate | `userFullInfo.birthdate` | yes | Day and month; year `0` if unknown |
| Groups in common | `userFullInfo.group_in_common_count` | yes | The list itself needs `getGroupsInCommon` (network) |
| Personal channel | `userFullInfo.personal_chat_id` | yes | `0` if none. Its title needs `getChat` |
| Contact note | `userFullInfo.note` | yes | The current user's own note about the contact |
| Blocked | `userFullInfo.block_list` | yes | `null` if not blocked |
| Calls | `userFullInfo.can_be_called`, `has_private_calls` | yes | |
| Business account | `userFullInfo.business_info` | yes | Location, opening hours, start page |
| Bot details | `userFullInfo.bot_info` | yes | Short description, description, commands, privacy policy URL |
| Rating, gifts | `userFullInfo.rating`, `gift_count` | yes | |
| Secret chat state | `getSecretChat` | no | State, key fingerprint (`key_hash`); offline method |

## Groups and channels

| Data | Source | Already fetched | Notes |
|------|--------|-----------------|-------|
| Chat ID | `chat.id` | yes | Basic group: `-basic_group_id`. Supergroup or channel: `-100` followed by `supergroup_id`, the same ID the Bot API uses |
| Public username | `supergroup.usernames` | no, `getSupergroup` (offline) | Gives a `t.me/<username>` link |
| Joined / created | `supergroup.date` | no, `getSupergroup` | When the current user joined; the creation date if they aren't a member |
| My role | `supergroup.status`, `basicGroup.status` | no, `getSupergroup` / `getBasicGroup` (offline) | Owner, admin, member, restricted, left |
| Kind and settings | `supergroup.is_forum`, `is_broadcast_group`, `join_by_request`, `is_slow_mode_enabled`, `sign_messages` | no, `getSupergroup` | |
| Verified / scam / fake | `supergroup.verification_status` | no, `getSupergroup` | |
| Admin count | `supergroupFullInfo.administrator_count` | yes | Restricted and banned counts are `0` unless known, in practice for admins |
| Linked chat | `supergroupFullInfo.linked_chat_id` | yes | A channel's discussion group, or the group's channel |
| Slow mode delay | `supergroupFullInfo.slow_mode_delay` | yes | Seconds |
| Location | `supergroupFullInfo.location` | yes | Location-based groups only |
| Invite link | `supergroupFullInfo.invite_link`, `basicGroupFullInfo.invite_link` | yes | Only for admins allowed to invite |
| Creator | `basicGroupFullInfo.creator_user_id` | yes | Basic groups only |
| Upgrade history | `supergroupFullInfo.upgraded_from_basic_group_id`, `basicGroup.upgraded_to_supergroup_id` | partly | |

## Design

Decisions from the design discussion on ROZ-172. The terms (chat info popup,
info template, account info) are defined in [GLOSSARY.md](../../GLOSSARY.md).

### Principles

- **Info templates.** Each kind of chat has its own info template: User, Bot,
  Group or Channel. Every popup of one kind shows the same blocks and rows in
  the same order, so the user always knows where to look.
- **Kinds.** A regular user's private chat, a secret chat, Saved Messages and a
  deleted account use User. A bot uses Bot. A basic group, a supergroup and a
  forum use Group. A channel uses Channel.
- **Layout.** A header (the chat title and the status line, as in the open
  chat's subtitle), then the blocks of the kind, then the common Chat block,
  always last. There is no `Name:` row.
- **Rows that are always shown.** A row is always shown when any chat of that
  kind can have the value. When the value is missing the row stays and shows
  a placeholder:
  - `hidden` when the value is certainly hidden by privacy settings: a user's
    phone number (every account has one, so an empty one is hidden) and a
    last-seen status of `recently`, `within a week` or `within a month`;
  - `—` when the value is not set or unknown.
- **Blocks that are shown on condition.** A block that exists only for part of
  the chats of a kind is shown only when it has data: Account info, Business,
  Secret chat, Location.
- **Requests.** Every request runs when the popup is opened with `I`, never in
  advance for the chat list. The popup loads in one go: `Loading...`, then the
  whole template.
- **Failures.** Only the requests behind the header are critical: `getChat`,
  `getUser`, `getBasicGroupFullInfo` and `getSupergroupFullInfo`. If one fails
  the popup shows an error, as today. Any other failed request leaves its rows
  at `—`, and a country falls back to its ISO code.
- **Account info is not stored.** It is shown while TDLib keeps it in
  `chat.action_bar`. Once TDLib drops it, its block disappears. RTG keeps no
  copy of its own.
- **No scrolling yet.** On a short terminal the bottom of the popup is cut off,
  as today. Scrolling is a separate task.

Dates and times use the format of the message info popup (`2026-10-01 14:20`).

### Chat block (every template)

```
 Chat
   Chat ID       -1001234567890
   Folders       Work, Friends, Archive
   Muted         until 2026-11-08 18:00
   Auto-delete   off
```

| Row | Source | Values |
|-----|--------|--------|
| Chat ID | `chat.id` | Groups and channels in the Bot API form (`-100…`) |
| Folders | `chat.chat_lists`, folder titles from the folder list RTG already keeps | Folder titles, plus `Archive` for the archive list; `—` if none |
| Muted | `chat.notification_settings`; when `use_default_mute_for` is set, `getScopeNotificationSettings` for the chat's scope | `no`, `yes` (muted with no end date), `until <date time>` |
| Auto-delete | `chat.message_auto_delete_time` | `off` or the period |

### User

```
 Alice Smith
 last seen recently (you hide yours)

 Profile
   User ID       123456789
   Username      @alice, @alice_dev, @alice (collectible)
   Phone         hidden
   Birthday      12 March 1990
   Channel       Alice's notes
   Badges        premium, verified
   Bio           Rust dev, Berlin

 You & them
   Contact       not a contact
   In common     3 groups
   Messages      paid, 100 stars each
   Blocked       no
   Note          —

 Account info                ← only while TDLib has it
   Registered    March 2021
   Phone from    Russia
   Name changed  2026-10-01 14:20
   Photo changed 2026-09-12 09:03

 Business                    ← business accounts only
   Address       Berlin, Main st. 1
   Hours         open, closes in 3 h

 Secret chat                 ← secret chats only
   State         ready
   Key           8f3a 91c2 4be0 …

 Chat …
```

| Row | Source | Values |
|-----|--------|--------|
| Status line | `user.status` | As today. A status of `recently`, `within a week` or `within a month` with `by_my_privacy_settings` gets `(you hide yours)`. A deleted account shows `deleted account` |
| User ID | `user.id` | |
| Username | `user.usernames.active_usernames` | All of them; those in `collectible_usernames` get `(collectible)` |
| Phone | `user.phone_number` | With a leading `+`; `hidden` when empty |
| Birthday | `userFullInfo.birthdate` | Without the year when it is `0` |
| Channel | `userFullInfo.personal_chat_id`, title via `getChat` | `—` when `0` |
| Badges | `user.verification_status`, `is_premium`, `is_support` | `verified`, `scam`, `fake`, `premium`, `support`; `scam` and `fake` in red |
| Bio | `userFullInfo.bio` | Multi-line, last in the block |
| Contact | `user.is_contact`, `is_mutual_contact`, `is_close_friend` | `mutual`, `contact`, `not a contact`; `close friend` added |
| In common | `userFullInfo.group_in_common_count` | The count only |
| Messages | `userFullInfo.outgoing_paid_message_star_count` | `free` or `paid, N stars each` |
| Blocked | `userFullInfo.block_list` | `no`, `yes`, `stories only` |
| Note | `userFullInfo.note` | The current user's own note about the contact |
| Account info rows | `chat.action_bar` = `chatActionBarReportAddBlock.account_info` | `Registered` from month and year; `Phone from` is the country name from `getCountries`, its code if that fails; name and photo change dates. `—` for any `0` or empty field |
| Business rows | `userFullInfo.business_info` | Shown when it has a location or opening hours. `Hours` from `opening_hours` with `next_open_in` / `next_close_in` |
| Secret chat rows | `getSecretChat` | `State`: `pending`, `ready`, `closed`. `Key`: the first 32 bytes of `key_hash` in hex |

### Bot

```
 Weather Bot
 bot · 120 345 monthly users

 Bot
   User ID       987654321
   Username      @weather_bot
   Commands      /start /help /settings
   Groups        can join, reads all messages
   Inline        yes, "Search city…"
   Privacy       https://example.com/privacy
   Badges        verified
   Blocked       no
   About         Short description…
   Description   Long description…

 Chat …
```

| Row | Source | Values |
|-----|--------|--------|
| Status line | `userTypeBot.active_user_count` | `bot · N monthly users`; `bot` when `0` |
| User ID, Username, Badges, Blocked | as in User | |
| Commands | `userFullInfo.bot_info.commands` | Command names, wrapped. To be revisited if it gets noisy on bots with many commands |
| Groups | `userTypeBot.can_join_groups`, `can_read_all_group_messages` | `can join`, `can join, reads all messages` (privacy mode off), `cannot join` |
| Inline | `userTypeBot.is_inline`, `inline_query_placeholder` | `yes, "<placeholder>"` or `no` |
| Privacy | `bot_info.privacy_policy_url` | |
| About | `bot_info.short_description` | |
| Description | `bot_info.description` | Multi-line, last in the block |

### Group

```
 Rust Talks
 1 234 members

 Group
   Link          t.me/rust_talks
   Kind          supergroup, forum
   Admins        12
   Channel       Rust News
   Slow mode     30 s
   Joining       by request
   History       hidden from new members
   Forwarding    restricted
   Invite link   —
   Badges        —
   Description   …

 You
   Role          admin "Moderator"
   Joined        2024-05-01

 Location                    ← location-based groups only
   Address       …

 Chat …
```

The member count stays in the header and is not repeated as a row.

| Row | Source | Values |
|-----|--------|--------|
| Link | `supergroup.usernames` | `t.me/<username>`; `—` for a basic group or a private supergroup |
| Kind | basic group or supergroup, `supergroup.is_forum`, `is_broadcast_group` | `basic group`, `supergroup`, plus `forum` or `broadcast group` |
| Admins | `supergroupFullInfo.administrator_count`; for a basic group the owner and admins in `basicGroupFullInfo.members` | `—` when `0` |
| Channel | `supergroupFullInfo.linked_chat_id`, title via `getChat` | |
| Slow mode | `supergroupFullInfo.slow_mode_delay` | `off` or the delay; `—` for a basic group |
| Joining | `supergroup.join_by_request`, `supergroup.usernames` | `by request`, `open` (public), `invite only` |
| History | `supergroupFullInfo.is_all_history_available` | `visible to new members`, `hidden from new members`; `—` for a basic group |
| Forwarding | `chat.has_protected_content` | `allowed`, `restricted` |
| Invite link | `supergroupFullInfo.invite_link`, `basicGroupFullInfo.invite_link` | Only admins who may invite get it |
| Badges | `supergroup.verification_status` | `verified`, `scam`, `fake` |
| Description | `*FullInfo.description` | Multi-line, last in the block |
| Role | `supergroup.status`, `basicGroup.status` | `owner`, `admin "<custom title>"`, `member`, `restricted until <date>`, `left`, `banned` |
| Joined | `supergroup.date` | `—` for a basic group, or when the user is not a member (then the field holds the creation date) |
| Location rows | `supergroupFullInfo.location` | Shown only when it is set |

### Channel

```
 Rust News
 12 345 subscribers

 Channel
   Link          t.me/rust_news
   Discussion    Rust Talks
   Signatures    on
   Forwarding    allowed
   Admins        —
   Badges        verified
   Description   …

 You
   Role          subscriber
   Joined        2023-01-10

 Chat …
```

Rows shared with Group use the same sources. Specific to channels:

| Row | Source | Values |
|-----|--------|--------|
| Discussion | `supergroupFullInfo.linked_chat_id`, title via `getChat` | |
| Signatures | `supergroup.sign_messages` | `on`, `off` |
| Admins | `supergroupFullInfo.administrator_count` | TDLib reports it only to admins, so subscribers see `—` |
| Role | `supergroup.status` | `subscriber` instead of `member` |

### Requests

| Kind | Already made | Added |
|------|--------------|-------|
| User, Bot | `getChat`, `getUser`, `getUserFullInfo` | `getScopeNotificationSettings`; `getChat` for the personal channel; `getSecretChat` for a secret chat; `getCountries` only when account info is present |
| Group | `getChat`, `getBasicGroupFullInfo` / `getSupergroupFullInfo` | `getBasicGroup` / `getSupergroup`; `getScopeNotificationSettings`; `getChat` for the linked chat |
| Channel | `getChat`, `getSupergroupFullInfo` | `getSupergroup`; `getScopeNotificationSettings`; `getChat` for the linked chat |

`getBasicGroup`, `getSupergroup`, `getSecretChat` and `getChat` are offline
methods. `getScopeNotificationSettings` is usually answered from memory.
`getCountries` may go to the server, and TDLib caches its result.

### Not shown

Colours, profile background and theme, emoji status, custom emoji, stories,
gifts and gift settings, rating, boosts, paid reactions, the group sticker
set, bot mini app settings, affiliate programs, and call flags (RTG has no
calls). They either drive native UI only or belong to features a terminal
client does not have.

### Out of scope

Each of these is a separate task:

- marking strangers and scam accounts in the chat list;
- a profile for a message's author in a group, reusing the User template;
- scrolling in the chat info popup;
- an approximate registration date estimated from the user ID (ROZ-190).
