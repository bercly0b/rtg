use crate::{
    domain::status::AuthConnectivityStatus,
    usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome, TelegramAuthClient},
};

use super::TelegramAdapter;

impl TelegramAuthClient for TelegramAdapter {
    fn auth_status_snapshot(&self) -> Option<AuthConnectivityStatus> {
        Some(self.status_snapshot())
    }

    fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        self.status_tracker.on_auth_start();

        let result = match self.tdlib_backend.as_mut() {
            Some(backend) => backend.request_login_code(phone),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        if let Err(error) = &result {
            self.status_tracker.on_auth_error(error);
        }

        result
    }

    fn sign_in_with_code(
        &mut self,
        token: &AuthCodeToken,
        code: &str,
    ) -> Result<SignInOutcome, AuthBackendError> {
        let result = match self.tdlib_backend.as_mut() {
            Some(backend) => backend.sign_in_with_code(token, code),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        match &result {
            Ok(SignInOutcome::Authorized) => self.status_tracker.on_auth_success(),
            Ok(SignInOutcome::PasswordRequired) => self.status_tracker.on_auth_password_required(),
            Err(error) => self.status_tracker.on_auth_error(error),
        }

        result
    }

    fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        let result = match self.tdlib_backend.as_mut() {
            Some(backend) => backend.verify_password(password),
            None => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "Telegram auth backend is not configured".into(),
            }),
        };

        match &result {
            Ok(()) => self.status_tracker.on_auth_success(),
            Err(error) => self.status_tracker.on_auth_error(error),
        }

        result
    }
}
