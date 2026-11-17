# Changelog

User-visible changes in each RTG release. Releases up to v0.1.14 are listed on
[GitHub Releases](https://github.com/bercly0b/rtg/releases).

## 0.1.18 - 2026-11-17

- Show the chat info popup from an open chat: `I` on a message shows who sent it, and a message with mentions first asks whether to show its sender or one of the mentioned.
- The message info popup moved from `I` to `m`.
- Look at a file before sending it: `Ctrl+L` in the file picker opens the selected file with the system opener, and the picker stays open.
- Videos, audio files and GIFs sent from the file picker now play in the recipient's client instead of arriving as plain files. Durations, frame sizes and audio tags are read with `ffprobe`; without it the file is still sent, just without them.
- See how much of a file you are sending has been uploaded: the message shows `sending=42%` while the file uploads.
- Send files from the file picker: `A` in an open chat lists local files newest first, typing filters them, `Enter` sends the selected one. Images go as photos, everything else as documents; the text in the input becomes the caption.
- Choose what the file picker lists with `[attach] dirs` and `[attach] list_cmd` in the config.

## 0.1.17 - 2026-11-13

- Open the chat behind an `@username` mention: mentions in messages are highlighted like links, and `o` on a message opens its link or mention.
- Pick what to open when a message holds several links or mentions: `o` shows a list of them.
- The `open_link_in_browser` key action is now `open_link_or_mention`; the old name keeps working in the config.
- Status bar no longer draws a contrasting stripe on light terminal themes; it uses the terminal background.

## 0.1.16 - 2026-11-11

- Install from crates.io with `cargo install --locked rtg-tui`.
- See the number of unread chats next to each list in the folder picker (`f`).

## 0.1.15 - 2026-11-07

- Press `U` in RTG to install an update in the background; the status bar asks you to restart once it is installed. A Homebrew or crates.io install shows its update command instead.
- Install RTG with Homebrew: `brew install bercly0b/rtg/rtg`.
