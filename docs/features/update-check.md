# Update check

When the TUI starts, RTG finds out in the background whether a newer release
exists and, if so, shows it in the status bar with the command that updates
this install:

```
 ● Connected                          v0.2.0 available: rtg update  ? for help
```

- **Never blocks startup**: the check runs on its own thread; the notice
  appears whenever the answer arrives.
- **At most once a day**: GitHub is asked only when the last successful check
  is older than 24 hours (`CHECK_INTERVAL` in `usecases/update_check.rs`). In
  between, the latest version from that check is reused, so a known update
  stays in the status bar across restarts. A failed request is not recorded:
  the next start asks again, and meanwhile the cached version (however old) is
  used.
- **Source**: `GET https://api.github.com/repos/bercly0b/rtg/releases/latest`,
  which skips drafts and pre-releases; its `tag_name` minus the `v` prefix is
  compared with the running version. A release is newer when its
  `MAJOR.MINOR.PATCH` is higher, or equal while the running version is a
  pre-release (`0.2.0` is newer than `0.2.0-rc.1`).
- **Cache**: `update_check.toml` in the cache directory
  (`~/.config/rtg/cache/`), with the Unix time of the last check and the
  latest version it found.
- **Turning it off**: `[updates] check = false` in the config.

## Status bar

The notice sits right before `? for help`. `U` installs the update from the
TUI ([Self-update](self-update.md#from-the-tui)); while it runs and once it is
installed, its progress takes the notice's place. A notification is short-lived and
keeps priority: when there is no room for both, the notice is hidden until the
notification expires. On a terminal too narrow for the notice, it is not shown.

## Install channel

The command in the notice depends on the install channel, detected only when
there is an update to show (`infra/update_check.rs`):

| Install channel | Detected by | Command |
|-----------------|-------------|---------|
| Release installer | the install receipt is for the running binary — the check `rtg update` makes ([Self-update](self-update.md)) | `rtg update` |
| Homebrew | the canonical path of the running binary goes through a Homebrew `Cellar` | `brew upgrade rtg` |
| crates.io | the running binary is in `$CARGO_HOME/bin` (`~/.cargo/bin` by default) | `cargo install --locked rtg-tui` |
| unknown | anything else: a build from source, a copied binary, an install from an older release | the release page, `https://github.com/bercly0b/rtg/releases/latest` |

The rules apply in this order. The receipt goes first because the release
installer and `cargo install` share `~/.cargo/bin`, so only the receipt tells
them apart ([ADR 0002](../adr/0002-release-through-cargo-dist.md)).

## HTTP client

The request goes through `ureq`, which tdlib-rs already brings in for
`download-tdlib`, so the update check adds no new crates. It uses a 10-second
timeout and sends `Accept: application/vnd.github+json` and a
`User-Agent: rtg/<version>` header, which the GitHub API requires.
