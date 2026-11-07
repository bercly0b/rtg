# Self-update

`rtg update` replaces an RTG installed by the release installer with the latest
release. It is built on the [`axoupdater`](https://docs.rs/axoupdater) crate
([ADR 0002](../adr/0002-release-through-cargo-dist.md)): RTG does not download
or install anything itself, it re-runs the release installer of the new
release into the same directory.

```
$ rtg update
downloading rtg-tui 0.2.0 aarch64-apple-darwin
installing to /Users/me/.cargo/bin
  rtg
everything's installed!
rtg is updated to 0.2.0.
```

## Flow

1. **Install receipt** (`load_receipt`): the release installer writes it to
   `$XDG_CONFIG_HOME/rtg-tui/rtg-tui-receipt.json` or
   `~/.config/rtg-tui/rtg-tui-receipt.json` ([Releases](../RELEASES.md)).
2. **Is it this binary's receipt** (`check_receipt_is_for_this_executable`):
   the receipt's install prefix must be the running binary's directory. The
   release installer and `cargo install` share `~/.cargo/bin`, and a receipt
   may be left over from another copy of RTG.
3. Without a receipt for this binary, RTG prints the update command of its
   install channel and exits with an error:

   ```
   Error: rtg update only updates an rtg installed by the release installer. Update this one with:

       brew upgrade rtg
   ```

   The channel is detected the same way as for the [update check](update-check.md#install-channel):
   Homebrew by the `Cellar` path, crates.io by `$CARGO_HOME/bin`. For any
   other binary RTG points at the release page instead:

   ```
   Error: rtg update only updates an rtg installed by the release installer. Download the latest release from:

       https://github.com/bercly0b/rtg/releases/latest
   ```
4. **Is there a newer release** (`is_update_needed`): the latest stable GitHub
   release is compared with the running version, not with the version in the
   receipt, so `rtg update` agrees with the status bar notice.
5. **Can RTG write to the install directory**: the installer stages the new
   binary in a temporary directory inside the directory of the running binary,
   so RTG first tries the same. On a permission error it prints a ready command
   instead of letting the installer fail:

   ```
   Error: rtg cannot write to /usr/local/bin. Update it as root:

       sudo /usr/local/bin/rtg update
   ```

   The command uses the binary's full path, because `sudo` may not search the
   user's `PATH`.
6. **Install** (`run`): `axoupdater` downloads the installer of the new release
   and runs it with `RTG_TUI_INSTALL_DIR` set to the receipt's install prefix;
   the installer's output goes straight to the terminal. The receipt's
   `modify_path` is passed on, so an install that did not touch shell profiles
   still does not.

`rtg update` does not take the single-instance lock and does not load the
config: the TUI suggests it while running, and replacing the binary on disk
does not affect a running RTG.

## Dependencies

`axoupdater` brings `reqwest` with rustls on `aws-lc-rs` (the update check's
`ureq` uses rustls on `ring`), `miette`, `axoasset` and their dependencies:
68 more crates in the dependency tree (212 → 280), all accepted by
`cargo deny check`. Its async API runs on a current-thread Tokio runtime; the
crate's `blocking` feature is not used because it turns on every Tokio
feature.

Binary size, `dist` profile, `aarch64-apple-darwin`:

| | Binary | `.tar.xz` |
|---|---|---|
| without `axoupdater` | 43.7 MiB | 10.5 MiB |
| with `axoupdater` | 47.1 MiB (+3.4 MiB, +7.8%) | 11.4 MiB (+0.9 MiB) |
