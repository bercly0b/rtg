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
