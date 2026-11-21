# Contributing to RTG

Thanks for your interest in contributing to RTG. This document covers the
basics: how to get the project building, what we expect from a pull request,
and where to find more detailed docs.

## Getting started

### Prerequisites

- [mise](https://mise.jdx.dev/installing-mise.html), activated in your shell —
  installs the Rust toolchain and `cargo-llvm-cov` pinned in
  [`mise.toml`](mise.toml) and runs the project tasks
- Git
- On Linux: libc++ — TDLib is linked in statically, but its C++ runtime
  comes from the system
  (`sudo apt install libc++-dev libc++abi-dev`)

TDLib is downloaded automatically on first build via the `download-tdlib`
feature of `tdlib-rs` — no manual install required.
Supported targets: Linux x86_64/arm64, macOS Intel/Apple Silicon, Windows
x86_64/arm64. For other platforms see the
[TDLib build instructions](https://tdlib.github.io/td/build.html).

### Setting up

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rtg.git
   cd rtg
   ```

2. Install the pinned tools:
   ```bash
   mise install
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run the test suite:
   ```bash
   mise run test
   ```

5. Optionally copy the example config:
   ```bash
   mkdir -p ~/.config/rtg
   cp config.example.toml ~/.config/rtg/config.toml
   ```

### API credentials

RTG ships with built-in API credentials (`api_id` and `api_hash`), so it signs
in without any setup. To run under your own credentials, get them at
[my.telegram.org/apps](https://my.telegram.org/apps) and set both values in
`~/.config/rtg/config.toml`:

```toml
[telegram]
api_id = 12345
api_hash = "0123456789abcdef0123456789abcdef"
```

Own credentials take precedence over the built-in ones; there is no prompt,
environment variable or CLI flag for them.

If you distribute a fork as a separate application, please register your own
API credentials and replace the built-in pair in
[`src/infra/config/built_in.rs`](src/infra/config/built_in.rs).
The reasoning is in
[ADR 0003](docs/adr/0003-built-in-credentials-in-source.md).

## Development workflow

### Tooling

`mise` pins the toolchain and holds the composite project tasks — the quality
gate, coverage and release (`mise tasks` lists them). Everyday cargo commands
such as `cargo build`, `cargo run` and `cargo build --release` are run directly:
with mise activated, `cargo` already resolves to the pinned toolchain. Without
activation, prefix them with `mise exec --`.

### Code style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Follow standard Rust naming conventions
- Keep changes surgical — touch only what the task requires

### Architecture

RTG keeps clear layer boundaries — please respect them when adding code:

- `src/ui` — terminal rendering and input handling
- `src/domain` — entities, events, and state
- `src/usecases` — application orchestration
- `src/infra` — config, logging, storage, opener adapters
- `src/telegram` — TDLib integration boundary

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full
module layout, and [`docs/HOTKEYS.md`](docs/HOTKEYS.md) for the keymap
architecture.

### Testing

- Add tests for new or changed logic
- Co-locate unit tests beside the code they cover (`#[cfg(test)]` modules)
- Place integration tests under the crate's `tests/` directory

### Commit messages

Use Conventional Commits:

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `refactor:` non-behavioral code change
- `test:` test additions or changes
- `chore:` tooling and maintenance

Commit messages must be written in English.

## Submitting a pull request

1. Create a feature branch from `main`:
   ```bash
   git checkout -b feature/your-feature
   ```

2. Make your changes and run the local quality gate (`fmt`, `clippy`,
   `ast-grep`, `deny`, `test`, `check` — the same tasks CI runs):
   ```bash
   mise run quality
   ```

3. Describe user-visible changes in a changelog fragment,
   `changelog.d/<your-branch>.md`, one English list item per change. A pull
   request without user-visible changes gets the `no-changelog` label
   instead. See [docs/RELEASES.md](docs/RELEASES.md#changelog).

4. Push your branch and open a pull request. Describe the change and link
   any related issues.

### Pull request guidelines

- Keep PRs focused on a single change
- Update documentation if the change affects user-facing behavior
- Include tests for new logic when feasible
- Ensure CI is green before requesting review

## Reporting issues

When opening an issue, please include:

- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs. actual behavior
- Relevant logs or error messages

## License

By contributing, you agree that your contributions are licensed under the
project's license (see [README.md](README.md)).
