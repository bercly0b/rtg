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
