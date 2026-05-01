use std::io;

use crate::domain::status::AuthConnectivityStatus;

use super::{AuthBackendError, AuthCodeToken, SignInOutcome};

pub trait TelegramAuthClient {
    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError>;
    fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError>;
    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError>;

    fn auth_status_snapshot(&self) -> Option<AuthConnectivityStatus> {
        None
    }
}

pub trait AuthTerminal {
    fn print_line(&mut self, line: &str) -> io::Result<()>;
    fn prompt_line(&mut self, prompt: &str) -> io::Result<Option<String>>;
    fn prompt_secret(&mut self, prompt: &str) -> io::Result<Option<String>>;

    /// Returns true when the terminal is in verbose mode (logging level >= debug).
    /// Diagnostic output gated on verbosity (e.g. status snapshots) checks this.
    fn is_verbose(&self) -> bool {
        false
    }
}
