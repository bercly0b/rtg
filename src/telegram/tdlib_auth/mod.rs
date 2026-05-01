//! TDLib authentication backend.
//!
//! Implements the `TelegramAuthClient` trait using TDLib for authentication.
//! Handles the TDLib authorization state machine:
//! - WaitTdlibParameters → set_tdlib_parameters
//! - WaitPhoneNumber → set_authentication_phone_number
//! - WaitCode → check_authentication_code
//! - WaitPassword → check_authentication_password
//! - Ready → authorization complete

mod chat_details;
mod chat_list;
mod error_mapping;
mod message_details;
mod messages;
mod reactions;

use std::time::Duration;

use tdlib_rs::enums::AuthorizationState;

use crate::domain::status::AuthConnectivityStatus;
use crate::infra::config::TelegramConfig;
use crate::infra::storage_layout::StorageLayout;
use crate::usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome};

use super::tdlib_client::{TdLibClient, TdLibConfig, TdLibError};

use error_mapping::{
    map_init_error, map_password_error, map_request_code_error, map_sign_in_error, map_tdlib_error,
};

/// Default timeout for waiting on authorization state changes.
const AUTH_STATE_TIMEOUT: Duration = Duration::from_secs(30);

/// TDLib-based authentication backend.
///
/// Manages the TDLib client and handles the authorization flow.
pub struct TdLibAuthBackend {
    client: TdLibClient,
    /// Current auth state token for code submission
    current_code_token: Option<AuthCodeToken>,
    /// Counter for generating unique tokens
    next_code_token_id: u64,
    /// Whether we've completed initialization (set_tdlib_parameters)
    initialized: bool,
    /// Cached last known authorization state for race condition prevention.
    last_auth_state: Option<AuthorizationState>,
}

impl TdLibAuthBackend {
    /// Creates a new TDLib auth backend.
    ///
    /// This creates the TDLib client and waits for initial authorization state.
    pub fn new(
        config: &TelegramConfig,
        layout: &StorageLayout,
        verbose: bool,
    ) -> Result<Self, AuthBackendError> {
        let tdlib_config = TdLibConfig {
            api_id: config.api_id,
            api_hash: config.api_hash.clone(),
            database_directory: layout.tdlib_database_dir(),
            files_directory: layout.tdlib_files_dir(),
            log_file: layout.tdlib_log_file(),
            verbose,
        };

        // Ensure directories exist
        std::fs::create_dir_all(&tdlib_config.database_directory).map_err(|e| {
            AuthBackendError::Transient {
                code: "AUTH_STORAGE_UNAVAILABLE",
                message: format!("failed to create TDLib database directory: {e}"),
            }
        })?;
        std::fs::create_dir_all(&tdlib_config.files_directory).map_err(|e| {
            AuthBackendError::Transient {
                code: "AUTH_STORAGE_UNAVAILABLE",
                message: format!("failed to create TDLib files directory: {e}"),
            }
        })?;
        if let Some(log_parent) = tdlib_config.log_file.parent() {
            std::fs::create_dir_all(log_parent).map_err(|e| AuthBackendError::Transient {
                code: "AUTH_STORAGE_UNAVAILABLE",
                message: format!("failed to create TDLib log directory: {e}"),
            })?;
        }

        let client = TdLibClient::new(tdlib_config).map_err(map_init_error)?;

        let mut backend = Self {
            client,
            current_code_token: None,
            next_code_token_id: 1,
            initialized: false,
            last_auth_state: None,
        };

        // Wait for initial WaitTdlibParameters and initialize
        backend.ensure_initialized()?;

        Ok(backend)
    }

    /// Ensures TDLib is initialized with parameters.
    fn ensure_initialized(&mut self) -> Result<(), AuthBackendError> {
        if self.initialized {
            return Ok(());
        }

        let update = self
            .client
            .recv_auth_state(AUTH_STATE_TIMEOUT)
            .map_err(map_tdlib_error)?;

        match &update.state {
            AuthorizationState::WaitTdlibParameters => {
                tracing::debug!("Received WaitTdlibParameters, setting parameters");
                self.client
                    .set_tdlib_parameters()
                    .map_err(map_tdlib_error)?;
                self.initialized = true;
                Ok(())
            }
            AuthorizationState::Ready => {
                tracing::info!("TDLib already authorized from cached session");
                self.initialized = true;
                self.last_auth_state = Some(update.state);
                Ok(())
            }
            AuthorizationState::Closed => Err(AuthBackendError::Transient {
                code: "AUTH_BACKEND_CLOSED",
                message: "TDLib client was closed unexpectedly".to_owned(),
            }),
            other => {
                tracing::warn!(?other, "Unexpected initial auth state");
                self.last_auth_state = Some(update.state);
                self.initialized = true;
                Ok(())
            }
        }
    }

    /// Waits for the next authorization state, with timeout.
    fn wait_for_auth_state(&mut self) -> Result<AuthorizationState, AuthBackendError> {
        let update = self
            .client
            .recv_auth_state(AUTH_STATE_TIMEOUT)
            .map_err(map_tdlib_error)?;
        self.last_auth_state = Some(update.state.clone());
        Ok(update.state)
    }

    /// Takes the cached auth state if available, otherwise waits for next state.
    fn take_or_wait_for_auth_state(&mut self) -> Result<AuthorizationState, AuthBackendError> {
        if let Some(state) = self.last_auth_state.take() {
            return Ok(state);
        }
        self.wait_for_auth_state()
    }

    /// Checks if we're already authorized (from cached session).
    pub fn is_authorized(&mut self) -> Result<bool, AuthBackendError> {
        if let Some(ref state) = self.last_auth_state {
            return Ok(matches!(state, AuthorizationState::Ready));
        }

        match self.client.recv_auth_state(Duration::from_millis(100)) {
            Ok(update) => {
                let is_ready = matches!(update.state, AuthorizationState::Ready);
                self.last_auth_state = Some(update.state);
                Ok(is_ready)
            }
            Err(TdLibError::Timeout { .. }) => Ok(false),
            Err(other) => Err(map_tdlib_error(other)),
        }
    }

    /// Requests a login code for the given phone number.
    pub fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        let state = self.take_or_wait_for_auth_state()?;

        match state {
            AuthorizationState::WaitPhoneNumber => {
                tracing::debug!("Sending phone number to TDLib");
            }
            AuthorizationState::Ready => {
                return Err(AuthBackendError::Transient {
                    code: "AUTH_ALREADY_AUTHORIZED",
                    message: "already authorized".to_owned(),
                });
            }
            other => {
                tracing::warn!(?other, "Unexpected auth state when requesting code");
            }
        }

        self.client
            .set_authentication_phone_number(phone)
            .map_err(map_request_code_error)?;

        let token = AuthCodeToken(format!("tdlib-code-{}", self.next_code_token_id));
        self.next_code_token_id += 1;
        self.current_code_token = Some(token.clone());

        Ok(token)
    }

    /// Signs in with the authentication code.
    pub fn sign_in_with_code(
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

        let state = self.take_or_wait_for_auth_state()?;

        match state {
            AuthorizationState::WaitCode(_) => {
                tracing::debug!("Submitting authentication code");
            }
            AuthorizationState::Ready => {
                self.current_code_token = None;
                return Ok(SignInOutcome::Authorized);
            }
            other => {
                tracing::warn!(?other, "Unexpected auth state when submitting code");
            }
        }

        self.client
            .check_authentication_code(code)
            .map_err(map_sign_in_error)?;

        let result_state = self.wait_for_auth_state()?;

        match result_state {
            AuthorizationState::Ready => {
                self.current_code_token = None;
                Ok(SignInOutcome::Authorized)
            }
            AuthorizationState::WaitPassword(_) => {
                tracing::debug!("2FA password required");
                Ok(SignInOutcome::PasswordRequired)
            }
            AuthorizationState::WaitRegistration(_) => Err(AuthBackendError::Transient {
                code: "AUTH_REGISTRATION_REQUIRED",
                message: "account registration is not supported".to_owned(),
            }),
            other => {
                tracing::warn!(?other, "Unexpected auth state after code submission");
                Err(AuthBackendError::Transient {
                    code: "AUTH_UNEXPECTED_STATE",
                    message: format!("unexpected state after code: {other:?}"),
                })
            }
        }
    }

    /// Verifies the 2FA password.
    pub fn verify_password(&mut self, password: &str) -> Result<(), AuthBackendError> {
        self.client
            .check_authentication_password(password)
            .map_err(map_password_error)?;

        let state = self.wait_for_auth_state()?;

        match state {
            AuthorizationState::Ready => {
                self.current_code_token = None;
                Ok(())
            }
            AuthorizationState::WaitPassword(_) => Err(AuthBackendError::WrongPassword),
            other => {
                tracing::warn!(?other, "Unexpected auth state after password");
                Err(AuthBackendError::Transient {
                    code: "AUTH_UNEXPECTED_STATE",
                    message: format!("unexpected state after password: {other:?}"),
                })
            }
        }
    }

    /// Returns the current authentication status snapshot.
    #[allow(dead_code)]
    pub fn auth_status_snapshot(&self) -> Option<AuthConnectivityStatus> {
        None
    }

    /// Disconnects and resets the auth state.
    pub fn disconnect_and_reset(&mut self) {
        self.current_code_token = None;
        self.last_auth_state = None;
    }

    /// Returns the underlying TDLib client.
    #[allow(dead_code)]
    pub fn client(&self) -> &TdLibClient {
        &self.client
    }

    /// Returns mutable reference to the underlying TDLib client.
    #[allow(dead_code)]
    pub fn client_mut(&mut self) -> &mut TdLibClient {
        &mut self.client
    }

    /// Takes the typed update receiver from the underlying TDLib client.
    pub fn take_update_receiver(
        &self,
    ) -> Option<std::sync::mpsc::Receiver<super::tdlib_updates::TdLibUpdate>> {
        self.client.take_update_receiver()
    }
}

#[cfg(test)]
mod tests;
