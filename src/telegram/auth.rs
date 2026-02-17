use std::{fs, path::Path};

use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;
use tokio::runtime::Builder;

use crate::{
    infra::config::TelegramConfig,
    usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    Disconnected,
    Connecting,
    CodeRequired,
    PasswordRequired,
    Authorized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLoginTransition {
    pub from: LoginState,
    pub to: LoginState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartLoginError {
    InvalidState { current: LoginState },
    Backend(AuthBackendError),
}

pub(super) struct GrammersAuthBackend {
    rt: tokio::runtime::Runtime,
    client: Client,
    login_token: Option<grammers_client::types::LoginToken>,
    password_token: Option<grammers_client::types::PasswordToken>,
    current_code_token: Option<AuthCodeToken>,
    next_code_token_id: u64,
    state: LoginState,
}

impl GrammersAuthBackend {
    pub(super) fn new(
        config: &TelegramConfig,
        session_path: &Path,
    ) -> Result<Self, AuthBackendError> {
        if let Some(parent) = session_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AuthBackendError::Transient {
                code: "AUTH_SESSION_STORE_UNAVAILABLE",
                message: format!("failed to create session dir: {source}"),
            })?;
        }

        let session = Session::load_file_or_create(session_path).map_err(map_session_load_error)?;

        let rt = Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|error| AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: format!("failed to initialize async runtime: {error}"),
            })?;

        let client = rt
            .block_on(async {
                Client::connect(Config {
                    session,
                    api_id: config.api_id,
                    api_hash: config.api_hash.clone(),
                    params: InitParams::default(),
                })
                .await
            })
            .map_err(map_connect_error)?;

        Ok(Self {
            rt,
            client,
            login_token: None,
            password_token: None,
            current_code_token: None,
            next_code_token_id: 1,
            state: LoginState::Disconnected,
        })
    }

    pub(super) fn start_login(
        &mut self,
        phone: &str,
    ) -> Result<StartLoginTransition, StartLoginError> {
        let from = self.state;
        let to = next_start_login_state(from)?;
        self.state = to;

        let login_token = match self
            .rt
            .block_on(self.client.request_login_code(phone))
            .map_err(map_request_code_error)
        {
            Ok(token) => token,
            Err(error) => {
                self.login_token = None;
                self.password_token = None;
                self.current_code_token = None;
                self.state = LoginState::Disconnected;
                return Err(StartLoginError::Backend(error));
            }
        };

        self.login_token = Some(login_token);
        self.password_token = None;
        self.current_code_token = None;
        self.state = LoginState::CodeRequired;

        Ok(StartLoginTransition {
            from,
            to: self.state,
        })
    }

    pub(super) fn request_login_code(
        &mut self,
        phone: &str,
    ) -> Result<AuthCodeToken, AuthBackendError> {
        self.start_login(phone).map_err(|error| match error {
            StartLoginError::InvalidState { current } => AuthBackendError::Transient {
                code: "AUTH_START_LOGIN_INVALID_STATE",
                message: format!("start-login is not allowed from state {current:?}"),
            },
            StartLoginError::Backend(err) => err,
        })?;

        let token = AuthCodeToken(format!("code-requested-{}", self.next_code_token_id));
        self.next_code_token_id += 1;
        self.current_code_token = Some(token.clone());

        Ok(token)
    }

    pub(super) fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        if self.current_code_token.as_ref() != Some(token) {
            return Err(AuthBackendError::Transient {
                code: "AUTH_INVALID_FLOW",
                message: "code submission token does not match active login request".to_owned(),
            });
        }

        let login_token = self
            .login_token
            .as_ref()
            .ok_or(AuthBackendError::Transient {
                code: "AUTH_INVALID_FLOW",
                message: "login code request token is missing".to_owned(),
            })?;

        self.state = LoginState::Connecting;

        let result = self.rt.block_on(self.client.sign_in(login_token, code));

        match result {
            Ok(_) => {
                self.login_token = None;
                self.current_code_token = None;
                self.password_token = None;
                self.state = LoginState::Authorized;
                Ok(SignInOutcome::Authorized)
            }
            Err(SignInError::PasswordRequired(password_token)) => {
                self.login_token = None;
                self.current_code_token = None;
                self.password_token = Some(password_token);
                self.state = LoginState::PasswordRequired;
                Ok(SignInOutcome::PasswordRequired)
            }
            Err(error) => {
                self.state = LoginState::CodeRequired;
                Err(map_sign_in_error(error))
            }
        }
    }

    pub(super) fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        let password_token = self
            .password_token
            .take()
            .ok_or(AuthBackendError::Transient {
                code: "AUTH_INVALID_FLOW",
                message: "password verification requested before password challenge".to_owned(),
            })?;

        match self
            .rt
            .block_on(self.client.check_password(password_token, password))
        {
            Ok(_) => {
                self.state = LoginState::Authorized;
                Ok(())
            }
            Err(error) => {
                self.state = LoginState::PasswordRequired;
                Err(map_password_error(error))
            }
        }
    }

    pub(super) fn persist_authorized_session(
        &self,
        session_path: &Path,
    ) -> Result<(), AuthBackendError> {
        self.client
            .session()
            .save_to_file(session_path)
            .map_err(|source| AuthBackendError::Transient {
                code: "AUTH_SESSION_PERSIST_FAILED",
                message: format!(
                    "failed to persist session at {}: {source}",
                    session_path.display()
                ),
            })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn state(&self) -> LoginState {
        self.state
    }
}

fn next_start_login_state(current: LoginState) -> Result<LoginState, StartLoginError> {
    match current {
        LoginState::Disconnected => Ok(LoginState::Connecting),
        LoginState::Connecting
        | LoginState::CodeRequired
        | LoginState::PasswordRequired
        | LoginState::Authorized => Err(StartLoginError::InvalidState { current }),
    }
}

fn map_connect_error(error: impl std::fmt::Display) -> AuthBackendError {
    AuthBackendError::Transient {
        code: "AUTH_BACKEND_UNAVAILABLE",
        message: format!("telegram backend connection failed: {error}"),
    }
}

fn map_session_load_error(error: impl std::fmt::Display) -> AuthBackendError {
    AuthBackendError::Transient {
        code: "AUTH_SESSION_LOAD_FAILED",
        message: format!("failed to load existing session: {error}"),
    }
}

fn map_request_code_error(error: impl std::fmt::Display) -> AuthBackendError {
    let msg = error.to_string().to_ascii_lowercase();
    if msg.contains("phone") {
        return AuthBackendError::InvalidPhone;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_REQUEST_CODE_FAILED",
        message: "telegram rejected login code request".to_owned(),
    }
}

fn map_sign_in_error(error: SignInError) -> AuthBackendError {
    match error {
        SignInError::InvalidCode => AuthBackendError::InvalidCode,
        SignInError::Other(other) => {
            let msg = other.to_string().to_ascii_lowercase();

            if is_recoverable_code_error(&msg) {
                return AuthBackendError::InvalidCode;
            }

            if let Some(seconds) = parse_flood_wait_seconds(&msg) {
                return AuthBackendError::FloodWait { seconds };
            }

            AuthBackendError::Transient {
                code: "AUTH_SIGN_IN_FAILED",
                message: "telegram sign-in failed".to_owned(),
            }
        }
        SignInError::InvalidPassword => AuthBackendError::Transient {
            code: "AUTH_SIGN_IN_FAILED",
            message: "telegram sign-in failed".to_owned(),
        },
        SignInError::SignUpRequired { .. } => AuthBackendError::Transient {
            code: "AUTH_SIGN_IN_FAILED",
            message: "telegram sign-in failed".to_owned(),
        },
        SignInError::PasswordRequired(_) => AuthBackendError::Transient {
            code: "AUTH_SIGN_IN_FAILED",
            message: "telegram sign-in failed".to_owned(),
        },
    }
}

fn is_recoverable_code_error(message: &str) -> bool {
    message.contains("invalid code")
        || message.contains("phone_code_invalid")
        || message.contains("phone code invalid")
        || message.contains("phone_code_expired")
        || message.contains("phone code expired")
        || message.contains("code expired")
}

fn map_password_error(error: impl std::fmt::Display) -> AuthBackendError {
    let msg = error.to_string().to_ascii_lowercase();

    if msg.contains("password") {
        return AuthBackendError::WrongPassword;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_PASSWORD_VERIFY_FAILED",
        message: "telegram password verification failed".to_owned(),
    }
}

fn parse_flood_wait_seconds(message: &str) -> Option<u32> {
    let marker = "flood";
    if !message.to_ascii_lowercase().contains(marker) {
        return None;
    }

    message
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| {
            (!part.is_empty())
                .then(|| part.parse::<u32>().ok())
                .flatten()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_invalid_phone_from_message() {
        let err = map_request_code_error("PHONE_NUMBER_INVALID");
        assert_eq!(err, AuthBackendError::InvalidPhone);
    }

    #[test]
    fn extracts_flood_wait_seconds() {
        assert_eq!(parse_flood_wait_seconds("FLOOD_WAIT_67"), Some(67));
    }

    #[test]
    fn maps_session_load_error() {
        let err = map_session_load_error("malformed data");
        assert_eq!(
            err,
            AuthBackendError::Transient {
                code: "AUTH_SESSION_LOAD_FAILED",
                message: "failed to load existing session: malformed data".to_owned(),
            }
        );
    }

    #[test]
    fn maps_sign_in_invalid_code_as_recoverable_error() {
        let err = map_sign_in_error(SignInError::InvalidCode);
        assert_eq!(err, AuthBackendError::InvalidCode);
    }

    #[test]
    fn detects_expired_code_message_as_recoverable_code_error() {
        assert!(is_recoverable_code_error("phone_code_expired"));
        assert!(is_recoverable_code_error("phone code expired"));
    }

    #[test]
    fn start_login_state_transition_is_deterministic_from_disconnected() {
        let next = next_start_login_state(LoginState::Disconnected).expect("valid transition");
        assert_eq!(next, LoginState::Connecting);
    }

    #[test]
    fn start_login_repeated_call_is_rejected_with_typed_error() {
        let err = next_start_login_state(LoginState::CodeRequired)
            .expect_err("repeated start-login should be invalid");

        assert_eq!(
            err,
            StartLoginError::InvalidState {
                current: LoginState::CodeRequired
            }
        );
    }
}
