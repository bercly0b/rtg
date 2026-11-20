# Built-in credentials live in the source

RTG ships its own API credentials so that a user signs in without first registering an application at my.telegram.org (RTG-110). They are committed to the repository, lightly obfuscated, instead of being injected into release binaries from a CI secret: a build from source (`cargo install`, a clone) can only receive credentials that are public, and we want the first run to be the same on every install channel. Telegram treats `api_id` as identifying the application, not the user, and other clients (nchat, paul-nameless/tg, telega.el, Debian's Telegram Desktop) have published their own for years.

## Considered Options

- **CI secret, release binaries only.** Keeps `api_hash` out of the source, but `cargo install` and clone builds keep asking the user for own credentials. Rejected for the uneven first run.
- **Each user brings own credentials** (the state before this decision). Rejected: the detour through my.telegram.org is the main obstacle on first launch.

## Consequences

- Publishing is irreversible: Telegram offers no way to rotate `api_hash` or delete an application, and an account gets a single `api_id`. The credentials are registered on a dedicated project account, not a personal one.
- The obfuscation only keeps scrapers and secret scanners from picking the pair up by pattern; it is not protection, and it should not be "strengthened".
- Own credentials in the config take precedence over the built-in ones. They are the way out for a user if Telegram limits the built-in `api_id`.
- A fork distributed as a separate application is asked to register its own API credentials; nothing enforces it.
- Way out if the built-in `api_id` gets limited or abused: register a new application on a new account, ship it through a CI secret instead of the source, and abandon the published pair.
