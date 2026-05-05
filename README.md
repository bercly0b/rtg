<h1 align="center">RTG</h1>

<h3 align="center">
  Fast, vim-like TUI client for Telegram, written in Rust.
</h3>

<div align="center">

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-success)](https://github.com/ratatui/ratatui)
[![Powered by TDLib](https://img.shields.io/badge/powered%20by-TDLib-blue)](https://github.com/tdlib/td)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Quality Gate](https://github.com/bercly0b/rtg/actions/workflows/quality-gate.yml/badge.svg)](https://github.com/bercly0b/rtg/actions/workflows/quality-gate.yml)

</div>

RTG is a fast, keyboard-driven TUI client built on top of
[TDLib](https://core.telegram.org/tdlib). It strips Telegram down to its core —
chats and messages. That's it. No stories. No ads. No notifications. Enjoy the
silence.

The project is in early development — the core chat experience is in place and
new features are landing regularly.

<!-- TODO: replace with a real screenshot / demo gif -->
![RTG screenshot placeholder](https://placehold.co/800x450?text=RTG+screenshot+coming+soon)

## Features

- TUI interface powered by [ratatui](https://github.com/ratatui/ratatui) and `crossterm`
- TDLib-backed Telegram backend with persisted session
- Live chat list with connectivity status and unread updates
- Read, send, reply, edit, delete, and copy messages
- Message reactions
- Voice message recording (via configurable `ffmpeg` command)
- File downloads with auto-download size limit
- Open attachments with custom MIME handlers (mailcap-style)
- Customizable, context-aware keybindings
- Optimistic UI for instant feedback on user actions

## Installation

> Package-manager distribution (Homebrew, AUR, `cargo install`, prebuilt
> binaries) is not available yet. **TODO** — track in a future release.

### Build from source

Prerequisites:

- Rust toolchain (stable) with `cargo`, `rustfmt`, and `clippy`
- On Linux, the libc++ runtime used by the prebuilt TDLib:
  ```bash
  sudo apt install libc++1 libc++abi1 libunwind8
  ```

TDLib (~50MB) is downloaded automatically on first build via the
`download-tdlib` feature of `tdlib-rs`. Supported targets: Linux x86_64/arm64,
macOS Intel/Apple Silicon, Windows x86_64/arm64. For other platforms see the
[TDLib build instructions](https://tdlib.github.io/td/build.html).

```bash
git clone https://github.com/bercly0b/rtg.git
cd rtg
cargo build --release
```

### Run

```bash
cargo run --release
```

Press `?` for the in-app help overlay.

## Configuration

RTG reads its configuration from `~/.config/rtg/config.toml`
(or `$XDG_CONFIG_HOME/rtg/config.toml` if set).

On first launch RTG will prompt for your Telegram API credentials
(`api_id` and `api_hash`) — get them from
[my.telegram.org/apps](https://my.telegram.org/apps). After you enter them,
RTG creates the config file at the path above with your credentials saved.

You can then edit that file to tune the rest of the available options —
logging, voice recording, download limits, MIME handlers, and key bindings.
See [`config.example.toml`](config.example.toml) for the full reference.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before
opening a pull request, and check the
[open issues](https://github.com/bercly0b/rtg/issues) for areas that need help.

## License

[MIT](LICENSE)
