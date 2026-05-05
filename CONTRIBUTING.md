# Contributing to RTG

Thanks for your interest in contributing to RTG. This document covers the
basics: how to get the project building, what we expect from a pull request,
and where to find more detailed docs.

## Getting started

### Prerequisites

- Stable Rust toolchain with `cargo`, `rustfmt`, and `clippy`
- Git
- On Linux: libc++ runtime for the prebuilt TDLib
  (`sudo apt install libc++1 libc++abi1 libunwind8`)

TDLib is downloaded automatically on first build via the `download-tdlib`
feature of `tdlib-rs` — no manual install required.

### Setting up

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rtg.git
   cd rtg
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run the test suite:
   ```bash
   cargo test
   ```

4. Optionally copy the example config:
   ```bash
   mkdir -p ~/.config/rtg
   cp config.example.toml ~/.config/rtg/config.toml
   ```

## Development workflow

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

See [`docs/project-structure.md`](docs/project-structure.md) for the full
module layout, and [`docs/hotkeys.md`](docs/hotkeys.md) for the keymap
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

2. Make your changes and run the local quality gate:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo check
   ```

3. Push your branch and open a pull request. Describe the change and link
   any related issues.

### Pull request guidelines

- Keep PRs focused on a single change
- Update documentation if the change affects user-facing behavior
- Include tests for new logic when feasible
- Ensure CI is green before requesting review

### Review anti-loop policy

To avoid endless review cycles:

- Fix **CRITICAL** review findings in the current PR
- Track **MAJOR/MINOR** follow-ups in
  [`docs/features/RTG_REVIEW_BACKLOG.md`](docs/features/RTG_REVIEW_BACKLOG.md)
- Do not expand PR scope with non-critical refactors

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
