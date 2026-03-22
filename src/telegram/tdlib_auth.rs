//! TDLib authentication backend.
//!
//! Implements the `TelegramAuthClient` trait using TDLib for authentication.
//! Handles the TDLib authorization state machine:
//! - WaitTdlibParameters → set_tdlib_parameters
//! - WaitPhoneNumber → set_authentication_phone_number
//! - WaitCode → check_authentication_code
//! - WaitPassword → check_authentication_password
//! - Ready → authorization complete

use std::time::Duration;

use tdlib_rs::enums::AuthorizationState;

use crate::domain::chat::ChatSummary;
use crate::domain::message::Message;
use crate::domain::status::AuthConnectivityStatus;
use crate::infra::config::TelegramConfig;
use crate::infra::storage_layout::StorageLayout;
use crate::usecases::guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome};
use crate::usecases::list_chats::ListChatsSourceError;
use crate::usecases::load_messages::MessagesSourceError;
use crate::usecases::send_message::SendMessageSourceError;

use super::tdlib_client::{TdLibClient, TdLibConfig, TdLibError};
use super::tdlib_mappers;

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
            log_file: layout.tdlib_log_file(),
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
    ///
    /// Returns `Ok(true)` if TDLib reports `AuthorizationState::Ready`,
    /// `Ok(false)` if the cached state is not `Ready` or if a short poll
    /// times out (no state available yet). Propagates non-timeout errors
    /// so the caller can distinguish "not yet authorized" from "broken client".
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
            Err(TdLibError::Timeout { .. }) => Ok(false),
            Err(other) => Err(map_tdlib_error(other)),
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

    /// Returns the current authentication status snapshot.
    #[allow(dead_code)]
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
    ///
    /// This can only be called once - subsequent calls return None.
    pub fn take_update_receiver(
        &self,
    ) -> Option<std::sync::mpsc::Receiver<super::tdlib_updates::TdLibUpdate>> {
        self.client.take_update_receiver()
    }

    /// Lists chat summaries from TDLib's local cache only.
    ///
    /// Does **not** call `loadChats`, so no network request is made.
    /// Returns whatever chats are already present in TDLib's SQLite database
    /// from previous sessions. Useful for instant startup display.
    pub fn list_cached_chat_summaries(
        &self,
        limit: usize,
    ) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let chat_ids = self
            .client
            .get_cached_chats(limit_i32)
            .map_err(map_list_chats_error)?;

        tracing::debug!(count = chat_ids.len(), "Fetched cached chat IDs from TDLib");

        Ok(self.build_summaries_from_ids(chat_ids))
    }

    /// Lists chat summaries from TDLib.
    ///
    /// Fetches chats from the main chat list and maps them to domain `ChatSummary`.
    pub fn list_chat_summaries(
        &self,
        limit: usize,
    ) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let chat_ids = self
            .client
            .get_chats(limit_i32)
            .map_err(map_list_chats_error)?;

        tracing::debug!(count = chat_ids.len(), "Fetched chat IDs from TDLib");

        Ok(self.build_summaries_from_ids(chat_ids))
    }

    /// Builds domain `ChatSummary` list from raw TDLib chat IDs.
    ///
    /// Fetches full chat info for each ID and maps to domain types.
    /// Skips individual chats that fail to load.
    fn build_summaries_from_ids(&self, chat_ids: Vec<i64>) -> Vec<ChatSummary> {
        let mut summaries = Vec::with_capacity(chat_ids.len());

        for chat_id in chat_ids {
            match self.client.get_chat(chat_id) {
                Ok(chat) => {
                    let (sender_name, is_online, is_bot) = self.resolve_chat_metadata(&chat);
                    let summary =
                        tdlib_mappers::map_chat_to_summary(&chat, sender_name, is_online, is_bot);
                    summaries.push(summary);
                }
                Err(e) => {
                    tracing::warn!(chat_id, error = %e, "Failed to fetch chat details, skipping");
                }
            }
        }

        summaries
    }

    /// Resolves additional metadata for a chat (sender name, online status).
    fn resolve_chat_metadata(
        &self,
        chat: &tdlib_rs::types::Chat,
    ) -> (Option<String>, Option<bool>, bool) {
        let chat_type = tdlib_mappers::map_chat_type(&chat.r#type);

        let (is_online, is_bot) = if matches!(chat_type, crate::domain::chat::ChatType::Private) {
            if let Some(user_id) = tdlib_mappers::get_private_chat_user_id(&chat.r#type) {
                match self.client.get_user(user_id).ok() {
                    Some(u) => (
                        Some(tdlib_mappers::is_user_online(&u.status)),
                        matches!(u.r#type, tdlib_rs::enums::UserType::Bot(_)),
                    ),
                    None => (None, false),
                }
            } else {
                (None, false)
            }
        } else {
            (None, false)
        };

        // For group chats, get the sender name of the last message
        let sender_name = if matches!(
            chat_type,
            crate::domain::chat::ChatType::Group | crate::domain::chat::ChatType::Channel
        ) {
            chat.last_message.as_ref().and_then(|msg| {
                if let Some(user_id) = tdlib_mappers::get_sender_user_id(&msg.sender_id) {
                    self.client
                        .get_user(user_id)
                        .ok()
                        .map(|u| tdlib_mappers::format_user_name(&u))
                } else {
                    None
                }
            })
        } else {
            None
        };

        (sender_name, is_online, is_bot)
    }

    /// Lists messages from TDLib's local cache only.
    ///
    /// Does **not** call `openChat`/`closeChat` or trigger any network requests.
    /// Returns whatever messages TDLib has cached locally for this chat.
    /// Used for instant chat display before a full background refresh.
    ///
    /// Returns messages in chronological order (oldest first).
    pub fn list_cached_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        let limit_i32 = i32::try_from(limit).unwrap_or(i32::MAX);

        let td_messages = self
            .client
            .get_cached_chat_history(chat_id, 0, 0, limit_i32)
            .map_err(map_messages_error)?;

        tracing::debug!(
            chat_id,
            count = td_messages.len(),
            "fetched cached messages from TDLib"
        );

        let mut messages: Vec<Message> = td_messages
            .iter()
            .map(|msg| {
                let sender_name = self.resolve_message_sender_name(msg);
                tdlib_mappers::map_tdlib_message_to_domain(msg, sender_name)
            })
            .collect();

        // TDLib returns newest-first; UI expects chronological (oldest-first)
        messages.reverse();

        Ok(messages)
    }

    /// Lists messages from a chat.
    ///
    /// Returns messages in chronological order (oldest first).
    ///
    /// **Note:** The caller is responsible for the TDLib `openChat`/`closeChat`
    /// lifecycle. This method only fetches messages via paginated
    /// `getChatHistory` calls.
    pub fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        self.fetch_messages_paginated(chat_id, limit)
    }

    /// Informs TDLib that the user has opened a chat.
    ///
    /// Must be paired with [`close_chat`](Self::close_chat). While a chat
    /// is open, TDLib delivers all updates for it (important for supergroups
    /// and channels) and `viewMessages` with `force_read: false` can mark
    /// messages as read.
    pub fn open_chat(&self, chat_id: i64) -> Result<(), MessagesSourceError> {
        self.client.open_chat(chat_id).map_err(map_messages_error)
    }

    /// Informs TDLib that the user has closed a chat.
    ///
    /// Must be called for every chat previously opened via
    /// [`open_chat`](Self::open_chat).
    pub fn close_chat(&self, chat_id: i64) -> Result<(), MessagesSourceError> {
        self.client.close_chat(chat_id).map_err(map_messages_error)
    }

    /// Marks the given messages as viewed/read in a chat.
    ///
    /// The chat must be opened via [`open_chat`](Self::open_chat) first.
    /// TDLib will send `Update::ChatReadInbox` when the read state changes,
    /// which triggers a reactive chat list refresh with updated `unread_count`.
    pub fn view_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
    ) -> Result<(), MessagesSourceError> {
        self.client
            .view_messages(chat_id, message_ids)
            .map_err(map_messages_error)
    }

    pub fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), MessagesSourceError> {
        self.client
            .delete_messages(chat_id, message_ids, revoke)
            .map_err(map_messages_error)
    }

    /// Fetches up to `limit` messages using paginated `getChatHistory` calls.
    ///
    /// TDLib documentation states that the number of returned messages is
    /// chosen by TDLib and can be smaller than the specified limit. This
    /// method accumulates results across multiple calls until the requested
    /// amount is reached or TDLib returns no more messages.
    fn fetch_messages_paginated(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        use super::message_pagination::{fetch_paginated, PageResult};

        let td_messages = fetch_paginated(
            limit,
            |from_message_id, page_limit| {
                let batch = self
                    .client
                    .get_chat_history(chat_id, from_message_id, 0, page_limit)
                    .map_err(map_messages_error)?;

                tracing::debug!(
                    chat_id,
                    batch_len = batch.len(),
                    "getChatHistory page fetched"
                );

                Ok(PageResult { messages: batch })
            },
            |msg| msg.id,
        )?;

        tracing::debug!(
            chat_id,
            total = td_messages.len(),
            "message pagination complete"
        );

        // Convert to domain messages (accumulated is newest-first)
        let mut messages: Vec<Message> = td_messages
            .iter()
            .map(|msg| {
                let sender_name = self.resolve_message_sender_name(msg);
                tdlib_mappers::map_tdlib_message_to_domain(msg, sender_name)
            })
            .collect();

        // Reverse to get oldest first (UI expects chronological order)
        messages.reverse();

        Ok(messages)
    }

    /// Sends a text message to a chat.
    pub fn send_message(&self, chat_id: i64, text: &str) -> Result<(), SendMessageSourceError> {
        self.client
            .send_message(chat_id, text)
            .map_err(map_send_message_error)?;

        tracing::debug!(chat_id, text_len = text.len(), "Message sent via TDLib");
        Ok(())
    }

    /// Resolves the sender name for a message.
    fn resolve_message_sender_name(&self, msg: &tdlib_rs::types::Message) -> String {
        resolve_sender_name(&self.client, msg)
    }

    /// Creates a `MessageMapper` that can be shared with the chat updates monitor.
    ///
    /// The mapper holds clones of the async runtime and client_id, allowing it
    /// to resolve sender names via `get_user`/`get_chat` on the monitor thread.
    pub fn create_message_mapper(&self) -> std::sync::Arc<dyn super::chat_updates::MessageMapper> {
        std::sync::Arc::new(TdLibMessageMapper {
            rt: self.client.runtime().clone(),
            client_id: self.client.client_id(),
        })
    }
}

/// Resolves the sender name for a TDLib message using the TDLib client.
fn resolve_sender_name(client: &TdLibClient, msg: &tdlib_rs::types::Message) -> String {
    match &msg.sender_id {
        tdlib_rs::enums::MessageSender::User(u) => client
            .get_user(u.user_id)
            .map(|user| tdlib_mappers::format_user_name(&user))
            .unwrap_or_else(|_| "Unknown".to_owned()),
        tdlib_rs::enums::MessageSender::Chat(c) => client
            .get_chat(c.chat_id)
            .map(|chat| chat.title.clone())
            .unwrap_or_else(|_| "Channel".to_owned()),
    }
}

/// Maps raw TDLib messages to domain `Message` types.
///
/// Holds the TDLib async runtime and client ID so it can resolve sender names
/// via `get_user`/`get_chat` on the monitor thread.
struct TdLibMessageMapper {
    rt: std::sync::Arc<tokio::runtime::Runtime>,
    client_id: i32,
}

impl super::chat_updates::MessageMapper for TdLibMessageMapper {
    fn map_message(&self, raw: &tdlib_rs::types::Message) -> Message {
        let sender_name = match &raw.sender_id {
            tdlib_rs::enums::MessageSender::User(u) => self
                .rt
                .block_on(async { tdlib_rs::functions::get_user(u.user_id, self.client_id).await })
                .map(|user| match user {
                    tdlib_rs::enums::User::User(u) => tdlib_mappers::format_user_name(&u),
                })
                .unwrap_or_else(|_| "Unknown".to_owned()),
            tdlib_rs::enums::MessageSender::Chat(c) => self
                .rt
                .block_on(async { tdlib_rs::functions::get_chat(c.chat_id, self.client_id).await })
                .map(|chat| match chat {
                    tdlib_rs::enums::Chat::Chat(c) => c.title,
                })
                .unwrap_or_else(|_| "Channel".to_owned()),
        };
        tdlib_mappers::map_tdlib_message_to_domain(raw, sender_name)
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
        TdLibError::Init { message } | TdLibError::Request { message, .. } => {
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
    let TdLibError::Request { message, .. } = error else {
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
    let TdLibError::Request { message, .. } = error else {
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
    let TdLibError::Request { message, .. } = error else {
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

/// Maps TDLib error to ListChatsSourceError.
fn map_list_chats_error(error: TdLibError) -> ListChatsSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    // Check for unauthorized errors
    if msg.contains("unauthorized") || msg.contains("auth") {
        return ListChatsSourceError::Unauthorized;
    }

    ListChatsSourceError::Unavailable
}

/// Maps TDLib error to MessagesSourceError.
fn map_messages_error(error: TdLibError) -> MessagesSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    if msg.contains("unauthorized") || msg.contains("auth") {
        return MessagesSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return MessagesSourceError::ChatNotFound;
    }

    MessagesSourceError::Unavailable
}

/// Maps TDLib error to SendMessageSourceError.
fn map_send_message_error(error: TdLibError) -> SendMessageSourceError {
    let msg = match &error {
        TdLibError::Request { message, .. } => message.to_ascii_lowercase(),
        _ => String::new(),
    };

    if msg.contains("unauthorized") || msg.contains("auth") {
        return SendMessageSourceError::Unauthorized;
    }

    if msg.contains("chat") && msg.contains("not found") {
        return SendMessageSourceError::ChatNotFound;
    }

    SendMessageSourceError::Unavailable
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
            code: 400,
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
            code: 400,
            message: "PHONE_CODE_INVALID".to_owned(),
        };
        assert_eq!(map_sign_in_error(error), AuthBackendError::InvalidCode);
    }

    #[test]
    fn map_password_error_detects_wrong_password() {
        let error = TdLibError::Request {
            code: 400,
            message: "PASSWORD_HASH_INVALID".to_owned(),
        };
        assert_eq!(map_password_error(error), AuthBackendError::WrongPassword);
    }

    #[test]
    fn map_flood_wait_in_request_code() {
        let error = TdLibError::Request {
            code: 429,
            message: "FLOOD_WAIT_300".to_owned(),
        };
        assert_eq!(
            map_request_code_error(error),
            AuthBackendError::FloodWait { seconds: 300 }
        );
    }

    #[test]
    fn map_list_chats_error_returns_unavailable_for_generic_error() {
        let error = TdLibError::Request {
            code: 500,
            message: "Internal Server Error".to_owned(),
        };
        assert_eq!(
            map_list_chats_error(error),
            ListChatsSourceError::Unavailable,
        );
    }

    #[test]
    fn map_list_chats_error_returns_unauthorized_for_auth_error() {
        let error = TdLibError::Request {
            code: 401,
            message: "Unauthorized".to_owned(),
        };
        assert_eq!(
            map_list_chats_error(error),
            ListChatsSourceError::Unauthorized,
        );
    }

    #[test]
    fn map_tdlib_error_maps_request_to_transient() {
        let error = TdLibError::Request {
            code: 400,
            message: "BAD_REQUEST".to_owned(),
        };
        assert_eq!(
            map_tdlib_error(error),
            AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: "BAD_REQUEST".to_owned(),
            }
        );
    }
}
