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

use super::tdlib_updates::TdLibUpdate;

/// Polling interval when no TDLib updates are available.
/// 10ms provides responsive update handling without excessive CPU usage.
const UPDATE_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Maximum size of TDLib's internal log file before automatic rotation.
/// TDLib creates a `.old` backup when this limit is reached.
const TDLIB_LOG_MAX_SIZE: i64 = 10 * 1024 * 1024; // 10 MB

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
    /// Path for TDLib's internal log file.
    ///
    /// TDLib (C++ library) has its own logger that writes to stderr by default,
    /// which corrupts the TUI alternate screen. This redirects it to a file.
    pub log_file: PathBuf,
}

// Custom Debug implementation to redact sensitive api_hash field.
impl std::fmt::Debug for TdLibConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdLibConfig")
            .field("api_id", &self.api_id)
            .field("api_hash", &"[REDACTED]")
            .field("database_directory", &self.database_directory)
            .field("files_directory", &self.files_directory)
            .field("log_file", &self.log_file)
            .finish()
    }
}

/// TDLib error code returned by `loadChats` when all chats are already cached.
///
/// This is a normal "nothing more to load" signal, not a real failure.
/// See TDLib docs: "Returns a 404 error if all chats have been loaded."
const TDLIB_ERROR_ALL_CHATS_LOADED: i32 = 404;

/// Error types for TDLib operations.
#[derive(Debug, thiserror::Error)]
pub enum TdLibError {
    /// TDLib initialization error
    #[error("TDLib initialization error: {message}")]
    Init { message: String },

    /// TDLib request error (carries TDLib error code for discrimination)
    #[error("TDLib request error {code}: {message}")]
    Request { code: i32, message: String },

    /// TDLib shutdown error
    #[error("TDLib shutdown error: {message}")]
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
/// - Typed update events channel
/// - Proper shutdown via `close()` function
pub struct TdLibClient {
    client_id: i32,
    config: TdLibConfig,
    rt: Arc<Runtime>,
    auth_state_rx: Mutex<mpsc::Receiver<AuthStateUpdate>>,
    /// Receiver for typed TDLib updates. Wrapped in Option to allow taking.
    update_rx: Mutex<Option<mpsc::Receiver<TdLibUpdate>>>,
    /// Update loop thread handle. Kept alive for the client's lifetime.
    _update_thread: Option<thread::JoinHandle<()>>,
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
        let (update_tx, update_rx) = mpsc::channel::<TdLibUpdate>();

        // Spawn update receiver thread (fully synchronous, no async runtime needed)
        let update_thread = {
            thread::spawn(move || {
                Self::run_update_loop(client_id, auth_state_tx, update_tx);
            })
        };

        // Redirect TDLib's internal C++ logger from stderr to a file FIRST,
        // before any other request. TDLib (C++ library) writes logs to stderr
        // by default, which corrupts the TUI alternate screen (ratatui).
        // This also serves as the mandatory initial request that activates
        // TDLib's update delivery — without at least one request, `receive()`
        // will never return the initial `WaitTdlibParameters` state update.
        // See: tdlib-rs examples/get_me.rs and lib.rs documentation.
        let log_path = config
            .log_file
            .to_str()
            .ok_or_else(|| TdLibError::Init {
                message: "TDLib log file path is not valid UTF-8".to_owned(),
            })?
            .to_owned();
        rt.block_on(async {
            tdlib_rs::functions::set_log_stream(
                tdlib_rs::enums::LogStream::File(tdlib_rs::types::LogStreamFile {
                    path: log_path,
                    max_file_size: TDLIB_LOG_MAX_SIZE,
                    redirect_stderr: false,
                }),
                client_id,
            )
            .await
        })
        .map_err(|e| TdLibError::Init {
            message: format!("failed to redirect TDLib log stream to file: {}", e.message),
        })?;

        // Set TDLib internal log verbosity: 2 = warnings and errors only.
        rt.block_on(async { tdlib_rs::functions::set_log_verbosity_level(2, client_id).await })
            .map_err(|e| TdLibError::Init {
                message: format!("failed to set TDLib log verbosity: {}", e.message),
            })?;

        tracing::debug!(
            client_id,
            log_file = %config.log_file.display(),
            "TDLib client initialized, logs redirected to file"
        );

        Ok(Self {
            client_id,
            config,
            rt,
            auth_state_rx: Mutex::new(auth_state_rx),
            update_rx: Mutex::new(Some(update_rx)),
            _update_thread: Some(update_thread),
            is_closed: AtomicBool::new(false),
        })
    }

    /// Background loop that receives and processes TDLib updates.
    ///
    /// This is a fully synchronous function that runs in a dedicated thread.
    /// It continuously polls `tdlib_rs::receive()` and dispatches updates
    /// through the appropriate channels.
    fn run_update_loop(
        client_id: i32,
        auth_state_tx: mpsc::Sender<AuthStateUpdate>,
        update_tx: mpsc::Sender<TdLibUpdate>,
    ) {
        tracing::debug!(client_id, "Starting TDLib update loop");

        loop {
            match tdlib_rs::receive() {
                Some((update, received_client_id)) => {
                    if received_client_id != client_id {
                        continue;
                    }

                    match update {
                        // Authorization state updates
                        Update::AuthorizationState(state_update) => {
                            let state = state_update.authorization_state.clone();
                            tracing::debug!(?state, "Authorization state changed");

                            let is_closed = matches!(state, AuthorizationState::Closed);

                            if auth_state_tx.send(AuthStateUpdate { state }).is_err() {
                                tracing::debug!(
                                    "Auth state receiver dropped, stopping update loop"
                                );
                                break;
                            }

                            if is_closed {
                                tracing::info!(
                                    client_id,
                                    "TDLib client closed, stopping update loop"
                                );
                                break;
                            }
                        }

                        // Message updates
                        Update::NewMessage(u) => {
                            let _ = update_tx.send(TdLibUpdate::NewMessage {
                                chat_id: u.message.chat_id,
                            });
                        }
                        Update::MessageContent(u) => {
                            let _ = update_tx.send(TdLibUpdate::MessageContent {
                                chat_id: u.chat_id,
                                message_id: u.message_id,
                            });
                        }
                        Update::DeleteMessages(u) => {
                            let _ = update_tx.send(TdLibUpdate::DeleteMessages {
                                chat_id: u.chat_id,
                                message_ids: u.message_ids,
                            });
                        }
                        Update::MessageSendSucceeded(u) => {
                            let _ = update_tx.send(TdLibUpdate::MessageSendSucceeded {
                                chat_id: u.message.chat_id,
                                old_message_id: u.old_message_id,
                            });
                        }

                        // Chat list updates
                        Update::ChatLastMessage(u) => {
                            let _ =
                                update_tx.send(TdLibUpdate::ChatLastMessage { chat_id: u.chat_id });
                        }
                        Update::ChatPosition(u) => {
                            let _ =
                                update_tx.send(TdLibUpdate::ChatPosition { chat_id: u.chat_id });
                        }

                        // Read status updates
                        Update::ChatReadInbox(u) => {
                            let _ =
                                update_tx.send(TdLibUpdate::ChatReadInbox { chat_id: u.chat_id });
                        }
                        Update::ChatReadOutbox(u) => {
                            let _ =
                                update_tx.send(TdLibUpdate::ChatReadOutbox { chat_id: u.chat_id });
                        }

                        // User status updates
                        Update::UserStatus(u) => {
                            let _ = update_tx.send(TdLibUpdate::UserStatus { user_id: u.user_id });
                        }

                        // Ignore other update types
                        _ => {
                            tracing::trace!("Unhandled TDLib update type");
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

    /// Takes the typed update receiver.
    ///
    /// This can only be called once - subsequent calls return None.
    /// Used by TelegramChatUpdatesMonitor to receive typed updates.
    pub fn take_update_receiver(&self) -> Option<mpsc::Receiver<TdLibUpdate>> {
        self.update_rx.lock().ok()?.take()
    }

    /// Returns the TDLib client ID for sending requests.
    #[allow(dead_code)]
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    /// Returns the configuration used to create this client.
    #[allow(dead_code)]
    pub fn config(&self) -> &TdLibConfig {
        &self.config
    }

    /// Returns the async runtime for executing TDLib operations.
    #[allow(dead_code)]
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.rt
    }

    /// Checks if the client has been closed.
    #[allow(dead_code)]
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

    /// Gets chat IDs already present in TDLib's local database.
    ///
    /// Unlike [`get_chats`](Self::get_chats), this does **not** call `loadChats`
    /// first, so it never triggers a network request. Returns whatever TDLib
    /// has cached locally from previous sessions (SQLite database).
    ///
    /// Useful for instant startup: show cached chats immediately, then
    /// refresh from the server in the background.
    pub fn get_cached_chats(&self, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            let chats = tdlib_rs::functions::get_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

            match chats {
                tdlib_rs::enums::Chats::Chats(c) => Ok(c.chat_ids),
            }
        })
    }

    /// Gets list of chat IDs from TDLib.
    ///
    /// Returns up to `limit` chat IDs from the main chat list, sorted by TDLib's order.
    pub fn get_chats(&self, limit: i32) -> Result<Vec<i64>, TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            // First, load chats to ensure TDLib has them cached.
            // TDLib returns error 404 when all chats have already been loaded —
            // this is a normal "nothing more to load" signal, not a real failure.
            // We must continue to `get_chats` regardless, because already-cached
            // chats are still available for retrieval.
            if let Err(e) = tdlib_rs::functions::load_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            {
                if e.code == TDLIB_ERROR_ALL_CHATS_LOADED {
                    tracing::debug!("load_chats returned 404: all chats already loaded");
                } else {
                    tracing::warn!(
                        code = e.code,
                        message = %e.message,
                        "load_chats failed with unexpected error"
                    );
                    return Err(TdLibError::Request {
                        code: e.code,
                        message: e.message,
                    });
                }
            }

            // Then get the chat IDs
            let chats = tdlib_rs::functions::get_chats(
                Some(tdlib_rs::enums::ChatList::Main),
                limit,
                client_id,
            )
            .await
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

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
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

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
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })?;

            match user {
                tdlib_rs::enums::User::User(u) => Ok(u),
            }
        })
    }

    /// Informs TDLib that the chat is opened by the user.
    ///
    /// Many useful activities depend on the chat being opened or closed
    /// (e.g., in supergroups and channels all updates are received only
    /// for opened chats). Must be paired with [`close_chat`](Self::close_chat).
    pub fn open_chat(&self, chat_id: i64) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::open_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
        })
    }

    /// Informs TDLib that the chat is closed by the user.
    ///
    /// Must be called for every chat previously opened via
    /// [`open_chat`](Self::open_chat).
    pub fn close_chat(&self, chat_id: i64) -> Result<(), TdLibError> {
        let client_id = self.client_id;

        self.rt.block_on(async {
            tdlib_rs::functions::close_chat(chat_id, client_id)
                .await
                .map_err(|e| TdLibError::Request {
                    code: e.code,
                    message: e.message,
                })
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
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

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
            .map_err(|e| TdLibError::Request {
                code: e.code,
                message: e.message,
            })?;

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
    #[allow(dead_code)]
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
            log_file: PathBuf::from("/tmp/test_logs/tdlib.log"),
        };

        assert_eq!(config.api_id, 12345);
        assert_eq!(config.api_hash, "test_hash");
        assert_eq!(config.database_directory, PathBuf::from("/tmp/test_db"));
        assert_eq!(config.files_directory, PathBuf::from("/tmp/test_files"));
        assert_eq!(config.log_file, PathBuf::from("/tmp/test_logs/tdlib.log"));
    }

    #[test]
    fn tdlib_error_all_chats_loaded_code_is_404() {
        assert_eq!(TDLIB_ERROR_ALL_CHATS_LOADED, 404);
    }

    #[test]
    fn request_error_displays_code_and_message() {
        let error = TdLibError::Request {
            code: 404,
            message: "Chat list loading completed".to_owned(),
        };
        let display = format!("{error}");
        assert!(display.contains("404"));
        assert!(display.contains("Chat list loading completed"));
    }

    #[test]
    fn config_debug_redacts_api_hash() {
        let config = TdLibConfig {
            api_id: 12345,
            api_hash: "secret_hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
            log_file: PathBuf::from("/tmp/test_logs/tdlib.log"),
        };

        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("secret_hash"));
    }
}
