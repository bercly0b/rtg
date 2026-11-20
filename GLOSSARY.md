# RTG

RTG is a terminal-first Telegram client. This glossary fixes the words used for its domain concepts.

## Distribution and updates

**Install channel**:
The way RTG got onto a user's machine: the release installer, Homebrew, or crates.io.
_Avoid_: install method, install source

**Release installer**:
The install channel that RTG owns: a one-command script that fetches a build from a GitHub Release.
_Avoid_: binary install, standalone install, install.sh

**Update check**:
RTG finding out that a newer release exists and telling the user about it.
_Avoid_: auto-update, update notification

**Self-update**:
RTG replacing its own installed files with a newer release; only possible for the release installer channel.
_Avoid_: auto-update, upgrade

## Chat info

**Chat info popup**:
The overlay that shows what is known about a chat and about whoever stands behind it.
_Avoid_: modal, profile, chat info modal

**Info template**:
The fixed set and order of rows the chat info popup uses for one kind of chat: User, Bot, Group or Channel.
_Avoid_: layout, field set

**Account info**:
What Telegram reveals about a non-contact who wrote first: the country of their phone number, when they registered, and when they last changed their name or photo.
_Avoid_: new-contact card, widget

**Registration estimate**:
RTG's own guess at when an account was registered, derived from its user ID; not data from Telegram, unlike the registration date in account info.
_Avoid_: approximate date, guessed date, account age

## Chat lists

**Chat list**:
The set of chats the left panel shows at a time: the main list, a folder, or the archive.
_Avoid_: view, tab, directory

**Main list**:
The chat list of every chat that is not archived, shown to the user as "All chats".
_Avoid_: all chats, inbox

**Archive**:
The chat list of chats moved out of the main list. Every chat is in exactly one of the main list and the archive; the archive is not a folder.
_Avoid_: archive folder, archived folder

**Folder**:
A user-defined chat list that picks chats from both the main list and the archive.
_Avoid_: directory, filter

**Archiving**:
Moving a chat from the main list into the archive; the reverse is unarchiving.
_Avoid_: hiding

**Folder picker**:
The popup that switches the left panel between the main list, the folders and the archive.
_Avoid_: folder selector, directory selector

**Unread chat count**:
The number of chats in a chat list that have something unread; it counts chats, not messages.
_Avoid_: unread counter, unread badge, unread messages

## Messages

**Sender**:
Whoever sent a message into a chat: a user, or a chat itself — a channel for its posts, a group for an anonymous admin.
_Avoid_: author

**Mention**:
An `@username` in the text of a message that points at a user, a bot, a group or a channel.
_Avoid_: username link, tag, handle

**Link**:
An address in the text of a message that opens in the browser.
_Avoid_: URL, hyperlink

**Link picker**:
The popup that asks what to open when a message holds more than one link or mention.
_Avoid_: link selector, target picker

**Info picker**:
The popup that asks whose chat info popup to show when a message holds a mention: its sender or one of the mentioned.
_Avoid_: info selector, sender picker

## Attachments

**Attachment**:
A local file the user sends into a chat, as opposed to a file that arrived inside a message.
_Avoid_: upload, outgoing file

**Attachment kind**:
The form in which Telegram receives an attachment: photo, video, audio, animation or document.
_Avoid_: native type, media type, send mode

**File picker**:
The popup that lists local files and lets the user choose the attachment to send.
_Avoid_: file selector, file browser, attach dialog

**File list command**:
The external command that tells the file picker which local files exist; it only names files, RTG finds out the rest about them itself.
_Avoid_: file lister, engine, finder

## Hotkeys

**Hotkey**:
A key or a short sequence of keys that triggers one action in the panel that has focus.
_Avoid_: keybinding, shortcut, key mapping

**Help popup**:
The overlay that lists the hotkeys available in the panel that has focus.
_Avoid_: help menu, help modal, help screen

**Hotkey group**:
A named set of hotkeys that serve the same purpose, shown together under one heading in the help popup; every hotkey belongs to exactly one group.
_Avoid_: section, category, keybinding group

## Telegram API access

**API credentials**:
The `api_id` and `api_hash` pair under which RTG presents itself to Telegram as an application; not what a user signs in with.
_Avoid_: app credentials, API keys, Telegram keys

**Built-in credentials**:
The API credentials that ship with RTG, so a user can sign in without getting their own.
_Avoid_: default credentials, shared credentials, embedded keys

**Own credentials**:
The API credentials a user got from Telegram themselves and gave to RTG; they take precedence over the built-in credentials.
_Avoid_: custom credentials, personal keys, user credentials
