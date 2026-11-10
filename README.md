<p align="center">
  <b>English</b> | <a href="README.ru.md">Русский</a>
</p>

<h1 align="center">RTG</h1>

<h3 align="center">
  Fast, vim-like TUI client for Telegram, written in Rust.
</h3>

<div align="center">

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Built with ratatui](https://img.shields.io/badge/built%20with-ratatui-success)](https://github.com/ratatui/ratatui)
[![Powered by TDLib](https://img.shields.io/badge/powered%20by-TDLib-blue)](https://github.com/tdlib/td)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/bercly0b/rtg)](https://github.com/bercly0b/rtg/releases/latest)

</div>

RTG is a fast, keyboard-driven Telegram client for the terminal, built on top of
[TDLib](https://core.telegram.org/tdlib). It strips Telegram down to its core —
chats and messages. That's it. No stories. No ads. No notifications. Enjoy the
silence.

The project is in early development — the core chat experience is in place and
new features are landing regularly.

<p align="center">
  <img src="docs/assets/readme-demo.gif" alt="RTG demo: browsing the Telegram chat list with vim-like keys, searching chats, reading messages and recording a voice message in the terminal" width="800">
</p>

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

On macOS and Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bercly0b/rtg/releases/latest/download/rtg-tui-installer.sh | sh
```

The installer puts `rtg` into `~/.cargo/bin` and adds it to your `PATH`. To
install elsewhere, end the command with `| RTG_TUI_INSTALL_DIR=~/.local sh`:
`rtg` then goes into `~/.local/bin`. For anything else — archives for each
platform, checksums — see the
[latest release](https://github.com/bercly0b/rtg/releases/latest).

Or with [Homebrew](https://brew.sh):

```bash
brew install bercly0b/rtg/rtg
```

Or from [crates.io](https://crates.io/crates/rtg-tui), building `rtg` from
source with a Rust toolchain:

```bash
cargo install --locked rtg-tui
```

On Linux the build needs libc++: `sudo apt install libc++-dev libc++abi-dev`.

On Linux, `rtg` needs Ubuntu 24.04 or newer (glibc 2.39) and the libc++ runtime:

- Ubuntu 24.04: `sudo apt install libc++1 libc++abi1`
- Ubuntu 26.04 and newer: `sudo apt install libc++1 libc++abi1 llvm-libunwind1`

To update an `rtg` installed by the installer, run `rtg update` or press `U` in
RTG; update a Homebrew install with `brew upgrade rtg` and a crates.io install
with `cargo install --locked rtg-tui`. When a newer release is out, RTG shows it
in the status bar.

If you installed an older release with its `install.sh`, remove that copy from
`/usr/local`:

```bash
sudo rm -f /usr/local/bin/rtg /usr/local/lib/libtdjson.*
```

### Build from source

Prerequisites:

- [mise](https://mise.jdx.dev/installing-mise.html), activated in your shell —
  installs the Rust toolchain pinned in [`mise.toml`](mise.toml)
- On Linux, libc++: TDLib is linked into `rtg` statically, but its C++
  runtime comes from the system:
  ```bash
  sudo apt install libc++-dev libc++abi-dev
  ```

TDLib (~50MB) is downloaded automatically on first build via the
`download-tdlib` feature of `tdlib-rs`. Supported targets: Linux x86_64/arm64,
macOS Intel/Apple Silicon, Windows x86_64/arm64. For other platforms see the
[TDLib build instructions](https://tdlib.github.io/td/build.html).

```bash
git clone https://github.com/bercly0b/rtg.git
cd rtg
mise install
cargo build --release
./target/release/rtg
```

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
