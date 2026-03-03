//! TDLib authentication backend.
//!
//! Implements the `TelegramAuthClient` trait using TDLib for authentication.
//! Handles the TDLib authorization state machine:
//! - WaitTdlibParameters → set_tdlib_parameters
//! - WaitPhoneNumber → set_authentication_phone_number
//! - WaitCode → check_authentication_code
//! - WaitPassword → check_authentication_password
//! - Ready → authorization complete

use std::path::Path;
use std::time::Duration;

use tdlib_rs::enums::AuthorizationState;

use crate::domain::status::AuthConnectivityStatus;
use crate::infra::config::TelegramConfig;
use crate::infra::storage_layout::StorageLayout;
use crate::usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome};

use super::tdlib_client::{TdLibClient, TdLibConfig, TdLibError};

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
    /// This tracks state changes to avoid missing updates that arrive
    /// before we start waiting.
    last_auth_state: Option<AuthorizationState>,
}

impl TdLibAuthBackend {
    /// Creates a new TDLib auth backend.
    ///
    /// This creates the TDLib client and waits for initial authorization state.
    pub fn new(config: &TelegramConfig, layout: &StorageLayout) -> Result<Self, AuthBackendError> {
        let tdlib_config = TdLibConfig {
            api_id: config.api_id,
            api_hash: config.api_hash.clone(),
            database_directory: layout.tdlib_database_dir(),
            files_directory: layout.tdlib_files_dir(),
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

        // Wait for WaitTdlibParameters state
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
                // Don't cache this state, we need to wait for the next one
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
                // Cache this state for the next operation
                self.last_auth_state = Some(update.state);
                self.initialized = true;
                Ok(())
            }
        }
    }

    /// Waits for the next authorization state, with timeout.
    ///
    /// Updates the cached `last_auth_state` to prevent race conditions.
    fn wait_for_auth_state(&mut self) -> Result<AuthorizationState, AuthBackendError> {
        let update = self
            .client
            .recv_auth_state(AUTH_STATE_TIMEOUT)
            .map_err(map_tdlib_error)?;
        self.last_auth_state = Some(update.state.clone());
        Ok(update.state)
    }

    /// Takes the cached auth state if available, otherwise waits for next state.
    ///
    /// This prevents race conditions where state updates arrive before we start waiting.
    fn take_or_wait_for_auth_state(&mut self) -> Result<AuthorizationState, AuthBackendError> {
        if let Some(state) = self.last_auth_state.take() {
            return Ok(state);
        }
        self.wait_for_auth_state()
    }

    /// Checks if we're already authorized (from cached session).
    pub fn is_authorized(&mut self) -> Result<bool, AuthBackendError> {
        // Check cached state first
        if let Some(ref state) = self.last_auth_state {
            return Ok(matches!(state, AuthorizationState::Ready));
        }

        // Try to receive state without blocking long
        match self.client.recv_auth_state(Duration::from_millis(100)) {
            Ok(update) => {
                let is_ready = matches!(update.state, AuthorizationState::Ready);
                self.last_auth_state = Some(update.state);
                Ok(is_ready)
            }
            Err(_) => Ok(false), // Timeout is not an error here
        }
    }

    /// Requests a login code for the given phone number.
    pub fn request_login_code(&mut self, phone: &str) -> Result<AuthCodeToken, AuthBackendError> {
        // Use cached state or wait for WaitPhoneNumber state
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

        // Generate a token for this code request
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
        // Verify token matches
        if self.current_code_token.as_ref() != Some(token) {
            return Err(AuthBackendError::Transient {
                code: "AUTH_INVALID_FLOW",
                message: "code submission token does not match active login request".to_owned(),
            });
        }

        // Use cached state or wait for WaitCode state
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

        // Wait for result state (always wait fresh after action)
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

        // Wait for result state
        let state = self.wait_for_auth_state()?;

        match state {
            AuthorizationState::Ready => {
                self.current_code_token = None;
                Ok(())
            }
            AuthorizationState::WaitPassword(_) => {
                // Still waiting for password, means wrong password
                Err(AuthBackendError::WrongPassword)
            }
            other => {
                tracing::warn!(?other, "Unexpected auth state after password");
                Err(AuthBackendError::Transient {
                    code: "AUTH_UNEXPECTED_STATE",
                    message: format!("unexpected state after password: {other:?}"),
                })
            }
        }
    }

    /// Persists the authorized session.
    ///
    /// Note: TDLib handles session persistence automatically.
    /// This method exists for API compatibility with the trait.
    pub fn persist_authorized_session(&self, _session_path: &Path) -> Result<(), AuthBackendError> {
        // TDLib automatically persists the session to its database directory.
        // No additional action needed.
        tracing::debug!("Session persistence handled by TDLib automatically");
        Ok(())
    }

    /// Returns the current authentication status snapshot.
    pub fn auth_status_snapshot(&self) -> Option<AuthConnectivityStatus> {
        // For now, return None. Full status tracking will be added
        // when integrating with StatusTracker.
        None
    }

    /// Disconnects and resets the auth state.
    pub fn disconnect_and_reset(&mut self) {
        self.current_code_token = None;
        self.last_auth_state = None;
        // Note: We don't close the TDLib client here, as it may be reused.
        // Full reset will happen on logout or app restart.
    }

    /// Returns the underlying TDLib client.
    pub fn client(&self) -> &TdLibClient {
        &self.client
    }

    /// Returns mutable reference to the underlying TDLib client.
    pub fn client_mut(&mut self) -> &mut TdLibClient {
        &mut self.client
    }
}

/// Maps TDLib initialization error to AuthBackendError.
fn map_init_error(error: TdLibError) -> AuthBackendError {
    AuthBackendError::Transient {
        code: "AUTH_BACKEND_UNAVAILABLE",
        message: format!("TDLib initialization failed: {error}"),
    }
}

/// Maps TDLib error to AuthBackendError.
fn map_tdlib_error(error: TdLibError) -> AuthBackendError {
    match error {
        TdLibError::Timeout { .. } => AuthBackendError::Timeout,
        TdLibError::Init { message } | TdLibError::Request { message } => {
            AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message,
            }
        }
        TdLibError::Shutdown { message } => AuthBackendError::Transient {
            code: "AUTH_BACKEND_CLOSED",
            message,
        },
    }
}

/// Maps phone number request error.
fn map_request_code_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("phone") && msg_lower.contains("invalid") {
        return AuthBackendError::InvalidPhone;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_REQUEST_CODE_FAILED",
        message,
    }
}

/// Maps sign-in error.
fn map_sign_in_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("code")
        && (msg_lower.contains("invalid") || msg_lower.contains("expired"))
    {
        return AuthBackendError::InvalidCode;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_SIGN_IN_FAILED",
        message,
    }
}

/// Maps password verification error.
fn map_password_error(error: TdLibError) -> AuthBackendError {
    let TdLibError::Request { message } = error else {
        return map_tdlib_error(error);
    };

    let msg_lower = message.to_ascii_lowercase();

    if msg_lower.contains("password") && msg_lower.contains("invalid") {
        return AuthBackendError::WrongPassword;
    }

    if let Some(seconds) = parse_flood_wait_seconds(&msg_lower) {
        return AuthBackendError::FloodWait { seconds };
    }

    AuthBackendError::Transient {
        code: "AUTH_PASSWORD_VERIFY_FAILED",
        message,
    }
}

/// Extracts flood wait seconds from error message.
fn parse_flood_wait_seconds(message: &str) -> Option<u32> {
    let msg_lower = message.to_ascii_lowercase();
    if !msg_lower.contains("flood") {
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
    fn parse_flood_wait_extracts_seconds() {
        assert_eq!(parse_flood_wait_seconds("flood_wait_67"), Some(67));
        assert_eq!(parse_flood_wait_seconds("FLOOD_WAIT_120"), Some(120));
        assert_eq!(parse_flood_wait_seconds("no flood here"), None);
        assert_eq!(parse_flood_wait_seconds("other error"), None);
    }

    #[test]
    fn map_request_code_error_detects_invalid_phone() {
        let error = TdLibError::Request {
            message: "PHONE_NUMBER_INVALID".to_owned(),
        };
        assert_eq!(
            map_request_code_error(error),
            AuthBackendError::InvalidPhone
        );
    }

    #[test]
    fn map_sign_in_error_detects_invalid_code() {
        let error = TdLibError::Request {
            message: "PHONE_CODE_INVALID".to_owned(),
        };
        assert_eq!(map_sign_in_error(error), AuthBackendError::InvalidCode);
    }

    #[test]
    fn map_password_error_detects_wrong_password() {
        let error = TdLibError::Request {
            message: "PASSWORD_HASH_INVALID".to_owned(),
        };
        assert_eq!(map_password_error(error), AuthBackendError::WrongPassword);
    }

    #[test]
    fn map_flood_wait_in_request_code() {
        let error = TdLibError::Request {
            message: "FLOOD_WAIT_300".to_owned(),
        };
        assert_eq!(
            map_request_code_error(error),
            AuthBackendError::FloodWait { seconds: 300 }
        );
    }
}
