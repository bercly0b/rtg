use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;
use tokio::runtime::Builder;

use crate::{
    infra::config::TelegramConfig,
    usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome},
};

pub(super) struct GrammersAuthBackend {
    rt: tokio::runtime::Runtime,
    client: Client,
    login_token: Option<grammers_client::types::LoginToken>,
    password_token: Option<grammers_client::types::PasswordToken>,
}

impl GrammersAuthBackend {
    pub(super) fn new(config: &TelegramConfig) -> Result<Self, AuthBackendError> {
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
                    session: Session::new(),
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
        })
    }

    pub(super) fn request_login_code(
        &mut self,
        phone: &str,
    ) -> Result<AuthCodeToken, AuthBackendError> {
        let login_token = self
            .rt
            .block_on(self.client.request_login_code(phone))
            .map_err(map_request_code_error)?;

        self.login_token = Some(login_token);
        self.password_token = None;

        Ok(AuthCodeToken("code-requested".to_owned()))
    }

    pub(super) fn sign_in_with_code(
        &mut self,
        _token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        let login_token = self.login_token.take().ok_or(AuthBackendError::Transient {
            code: "AUTH_INVALID_FLOW",
            message: "login code request token is missing".to_owned(),
        })?;

        let result = self.rt.block_on(self.client.sign_in(&login_token, code));

        match result {
            Ok(_) => {
                self.password_token = None;
                Ok(SignInOutcome::Authorized)
            }
            Err(SignInError::PasswordRequired(password_token)) => {
                self.password_token = Some(password_token);
                Ok(SignInOutcome::PasswordRequired)
            }
            Err(error) => Err(map_sign_in_error(error)),
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

        self.rt
            .block_on(self.client.check_password(password_token, password))
            .map(|_| ())
            .map_err(map_password_error)
    }
}

fn map_connect_error(error: impl std::fmt::Display) -> AuthBackendError {
    AuthBackendError::Transient {
        code: "AUTH_BACKEND_UNAVAILABLE",
        message: format!("telegram backend connection failed: {error}"),
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
    let msg = error.to_string().to_ascii_lowercase();

    if msg.contains("code") {
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
}
