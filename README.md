<p align="center">
  <b>English</b> | <a href="README.ru.md">Русский</a>
</p>

<h1 align="center">RTG — Telegram TUI client in Rust</h1>

<h3 align="center">
  Fast, vim-like terminal Telegram client (CLI/TUI), built on TDLib and ratatui.
</h3>

<div align="center">

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-success)](https://github.com/ratatui/ratatui)
[![Powered by TDLib](https://img.shields.io/badge/powered%20by-TDLib-blue)](https://github.com/tdlib/td)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/bercly0b/rtg)](https://github.com/bercly0b/rtg/releases/latest)

</div>

RTG is a fast, keyboard-driven Telegram client for the terminal on Linux and
macOS, written in Rust on top of [TDLib](https://core.telegram.org/tdlib). It
strips Telegram down to its core — chats and messages. That's it. No stories. No
ads. No notifications. Enjoy the silence.

<p align="center">
  <img src="docs/assets/readme-demo.gif" alt="RTG demo: browsing the Telegram chat list with vim-like keys, searching chats, reading and sending messages, reacting to a message and viewing chat info in the terminal" width="800">
</p>

## Features

- Chat list with unread counters, search, folders and archive
- Forum topics
- Read, send, reply to, edit, delete and copy messages
- Reactions
- Voice messages, recorded with `ffmpeg`
- Send files from a built-in file picker
- Download and save files, with a size limit for auto-download
- Open attachments and links in your own apps (mailcap-style handlers)
- Chat and message info
- Vim-like keys, every one remappable
- Built-in updater: `rtg update`

## Installation

RTG runs on macOS and Linux.

### Install script

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bercly0b/rtg/releases/latest/download/rtg-tui-installer.sh | sh
```

The script puts `rtg` into `~/.cargo/bin` and adds it to your `PATH`. Archives
for each platform and checksums are in the
[latest release](https://github.com/bercly0b/rtg/releases/latest).

### Homebrew

```bash
brew install bercly0b/rtg/rtg
```

### Cargo

```bash
cargo install --locked rtg-tui
```

This builds `rtg` from source and needs a Rust toolchain.

### Linux requirements

`rtg` needs Ubuntu 24.04 or newer (glibc 2.39) and the libc++ runtime:

- Ubuntu 24.04: `sudo apt install libc++1 libc++abi1`
- Ubuntu 26.04 and newer: `sudo apt install libc++1 libc++abi1 llvm-libunwind1`

Building with Cargo also needs the libc++ headers:
`sudo apt install libc++-dev libc++abi-dev`.

## Getting started

1. Run `rtg`.
2. On the first run, enter your phone number and the code Telegram sends you,
   then your two-step verification password if you have one.
3. Press `?` to see the hotkeys.

`rtg logout` signs you out and removes the local session.

## Hotkeys

Everywhere:

| Key | Action |
| --- | --- |
| `?` | Show the hotkeys |
| `U` | Install the update |
| `q` | Quit |

In the chat list:

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous chat |
| `gg` | First chat |
| `Enter` / `l` | Open the chat |
| `/` | Search chats |
| `f` | Pick a folder or the archive |
| `A` | Archive or unarchive the chat |
| `r` | Mark the chat as read |
| `I` | Chat info |

In an open chat:

| Key | Action |
| --- | --- |
| `j` / `k` | Next / previous message |
| `G` | Last message |
| `h` / `Esc` | Back to the chat list |
| `i` | Write a message |
| `r` | Reply to the message |
| `e` | Edit the message |
| `dd` | Delete the message |
| `y` | Copy the message |
| `R` | React to the message |
| `l` | Open the attachment |
| `o` | Open a link from the message |
| `A` | Attach a file |
| `v` | Record a voice message |
| `D` | Download the file |
| `S` | Save the file to Downloads |
| `m` | Message info |

While writing a message, `Enter` sends it and `Shift+Enter` starts a new line.

The full list is in RTG itself: press `?`.

## Configuration

RTG reads its configuration from `~/.config/rtg/config.toml`
(or `$XDG_CONFIG_HOME/rtg/config.toml` if set). The file is optional.

```toml
[download]
max_auto_download_size = "10MB"

[open]
"audio/*" = "mpv {file_path}"
"image/*" = "imv {file_path}"

[attach]
dirs = ["~/Downloads", "~/Documents"]

[keys]
quit = "Q"
delete_message = "xx"
```

| Section | What it sets |
| --- | --- |
| `[logging]` | Log level and how many log files to keep |
| `[telegram]` | Your own Telegram API credentials |
| `[voice]` | The command that records voice messages |
| `[attach]` | Where the file picker looks for files |
| `[download]` | The size limit for auto-download |
| `[open]` | Apps that open attachments, by MIME type |
| `[updates]` | The daily check for a newer release |
| `[keys]` | Hotkeys |

See [`config.example.toml`](config.example.toml) for the full reference.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before
opening a pull request — it also covers building from source — and check the
[open issues](https://github.com/bercly0b/rtg/issues) for areas that need help.

If you like RTG, a star helps: curated lists such as
[awesome-rust](https://github.com/rust-unofficial/awesome-rust) only accept
projects with enough of them.

## License

[MIT](LICENSE)
