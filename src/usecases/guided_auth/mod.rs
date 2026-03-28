mod flow;
mod helpers;
mod terminal;
mod traits;
mod types;

pub use flow::run_guided_auth;
pub use terminal::StdTerminal;
pub use traits::{AuthTerminal, TelegramAuthClient};
pub use types::{AuthBackendError, AuthCodeToken, GuidedAuthOutcome, RetryPolicy, SignInOutcome};

#[cfg(test)]
mod tests;
