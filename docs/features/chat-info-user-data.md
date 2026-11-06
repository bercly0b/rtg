# Chat info: what TDLib exposes about a chat

Research notes for extending the chat info popup (`I` in the chat list). They
list what TDLib can return about the other side of a chat, what each field
costs to fetch, and what is worth showing. Nothing here is implemented yet.

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

## The "new contact" card of the native clients

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

## Proposal

Ordered by value for the cost:

1. **No new requests**, read from what the popup already fetches:
   - the user ID or chat ID;
   - the new-contact card from `chat.action_bar`: phone country, registered
     month and year, last name and photo change. Shown only while TDLib has
     it, like the native clients;
   - contact status and the verified / scam / fake flags;
   - the phone number when it is visible, and all active usernames;
   - birthdate, number of groups in common, personal channel;
   - for groups and channels: admin count and linked chat.
2. **Offline requests** (`getSupergroup`, `getBasicGroup`, `getChat` for a
   linked chat): public `@username` link, the current user's role, joined
   date.
3. **Network requests**: the list of groups in common (`getGroupsInCommon`) and
   country names (`getCountries`, once per session).

Not worth showing in a terminal client: colours, custom emoji, sticker sets,
boosts, gift settings, paid-reaction flags and other fields that only drive
native UI.

## Open questions

- **Keep the card after it disappears?** TDLib cannot return `accountInfo` once
  the action bar is gone. Keeping it would mean storing it ourselves, which
  goes against following TDLib's model. The proposal is to show it only while
  TDLib has it.
- **Country format**: the bare code (`RU`), a flag (needs emoji support in the
  terminal), or a name from `getCountries`.
- **Chat list**: the same data could mark strangers or scam accounts directly
  in the chat list. `chat.action_bar` comes with the chat and
  `updateChatActionBar`, so the list would need no extra requests.
