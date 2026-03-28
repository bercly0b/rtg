use super::types::{AuthStateUpdate, TdLibError};
use super::TdLibClient;

impl TdLibClient {
    /// Receives the next authorization state update.
    ///
    /// Blocks until an auth state update is received or timeout expires.
    pub fn recv_auth_state(
        &self,
        timeout: std::time::Duration,
    ) -> Result<AuthStateUpdate, TdLibError> {
        let rx = self.auth_state_rx.lock().map_err(|_| TdLibError::Init {
            message: "auth state receiver lock poisoned".to_owned(),
        })?;
        rx.recv_timeout(timeout).map_err(|_| TdLibError::Timeout {
            message: "waiting for authorization state".to_owned(),
        })
    }

    /// Sends TDLib parameters to initialize the client.
    ///
    /// This should be called when receiving `AuthorizationState::WaitTdlibParameters`.
    pub fn set_tdlib_parameters(&self) -> Result<(), TdLibError> {
        let config = &self.config;
        let client_id = self.client_id;

        let database_directory = config
            .database_directory
            .to_str()
            .ok_or_else(|| TdLibError::Init {
                message: "database directory path is not valid UTF-8".to_owned(),
            })?
            .to_owned();

        let files_directory = config
            .files_directory
            .to_str()
            .ok_or_else(|| TdLibError::Init {
                message: "files directory path is not valid UTF-8".to_owned(),
            })?
            .to_owned();

        self.rt.block_on(async {
            tdlib_rs::functions::set_tdlib_parameters(
                false, // use_test_dc
                database_directory,
                files_directory,
                String::new(), // files_directory (deprecated parameter, use empty)
                true,          // use_file_database
                true,          // use_chat_info_database
                true,          // use_message_database
                false,         // use_secret_chats
                config.api_id,
                config.api_hash.clone(),
                "en".to_owned(),                      // system_language_code
                "RTG".to_owned(),                     // device_model
                String::new(),                        // system_version
                env!("CARGO_PKG_VERSION").to_owned(), // application_version
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Init { message: e.message })
        })
    }

    /// Requests a login code to be sent to the given phone number.
    pub fn set_authentication_phone_number(&self, phone: &str) -> Result<(), TdLibError> {
        let phone = phone.to_owned();
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::set_authentication_phone_number(phone, None, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }

    /// Checks the authentication code entered by the user.
    pub fn check_authentication_code(&self, code: &str) -> Result<(), TdLibError> {
        let code = code.to_owned();
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::check_authentication_code(code, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }

    /// Checks the 2FA password.
    pub fn check_authentication_password(&self, password: &str) -> Result<(), TdLibError> {
        let password = password.to_owned();
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::check_authentication_password(password, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }
}
