//! TDLib client wrapper with lifecycle management.
//!
//! Provides a foundational TDLib client for RTG with:
//! - Client initialization with configuration parameters
//! - Update receiver loop for processing TDLib events
//! - Proper shutdown handling

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tdlib_rs::enums::{AuthorizationState, Update};
use tokio::runtime::Runtime;

/// Polling interval when no TDLib updates are available.
/// 10ms provides responsive update handling without excessive CPU usage.
const UPDATE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Configuration for TDLib client initialization.
#[derive(Clone)]
pub struct TdLibConfig {
    /// Telegram API ID from <https://my.telegram.org>
    pub api_id: i32,
    /// Telegram API hash from <https://my.telegram.org>
    pub api_hash: String,
    /// Directory for TDLib's SQLite database
    pub database_directory: PathBuf,
    /// Directory for downloaded files
    pub files_directory: PathBuf,
}

// Custom Debug implementation to redact sensitive api_hash field.
impl std::fmt::Debug for TdLibConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdLibConfig")
            .field("api_id", &self.api_id)
            .field("api_hash", &"[REDACTED]")
            .field("database_directory", &self.database_directory)
            .field("files_directory", &self.files_directory)
            .finish()
    }
}

/// Error types for TDLib operations.
#[derive(Debug, thiserror::Error)]
pub enum TdLibError {
    /// TDLib initialization error
    #[error("TDLib initialization error: {message}")]
    Init { message: String },

    /// TDLib request error
    #[error("TDLib request error: {message}")]
    Request { message: String },

    /// TDLib shutdown error
    #[error("TDLib shutdown error: {message}")]
    #[allow(dead_code)] // Will be used in error handling
    Shutdown { message: String },

    /// TDLib timeout error
    #[error("TDLib operation timed out: {message}")]
    Timeout { message: String },
}

/// Authorization state change event.
#[derive(Debug, Clone)]
pub struct AuthStateUpdate {
    pub state: AuthorizationState,
}

/// TDLib client with managed lifecycle.
///
/// This wrapper around `tdlib_rs` manages:
/// - Client ID allocation
/// - Dedicated async runtime for TDLib operations
/// - Background update receiver loop
/// - Authorization state channel
/// - Proper shutdown via `close()` function
pub struct TdLibClient {
    client_id: i32,
    config: TdLibConfig,
    rt: Arc<Runtime>,
    auth_state_rx: Mutex<mpsc::Receiver<AuthStateUpdate>>,
    #[allow(dead_code)]
    update_thread: Option<thread::JoinHandle<()>>,
    is_closed: AtomicBool,
}

impl TdLibClient {
    /// Creates a new TDLib client and starts the update receiver loop.
    ///
    /// This allocates a new TDLib client ID and spawns a background thread
    /// that continuously calls `tdlib_rs::receive()` to process updates.
    pub fn new(config: TdLibConfig) -> Result<Self, TdLibError> {
        let client_id = tdlib_rs::create_client();

        tracing::info!(
            client_id,
            database_dir = %config.database_directory.display(),
            "Created TDLib client"
        );

        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| TdLibError::Init {
                    message: format!("failed to create tokio runtime: {e}"),
                })?,
        );

        let (auth_state_tx, auth_state_rx) = mpsc::channel::<AuthStateUpdate>();

        // Spawn update receiver thread (fully synchronous, no async runtime needed)
        let update_thread = {
            thread::spawn(move || {
                Self::run_update_loop(client_id, auth_state_tx);
            })
        };

        Ok(Self {
            client_id,
            config,
            rt,
            auth_state_rx: Mutex::new(auth_state_rx),
            update_thread: Some(update_thread),
            is_closed: AtomicBool::new(false),
        })
    }

    /// Background loop that receives and processes TDLib updates.
    ///
    /// This is a fully synchronous function that runs in a dedicated thread.
    /// It continuously polls `tdlib_rs::receive()` and dispatches auth state
    /// updates through the channel.
    fn run_update_loop(client_id: i32, auth_state_tx: mpsc::Sender<AuthStateUpdate>) {
        tracing::debug!(client_id, "Starting TDLib update loop");

        loop {
            match tdlib_rs::receive() {
                Some((update, received_client_id)) => {
                    if received_client_id != client_id {
                        continue;
                    }

                    // Handle authorization state updates; other updates will be
                    // handled in Phase 6 (Live Updates)
                    if let Update::AuthorizationState(state_update) = update {
                        let state = state_update.authorization_state.clone();
                        tracing::debug!(?state, "Authorization state changed");

                        let is_closed = matches!(state, AuthorizationState::Closed);

                        if auth_state_tx.send(AuthStateUpdate { state }).is_err() {
                            tracing::debug!("Auth state receiver dropped, stopping update loop");
                            break;
                        }

                        if is_closed {
                            tracing::info!(client_id, "TDLib client closed, stopping update loop");
                            break;
                        }
                    }
                }
                None => {
                    // No updates available, sleep before next poll
                    std::thread::sleep(UPDATE_POLL_INTERVAL);
                }
            }
        }

        tracing::debug!(client_id, "TDLib update loop finished");
    }

    /// Returns the TDLib client ID for sending requests.
    #[allow(dead_code)] // Will be used in extended API
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the configuration used to create this client.
    #[allow(dead_code)] // Will be used in extended API
    pub fn config(&self) -> &TdLibConfig {
        &self.config
    }

    /// Returns the async runtime for executing TDLib operations.
    #[allow(dead_code)] // Will be used in extended API
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.rt
    }

    /// Checks if the client has been closed.
    #[allow(dead_code)] // Will be used in lifecycle management
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

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
                .map_err(|e| TdLibError::Request { message: e.message })
        })
    }

    /// Checks the authentication code entered by the user.
    pub fn check_authentication_code(&self, code: &str) -> Result<(), TdLibError> {
        let code = code.to_owned();
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::check_authentication_code(code, client_id)
                .await
                .map_err(|e| TdLibError::Request { message: e.message })
        })
    }

    /// Checks the 2FA password.
    pub fn check_authentication_password(&self, password: &str) -> Result<(), TdLibError> {
        let password = password.to_owned();
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::check_authentication_password(password, client_id)
                .await
                .map_err(|e| TdLibError::Request { message: e.message })
        })
    }

    /// Gets list of chat IDs from TDLib.
    ///
    /// Returns up to `limit` chat IDs from the main chat list, sorted by TDLib's order.
    pub fn get_chats(&self, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            // First, load chats to ensure TDLib has them cached
            tdlib_rs::functions::load_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request { message: e.message })?;

            // Then get the chat IDs
            let chats = tdlib_rs::functions::get_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request { message: e.message })?;

            match chats {
                tdlib_rs::enums::Chats::Chats(c) => Ok(c.chat_ids),
            }
        })
    }

    /// Gets full chat information by ID.
    pub fn get_chat(&self, chat_id: i64) -> Result<tdlib_rs::types::Chat, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let chat = tdlib_rs::functions::get_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request { message: e.message })?;

            match chat {
                tdlib_rs::enums::Chat::Chat(c) => Ok(c),
            }
        })
    }

    /// Gets user information by ID.
    pub fn get_user(&self, user_id: i64) -> Result<tdlib_rs::types::User, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let user = tdlib_rs::functions::get_user(user_id, client_id)
                .await
                .map_err(|e| TdLibError::Request { message: e.message })?;

            match user {
                tdlib_rs::enums::User::User(u) => Ok(u),
            }
        })
    }

    /// Gets message history for a chat.
    ///
    /// Returns messages in reverse chronological order (newest first).
    /// Use `from_message_id: 0` to get the most recent messages.
    pub fn get_chat_history(
        &self,
        chat_id: i64,
        from_message_id: i64,
        offset: i32,
        limit: i32,
    ) -> Result<Vec<tdlib_rs::types::Message>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let messages = tdlib_rs::functions::get_chat_history(
                chat_id,
                from_message_id,
                offset,
                limit,
                false, // only_local: fetch from server if needed
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request { message: e.message })?;

            match messages {
                tdlib_rs::enums::Messages::Messages(m) => {
                    // Filter out None values (deleted messages)
                    Ok(m.messages.into_iter().flatten().collect())
                }
            }
        })
    }

    /// Sends a text message to a chat.
    ///
    /// Returns the sent message (which may have a temporary ID until confirmed).
    pub fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<tdlib_rs::types::Message, TdLibError> {
        let client_id = self.client_id;
        let text = text.to_owned();

        self.rt.block_on(async {
            let formatted_text = tdlib_rs::types::FormattedText {
                text,
                entities: vec![],
            };

            let input_content = tdlib_rs::enums::InputMessageContent::InputMessageText(
                tdlib_rs::types::InputMessageText {
                    text: formatted_text,
                    link_preview_options: None,
                    clear_draft: true,
                },
            );

            let message = tdlib_rs::functions::send_message(
                chat_id,
                None, // topic_id
                None, // reply_to
                None, // options
                input_content,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request { message: e.message })?;

            match message {
                tdlib_rs::enums::Message::Message(m) => Ok(m),
            }
        })
    }

    /// Graceful shutdown: sends `close()` and marks client as closed.
    ///
    /// After calling this method, the client should not be used for any
    /// further operations. TDLib will flush all data to disk and send
    /// `AuthorizationStateClosed` update.
    #[allow(dead_code)] // Will be used in shutdown handling
    pub fn close(&self) -> Result<(), TdLibError> {
        // Use compare_exchange to atomically check and set is_closed
        // This prevents race conditions when close() is called concurrently
        if self
            .is_closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            tracing::debug!(client_id = self.client_id, "Client already closed");
            return Ok(());
        }

        tracing::info!(client_id = self.client_id, "Closing TDLib client");

        let client_id = self.client_id;
        let result = self.rt.block_on(async {
            tdlib_rs::functions::close(client_id)
                .await
                .map_err(|e| TdLibError::Shutdown { message: e.message })
        });

        // Don't reset is_closed on failure - a failed close attempt leaves the
        // client in an undefined state. The client should be considered unusable
        // after any close attempt, successful or not.
        if result.is_err() {
            tracing::error!(
                client_id = self.client_id,
                "TDLib close failed - client is in undefined state"
            );
        } else {
            tracing::info!(client_id = self.client_id, "TDLib client closed");
        }

        result
    }
}

impl Drop for TdLibClient {
    fn drop(&mut self) {
        if !self.is_closed.load(Ordering::Acquire) {
            tracing::warn!(
                client_id = self.client_id,
                "TdLibClient dropped without calling close() - resources may not be properly released"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_stores_all_fields() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "test_hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
        };

        assert_eq!(config.api_id, 12345);
        assert_eq!(config.api_hash, "test_hash");
        assert_eq!(config.database_directory, PathBuf::from("/tmp/test_db"));
        assert_eq!(config.files_directory, PathBuf::from("/tmp/test_files"));
    }

    #[test]
    fn config_debug_redacts_api_hash() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "secret_hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
        };

        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("secret_hash"));
    }
}
