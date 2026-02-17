use std::{fs, io, path::Path};

use crate::infra::secrets::sanitize_error_code;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub phone_attempts: usize,
    pub code_attempts: usize,
    pub password_attempts: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            phone_attempts: 3,
            code_attempts: 3,
            password_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCodeToken(pub String);

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInOutcome {
    Authorized,
    PasswordRequired,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthBackendError {
    InvalidPhone,
    InvalidCode,
    WrongPassword,
    Timeout,
    FloodWait { seconds: u32 },
    Transient { code: &'static str, message: String },
}

pub trait TelegramAuthClient {
    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError>;
    fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError>;
    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError>;
}

pub trait AuthTerminal {
    fn print_line(&mut self, line: &str) -> io::Result<()>;
    fn prompt_line(&mut self, prompt: &str) -> io::Result<Option<String>>;
    fn prompt_secret(&mut self, prompt: &str) -> io::Result<Option<String>>;
}

pub struct StdTerminal;

impl AuthTerminal for StdTerminal {
    fn print_line(&mut self, line: &str) -> io::Result<()> {
        println!("{line}");
        Ok(())
    }

    fn prompt_line(&mut self, prompt: &str) -> io::Result<Option<String>> {
        use std::io::Write;

        print!("{prompt}");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes = io::stdin().read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }

        Ok(Some(line.trim().to_owned()))
    }

    fn prompt_secret(&mut self, prompt: &str) -> io::Result<Option<String>> {
        match rpassword::prompt_password(prompt) {
            Ok(password) => Ok(Some(password.trim().to_owned())),
            Err(source) if source.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(source) => Err(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidedAuthOutcome {
    Authenticated,
    ExitWithGuidance,
}

pub fn run_guided_auth(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    session_path: &Path,
    retry_policy: &RetryPolicy,
) -> io::Result<GuidedAuthOutcome> {
    terminal.print_line("No valid session found. Starting guided authentication.")?;

    let Some(phone) = collect_phone(terminal, retry_policy.phone_attempts)? else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    let Some(token) = request_code(terminal, auth_client, &phone, retry_policy.phone_attempts)?
    else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    let Some(outcome) = collect_code(terminal, auth_client, &token, retry_policy.code_attempts)?
    else {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    };

    if matches!(outcome, SignInOutcome::PasswordRequired)
        && collect_password(terminal, auth_client, retry_policy.password_attempts)?.is_none()
    {
        return Ok(GuidedAuthOutcome::ExitWithGuidance);
    }

    persist_session_marker(session_path)?;
    terminal.print_line("Authentication successful. Session saved.")?;

    Ok(GuidedAuthOutcome::Authenticated)
}

fn collect_phone(terminal: &mut dyn AuthTerminal, attempts: usize) -> io::Result<Option<String>> {
    for attempt in 1..=attempts {
        terminal.print_line(
            "Step 1/3 — Enter your phone number in international format, e.g. +15551234567.",
        )?;
        let Some(phone) = terminal.prompt_line("Phone: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if !is_valid_phone(&phone) {
            terminal.print_line(&format!(
                "Invalid format. Use + followed by 8-15 digits. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        return Ok(Some(phone));
    }

    terminal
        .print_line("Phone step failed too many times. Please restart rtg and try again later.")?;
    Ok(None)
}

fn request_code(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    phone: &str,
    attempts: usize,
) -> io::Result<Option<AuthCodeToken>> {
    for attempt in 1..=attempts {
        match auth_client.request_login_code(phone) {
            Ok(token) => {
                terminal
                    .print_line("Code has been sent in Telegram. Continue to the next step.")?;
                return Ok(Some(token));
            }
            Err(err) => {
                if !handle_backend_error(terminal, err, attempt, attempts, "phone")? {
                    return Ok(None);
                }
            }
        }
    }

    terminal.print_line("Unable to request login code. Please restart rtg later.")?;
    Ok(None)
}

fn collect_code(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    token: &AuthCodeToken,
    attempts: usize,
) -> io::Result<Option<SignInOutcome>> {
    for attempt in 1..=attempts {
        terminal.print_line("Step 2/3 — Enter the code from Telegram (digits only).")?;
        let Some(code) = terminal.prompt_line("Code: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if !is_valid_code(&code) {
            terminal.print_line(&format!(
                "Invalid code format. Use 3-8 digits. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        match auth_client.sign_in_with_code(token, &code) {
            Ok(outcome) => return Ok(Some(outcome)),
            Err(err) => {
                if !handle_backend_error(terminal, err, attempt, attempts, "code")? {
                    return Ok(None);
                }
            }
        }
    }

    terminal.print_line("Code step failed too many times. Please restart rtg.")?;
    Ok(None)
}

fn collect_password(
    terminal: &mut dyn AuthTerminal,
    auth_client: &mut dyn TelegramAuthClient,
    attempts: usize,
) -> io::Result<Option<()>> {
    for attempt in 1..=attempts {
        terminal.print_line("Step 3/3 — 2FA password is required for this account.")?;
        let Some(password) = terminal.prompt_secret("2FA password: ")? else {
            terminal.print_line("Input cancelled (EOF). Run rtg again to retry.")?;
            return Ok(None);
        };

        if password.trim().is_empty() {
            terminal.print_line(&format!(
                "Password cannot be empty. Attempts left: {}",
                attempts.saturating_sub(attempt)
            ))?;
            continue;
        }

        match auth_client.verify_password(&password) {
            Ok(()) => return Ok(Some(())),
            Err(err) => {
                if !handle_backend_error(terminal, err, attempt, attempts, "2fa")? {
                    return Ok(None);
                }
            }
        }
    }

    terminal.print_line("2FA step failed too many times. Please restart rtg.")?;
    Ok(None)
}

fn handle_backend_error(
    terminal: &mut dyn AuthTerminal,
    error: AuthBackendError,
    attempt: usize,
    max_attempts: usize,
    step: &str,
) -> io::Result<bool> {
    let attempts_left = max_attempts.saturating_sub(attempt);

    match error {
        AuthBackendError::InvalidPhone => {
            terminal.print_line(&format!(
                "AUTH_INVALID_PHONE: Telegram rejected the phone. Check number and retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::InvalidCode => {
            terminal.print_line(&format!(
                "AUTH_INVALID_CODE: The code is incorrect or expired. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::WrongPassword => {
            terminal.print_line(&format!(
                "AUTH_WRONG_2FA: Incorrect 2FA password. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::Timeout => {
            terminal.print_line(&format!(
                "AUTH_TIMEOUT: Request timed out at {step} step. Check network and retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
        AuthBackendError::FloodWait { seconds } => {
            terminal.print_line(&format!(
                "AUTH_FLOOD_WAIT: Too many attempts. Wait about {seconds}s before retrying."
            ))?;
            Ok(false)
        }
        AuthBackendError::Transient { code, .. } => {
            let safe_code = sanitize_error_code(code);
            terminal.print_line(&format!(
                "{safe_code}: temporary authorization issue at {step} step. Please retry. Attempts left: {attempts_left}"
            ))?;
            Ok(attempts_left > 0)
        }
    }
}

fn persist_session_marker(path: &Path) -> io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, b"authorized")?;
    fs::rename(tmp_path, path)
}

fn is_valid_phone(phone: &str) -> bool {
    let digits = phone.strip_prefix('+').unwrap_or_default();
    phone.starts_with('+')
        && (8..=15).contains(&digits.len())
        && digits.chars().all(|ch| ch.is_ascii_digit())
}

fn is_valid_code(code: &str) -> bool {
    (3..=8).contains(&code.len()) && code.chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, env};

    use super::*;

    struct FakeTerminal {
        inputs: VecDeque<Option<String>>,
        output: Vec<String>,
    }

    impl FakeTerminal {
        fn new(inputs: Vec<Option<&str>>) -> Self {
            Self {
                inputs: inputs
                    .into_iter()
                    .map(|item| item.map(|value| value.to_owned()))
                    .collect(),
                output: Vec::new(),
            }
        }
    }

    impl AuthTerminal for FakeTerminal {
        fn print_line(&mut self, line: &str) -> io::Result<()> {
            self.output.push(line.to_owned());
            Ok(())
        }

        fn prompt_line(&mut self, _prompt: &str) -> io::Result<Option<String>> {
            Ok(self.inputs.pop_front().flatten())
        }

        fn prompt_secret(&mut self, _prompt: &str) -> io::Result<Option<String>> {
            Ok(self.inputs.pop_front().flatten())
        }
    }

    enum Action {
        RequestCode(Result<AuthCodeToken, AuthBackendError>),
        SignIn(Result<SignInOutcome, AuthBackendError>),
        Verify(Result<(), AuthBackendError>),
    }

    struct FakeClient {
        actions: VecDeque<Action>,
    }

    impl FakeClient {
        fn new(actions: Vec<Action>) -> Self {
            Self {
                actions: actions.into(),
            }
        }
    }

    impl TelegramAuthClient for FakeClient {
        fn request_login_code(&mut self, _phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
            match self.actions.pop_front().expect("missing request action") {
                Action::RequestCode(result) => result,
                _ => panic!("unexpected action order"),
            }
        }

        fn sign_in_with_code(
            &mut self,
            _token: &AuthCodeToken,
            _code: &str,
        ) -> Result<SignInOutcome, AuthBackendError> {
            match self.actions.pop_front().expect("missing sign in action") {
                Action::SignIn(result) => result,
                _ => panic!("unexpected action order"),
            }
        }

        fn verify_password(&mut self, _password: &str) -> Result<(), AuthBackendError> {
            match self.actions.pop_front().expect("missing verify action") {
                Action::Verify(result) => result,
                _ => panic!("unexpected action order"),
            }
        }
    }

    fn temp_session_path() -> std::path::PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rtg-guided-auth-test-{}-session.dat",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        path
    }

    #[test]
    fn guided_auth_happy_path_with_optional_2fa() {
        let session_path = temp_session_path();
        let mut terminal =
            FakeTerminal::new(vec![Some("+15551234567"), Some("12345"), Some("s3cret")]);
        let mut client = FakeClient::new(vec![
            Action::RequestCode(Ok(AuthCodeToken("token".into()))),
            Action::SignIn(Ok(SignInOutcome::PasswordRequired)),
            Action::Verify(Ok(())),
        ]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::Authenticated);
        assert!(session_path.exists());

        let _ = fs::remove_file(session_path);
    }

    #[test]
    fn invalid_code_retries_then_succeeds() {
        let session_path = temp_session_path();
        let mut terminal =
            FakeTerminal::new(vec![Some("+15551234567"), Some("000"), Some("12345")]);
        let mut client = FakeClient::new(vec![
            Action::RequestCode(Ok(AuthCodeToken("token".into()))),
            Action::SignIn(Err(AuthBackendError::InvalidCode)),
            Action::SignIn(Ok(SignInOutcome::Authorized)),
        ]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::Authenticated);
        assert!(terminal
            .output
            .iter()
            .any(|line| line.contains("AUTH_INVALID_CODE")));

        let _ = fs::remove_file(session_path);
    }

    #[test]
    fn wrong_2fa_exhausts_retries_and_exits_with_guidance() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![
            Some("+15551234567"),
            Some("12345"),
            Some("wrong-1"),
            Some("wrong-2"),
            Some("wrong-3"),
        ]);
        let mut client = FakeClient::new(vec![
            Action::RequestCode(Ok(AuthCodeToken("token".into()))),
            Action::SignIn(Ok(SignInOutcome::PasswordRequired)),
            Action::Verify(Err(AuthBackendError::WrongPassword)),
            Action::Verify(Err(AuthBackendError::WrongPassword)),
            Action::Verify(Err(AuthBackendError::WrongPassword)),
        ]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
        assert!(!session_path.exists());
    }

    #[test]
    fn flood_wait_exits_immediately() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![Some("+15551234567")]);
        let mut client = FakeClient::new(vec![Action::RequestCode(Err(
            AuthBackendError::FloodWait { seconds: 120 },
        ))]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
        assert!(terminal
            .output
            .iter()
            .any(|line| line.contains("AUTH_FLOOD_WAIT")));
    }

    #[test]
    fn eof_cancels_flow_cleanly() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![None]);
        let mut client = FakeClient::new(vec![]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
        assert!(!session_path.exists());
    }

    #[test]
    fn timeout_then_successful_retry() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![Some("+15551234567"), Some("12345")]);
        let mut client = FakeClient::new(vec![
            Action::RequestCode(Err(AuthBackendError::Timeout)),
            Action::RequestCode(Ok(AuthCodeToken("token".into()))),
            Action::SignIn(Ok(SignInOutcome::Authorized)),
        ]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::Authenticated);
        assert!(terminal
            .output
            .iter()
            .any(|line| line.contains("AUTH_TIMEOUT")));

        let _ = fs::remove_file(session_path);
    }

    #[test]
    fn invalid_phone_from_backend_retries_then_exits() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![
            Some("+15551234567"),
            Some("+15551234567"),
            Some("+15551234567"),
        ]);
        let mut client = FakeClient::new(vec![
            Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
            Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
            Action::RequestCode(Err(AuthBackendError::InvalidPhone)),
        ]);

        let result = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy::default(),
        )
        .expect("guided auth should complete");

        assert_eq!(result, GuidedAuthOutcome::ExitWithGuidance);
        assert!(terminal
            .output
            .iter()
            .any(|line| line.contains("AUTH_INVALID_PHONE")));
    }

    #[test]
    fn transient_error_message_is_not_leaked_to_terminal_output() {
        let session_path = temp_session_path();
        let mut terminal = FakeTerminal::new(vec![Some("+15551234567")]);
        let mut client = FakeClient::new(vec![Action::RequestCode(Err(
            AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "password=s3cret code=12345".to_owned(),
            },
        ))]);

        let _ = run_guided_auth(
            &mut terminal,
            &mut client,
            &session_path,
            &RetryPolicy {
                phone_attempts: 1,
                code_attempts: 1,
                password_attempts: 1,
            },
        )
        .expect("guided auth should complete");

        let joined = terminal.output.join("\n");
        assert!(joined.contains("AUTH_BACKEND_UNAVAILABLE"));
        assert!(!joined.contains("password=s3cret"));
        assert!(!joined.contains("code=12345"));
    }
}
