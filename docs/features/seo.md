# Repository discoverability (GitHub SEO)

How to make people who look for a terminal Telegram client land on
<https://github.com/bercly0b/rtg>. Research notes and a prioritized action plan.

## Scope

In scope: the GitHub repository itself — its ranking in GitHub search, the
ranking of the repository page in Google, and third-party pages that link to
it. Target audience: English-speaking users.

Out of scope, tracked as separate tasks (see [Follow-ups](#follow-ups)): a
project website, crates.io, Homebrew and other package managers, a Russian
README.

## Current state (2026-10-27)

- The name `rtg` is a three-letter acronym shared with unrelated, established
  terms (radioisotope thermoelectric generator, Real Time Gaming). On its own
  it will not rank; it has to appear next to "Telegram".
- The About block (description, website, topics) is being filled in ROZ-100.
- A web search for `bercly0b rtg telegram` returns nothing about the project:
  the repository page is not indexed or not ranked yet.
- README before this change: the H1 is just "RTG", the first sentence called
  RTG a "TUI client built on top of TDLib" without naming Telegram, and the
  demo GIF alt text was "RTG demo".
- RTG is distributed only through GitHub Releases: no crates.io, docs.rs,
  lib.rs or package managers.
- Release notes are auto-generated from the commit log
  (`generate_release_notes: true` in
  [`release.yml`](../../.github/workflows/release.yml)).

### Competitors

A web search for `terminal Telegram client TUI rust` returns two Rust projects,
and most hits are third-party pages, not GitHub:

| Project | Stack | Found via |
|---|---|---|
| [tgt](https://github.com/FedericoBruzzone/tgt) | Rust, TDLib, ratatui, ~1k stars | GitHub, docs.rs, lib.rs, Terminal Trove |
| vimgram | Rust, ratatui, grammers | docs.rs |

Other well-known clients in the niche: nchat (C++), tg (Python).

Takeaway: in this niche people find projects mostly through package registries
and curated catalogs that link to the repository.

## How a repository gets found

1. **GitHub search.** Repository search matches only the name, description and
   topics by default; the README is searched only with `in:readme`
   ([docs](https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories)).
   Popularity (stars) affects the ranking. So the About description and topics
   are the main lever for GitHub search, and README text does not help there.
2. **Topic pages.** `github.com/topics/<topic>` lists public repositories with
   that topic. Up to 20 topics per repository: lowercase letters, digits and
   hyphens, 50 characters max
   ([docs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics)).
3. **Google.** The page title is `GitHub - bercly0b/rtg: <description>` and the
   description is also the meta description, so the About description is the
   snippet people see in results. Google shows roughly the first 60 characters
   of a title and the prefix takes 23 of them, so the key words must be in the
   first ~35 characters of the description. The README is the page body: its
   headings, opening paragraph and image alt text are the main on-page signals.
   Google discovers the page mostly through links from already-indexed pages
   (catalogs, awesome lists, posts).
4. **Link previews.** When the link is shared in chats or social networks,
   GitHub serves the social preview image. A custom one should be 1280×640,
   PNG/JPG/GIF, under 1 MB
   ([docs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview)).

GitHub Wiki does not help: search engines index wikis only for repositories
with 500+ stars and public editing disabled
([docs](https://docs.github.com/en/communities/documenting-your-project-with-wikis/about-wikis)).

## Target queries

- Category: `telegram tui`, `terminal telegram client`, `telegram cli client`,
  `rust telegram client`, `vim telegram client`.
- Brand: `rtg telegram`, `rtg telegram client`.

Rule of thumb: wherever "RTG" appears (description, README, release titles,
posts), "Telegram" and "terminal"/"TUI" should appear next to it.

## Action plan

P0 — now, cheap, highest impact. P1 — next. P2 — once the project has some
traction.

### P0 — README (done)

- The first sentence names the category: "RTG is a fast, keyboard-driven
  Telegram client for the terminal, built on top of TDLib."
- The demo GIF alt text describes what the GIF shows instead of "RTG demo".
- The H1 and the tagline stay as they are: the tagline already reads "Fast,
  vim-like TUI client for Telegram, written in Rust."

### P0 — About block (ROZ-100)

Requirements for the texts prepared in ROZ-100:

- **Description:** keywords first — "Telegram", "TUI"/"terminal", "vim",
  "Rust". The README tagline fits: "Fast, vim-like TUI client for Telegram"
  is within the first 40 characters.
- **Website:** the latest release page.
- **Topics:** use most of the 20 slots. On top of `telegram`,
  `telegram-client`, `tui`, `terminal`, `cli`, `rust`, `ratatui`, `tdlib`,
  `vim`, worth adding: `tui-app`, `terminal-app`, `command-line`, `messenger`,
  `keyboard-driven`, `crossterm`, `linux`, `macos`.

### P1 — Repository page

- Upload a custom social preview (Settings → General → Social preview): a
  chat-list frame from the demo GIF with "RTG — Telegram in your terminal".
- Pin the repository on the `bercly0b` profile.
- Start each GitHub Release with a few human-written highlights above the
  auto-generated list: release pages are indexed, and commit subjects alone
  rarely contain the target queries.

### P1 — Catalogs and awesome lists

Each entry is a backlink from a page Google already trusts and a direct
source of visitors.

| Where | How | Prerequisite |
|---|---|---|
| [ratatui/awesome-ratatui](https://github.com/ratatui/awesome-ratatui) | PR to the Apps section: link, short description, GIF | — |
| [rothgar/awesome-tuis](https://github.com/rothgar/awesome-tuis) | PR | — |
| [AlternativeTo](https://alternativeto.net/) | Add RTG as an alternative to Telegram Desktop, tgt, nchat | — |
| [ebertti/awesome-telegram](https://github.com/ebertti/awesome-telegram) | PR if the list has a clients section | — |
| [Terminal Trove](https://terminaltrove.com/submit/) | Submission form: preview image, per-platform install instructions | Available from a package manager (Homebrew task) |
| [rust-unofficial/awesome-rust](https://github.com/rust-unofficial/awesome-rust) | PR to Applications | 50+ stars or 2000+ crates.io downloads |

### P1 — Launch posts

One post per channel, with the GIF and the repository link: r/rust,
r/commandline, Show HN. A published post can then be submitted to This Week in
Rust. Better done after the About block and `brew install` are in place, so
visitors can try RTG with one command.

### P2 — Community signals

- Mark a few real issues `good first issue` / `help wanted`: GitHub surfaces
  them on the repository's `/contribute` page and aggregators pick them up.
- Add issue templates and a code of conduct to complete the community profile
  (Insights → Community standards).

### Not recommended now

- **Renaming the repository** (e.g. `rtg-telegram`): improves name matching,
  but the description and topics already cover "telegram", while a rename
  changes the brand. Revisit only if the measurements below do not improve.
- **GitHub Wiki:** not indexed at the current star count.
- **Own website / GitHub Pages:** the only way to get Google Search Console
  data and full control over titles, but out of scope for now.

## Measuring

- Insights → Traffic: views, unique visitors, referring sites. GitHub keeps
  only the last 14 days, so record the numbers weekly.
- Stars over time.
- Monthly check in a private browser window: Google
  `site:github.com/bercly0b/rtg` (is the page indexed?) and the target queries;
  GitHub search for `telegram tui` and `telegram client`.

## Follow-ups

Tracked as separate tasks:

- **crates.io.** The name `rtg` is taken by an empty 0.1.0 placeholder from
  2020, so publishing needs another package name (e.g. `rtg-tui`; the binary
  can still be `rtg`) or a name transfer from the current owner. Also needs
  `keywords`, `categories`, `homepage` and `readme` in `Cargo.toml`, and a
  check that `cargo install` works with the `download-tdlib` feature. A
  crates.io release also creates docs.rs and lib.rs pages — that is where
  competitors show up in search.
- **Homebrew.** `brew install` is a prerequisite for Terminal Trove and makes
  launch posts convert better. Only homebrew-core formulae get a page on
  formulae.brew.sh; a personal tap does not.
- **Russian README.** `README.ru.md` with a language switcher at the top of
  both READMEs. For the Russian-speaking audience, a Habr post plays the role
  of the r/rust / Show HN launch.
