mod flow;

use std::collections::VecDeque;

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
    fn print_line(&mut self, line: &str) -> std::io::Result<()> {
        self.output.push(line.to_owned());
        Ok(())
    }

    fn prompt_line(&mut self, _prompt: &str) -> std::io::Result<Option<String>> {
        Ok(self.inputs.pop_front().flatten())
    }

    fn prompt_secret(&mut self, _prompt: &str) -> std::io::Result<Option<String>> {
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
    snapshot: Option<crate::domain::status::AuthConnectivityStatus>,
    verified_passwords: Vec<String>,
}

impl FakeClient {
    fn new(actions: Vec<Action>) -> Self {
        Self {
            actions: actions.into(),
            snapshot: None,
            verified_passwords: Vec::new(),
        }
    }

    fn with_snapshot(
        actions: Vec<Action>,
        snapshot: crate::domain::status::AuthConnectivityStatus,
    ) -> Self {
        Self {
            actions: actions.into(),
            snapshot: Some(snapshot),
            verified_passwords: Vec::new(),
        }
    }
}

impl TelegramAuthClient for FakeClient {
    fn auth_status_snapshot(&self) -> Option<crate::domain::status::AuthConnectivityStatus> {
        self.snapshot.clone()
    }

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

    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        self.verified_passwords.push(password.to_owned());
        match self.actions.pop_front().expect("missing verify action") {
            Action::Verify(result) => result,
            _ => panic!("unexpected action order"),
        }
    }
}
