# Changelog

User-visible changes in each RTG release. Releases up to v0.1.14 are listed on
[GitHub Releases](https://github.com/bercly0b/rtg/releases).

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
