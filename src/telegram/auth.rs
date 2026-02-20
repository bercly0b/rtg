use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::Session;
use tokio::runtime::Builder;

use crate::{
    domain::{
        chat::ChatSummary,
        message::{Message, MessageMedia},
    },
    infra::config::TelegramConfig,
    usecases::{
        guided_auth::{AuthBackendError, AuthCodeToken, SignInOutcome},
        list_chats::ListChatsSourceError,
        load_messages::MessagesSourceError,
    },
};

use super::chat_updates::{ChatUpdatesMonitorStartError, TelegramChatUpdatesMonitor};

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
    rt: Arc<tokio::runtime::Runtime>,
    client: Client,
    login_token: Option<grammers_client::types::LoginToken>,
    password_token: Option<grammers_client::types::PasswordToken>,
    current_code_token: Option<AuthCodeToken>,
    next_code_token_id: u64,
    state: LoginState,
    cached_folder_scope: RwLock<Option<DialogFetchScope>>,
    chat_cache: RwLock<std::collections::HashMap<i64, grammers_client::types::PackedChat>>,
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

        let rt = Arc::new(
            build_auth_runtime().map_err(|error| AuthBackendError::Transient {
                code: "AUTH_BACKEND_UNAVAILABLE",
                message: format!("failed to initialize async runtime: {error}"),
            })?,
        );

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
            cached_folder_scope: RwLock::new(None),
            chat_cache: RwLock::new(std::collections::HashMap::new()),
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
        verify_password_with_token(
            &mut self.password_token,
            &mut self.state,
            password,
            |password_token, candidate_password| {
                self.rt
                    .block_on(
                        self.client
                            .check_password(password_token, candidate_password),
                    )
                    .map(|_| ())
                    .map_err(map_password_error)
            },
        )
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

    pub(super) fn list_chat_summaries(
        &self,
        limit: usize,
    ) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
        self.rt.block_on(async {
            let fetch_scope = {
                let cached_scope = {
                    let cached = self.cached_folder_scope.read().unwrap();
                    *cached
                };

                if let Some(scope) = cached_scope {
                    scope
                } else {
                    let scope = determine_dialog_fetch_scope(&self.client)
                        .await
                        .unwrap_or(DialogFetchScope::AllDialogs);
                    let mut cache = self.cached_folder_scope.write().unwrap();
                    *cache = Some(scope);
                    scope
                }
            };

            match fetch_scope {
                DialogFetchScope::MainFolderOnly => {
                    fetch_chat_summaries_from_main_folder(&self.client, limit, &self.chat_cache)
                        .await
                }
                DialogFetchScope::AllDialogs => {
                    fetch_chat_summaries_from_all_dialogs(&self.client, limit, &self.chat_cache)
                        .await
                }
            }
        })
    }

    pub(super) fn list_messages(
        &self,
        chat_id: i64,
        limit: usize,
    ) -> Result<Vec<Message>, MessagesSourceError> {
        self.rt.block_on(async {
            fetch_messages_from_chat(&self.client, chat_id, limit, &self.chat_cache).await
        })
    }

    pub(super) fn disconnect_and_reset(&mut self) {
        self.login_token = None;
        self.password_token = None;
        self.current_code_token = None;
        self.state = LoginState::Disconnected;
        *self.cached_folder_scope.write().unwrap() = None;
        self.chat_cache.write().unwrap().clear();
    }

    pub(super) fn start_chat_updates_monitor(
        &self,
        updates_tx: std::sync::mpsc::Sender<()>,
    ) -> Result<TelegramChatUpdatesMonitor, ChatUpdatesMonitorStartError> {
        TelegramChatUpdatesMonitor::start(&self.rt, self.client.clone(), updates_tx)
    }

    #[allow(dead_code)]
    pub(super) fn state(&self) -> LoginState {
        self.state
    }
}

fn build_auth_runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogFetchScope {
    AllDialogs,
    MainFolderOnly,
}

async fn determine_dialog_fetch_scope(
    client: &grammers_client::Client,
) -> Result<DialogFetchScope, ListChatsSourceError> {
    let filters_response = client
        .invoke(&grammers_client::grammers_tl_types::functions::messages::GetDialogFilters {})
        .await
        .map_err(map_list_chats_invocation_error)?;

    let filters = match filters_response {
        grammers_client::grammers_tl_types::enums::messages::DialogFilters::Filters(data) => {
            data.filters
        }
    };

    Ok(dialog_fetch_scope_from_filters(&filters))
}

fn dialog_fetch_scope_from_filters(
    filters: &[grammers_client::grammers_tl_types::enums::DialogFilter],
) -> DialogFetchScope {
    let has_custom_folder = filters.iter().any(|filter| {
        !matches!(
            filter,
            grammers_client::grammers_tl_types::enums::DialogFilter::Default
        )
    });

    if has_custom_folder {
        DialogFetchScope::MainFolderOnly
    } else {
        DialogFetchScope::AllDialogs
    }
}

async fn fetch_chat_summaries_from_all_dialogs(
    client: &grammers_client::Client,
    limit: usize,
    chat_cache: &RwLock<std::collections::HashMap<i64, grammers_client::types::PackedChat>>,
) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
    let mut dialogs = client.iter_dialogs().limit(limit);
    let mut chats = Vec::with_capacity(limit);
    let mut cache_entries = Vec::new();

    while let Some(dialog) = dialogs
        .next()
        .await
        .map_err(map_list_chats_invocation_error)?
    {
        let chat = dialog.chat();
        let chat_id = chat.id();
        let packed: grammers_client::types::PackedChat = chat.into();
        cache_entries.push((chat_id, packed));

        let unread_count = dialog_unread_count(&dialog.raw)?;
        let is_pinned = dialog_is_pinned(&dialog.raw);
        let last_message_preview = dialog
            .last_message
            .as_ref()
            .and_then(|message| normalize_preview_text(message.text()));
        let last_message_unix_ms = dialog
            .last_message
            .as_ref()
            .map(|message| message.date().timestamp_millis());

        chats.push(ChatSummary {
            chat_id,
            title: chat.name().to_owned(),
            unread_count,
            last_message_preview,
            last_message_unix_ms,
            is_pinned,
        });
    }

    let mut cache = chat_cache.write().unwrap();
    for (chat_id, packed) in cache_entries {
        cache.insert(chat_id, packed);
    }

    Ok(chats)
}

async fn fetch_chat_summaries_from_main_folder(
    client: &grammers_client::Client,
    limit: usize,
    chat_cache: &RwLock<std::collections::HashMap<i64, grammers_client::types::PackedChat>>,
) -> Result<Vec<ChatSummary>, ListChatsSourceError> {
    let response = client
        .invoke(
            &grammers_client::grammers_tl_types::functions::messages::GetDialogs {
                exclude_pinned: false,
                folder_id: Some(0),
                offset_date: 0,
                offset_id: 0,
                offset_peer: grammers_client::grammers_tl_types::enums::InputPeer::Empty,
                limit: i32::try_from(limit.min(100))
                    .map_err(|_| ListChatsSourceError::InvalidData)?,
                hash: 0,
            },
        )
        .await
        .map_err(map_list_chats_invocation_error)?;

    let (dialogs, messages, users, chats) = match response {
        grammers_client::grammers_tl_types::enums::messages::Dialogs::Dialogs(data) => {
            (data.dialogs, data.messages, data.users, data.chats)
        }
        grammers_client::grammers_tl_types::enums::messages::Dialogs::Slice(data) => {
            (data.dialogs, data.messages, data.users, data.chats)
        }
        grammers_client::grammers_tl_types::enums::messages::Dialogs::NotModified(_) => {
            return Ok(Vec::new())
        }
    };

    let chat_map = grammers_client::types::ChatMap::new(users, chats);
    let mut message_map = std::collections::HashMap::<
        (i64, i32),
        grammers_client::grammers_tl_types::enums::Message,
    >::new();

    for message in messages {
        if let Some((peer_key, message_id)) = dialog_message_key(&message) {
            message_map.insert((peer_key, message_id), message);
        }
    }

    let mut result = Vec::new();
    let mut cache = chat_cache.write().unwrap();

    for dialog in dialogs {
        let grammers_client::grammers_tl_types::enums::Dialog::Dialog(data) = dialog else {
            continue;
        };

        let Some(chat) = chat_map.get(&data.peer) else {
            continue;
        };

        let chat_id = chat.id();
        let packed: grammers_client::types::PackedChat = chat.into();
        cache.insert(chat_id, packed);

        let unread_count =
            u32::try_from(data.unread_count).map_err(|_| ListChatsSourceError::InvalidData)?;
        let is_pinned = data.pinned;
        let message_key = (dialog_peer_key(&data.peer), data.top_message);
        let last_message = message_map.get(&message_key).and_then(|message| {
            grammers_client::types::Message::from_raw(client, message.clone(), &chat_map)
        });
        let last_message_preview = last_message
            .as_ref()
            .and_then(|message| normalize_preview_text(message.text()));
        let last_message_unix_ms = last_message
            .as_ref()
            .map(|message| message.date().timestamp_millis());

        result.push(ChatSummary {
            chat_id: chat.id(),
            title: chat.name().to_owned(),
            unread_count,
            last_message_preview,
            last_message_unix_ms,
            is_pinned,
        });

        if result.len() >= limit {
            break;
        }
    }

    Ok(result)
}

fn dialog_peer_key(peer: &grammers_client::grammers_tl_types::enums::Peer) -> i64 {
    match peer {
        grammers_client::grammers_tl_types::enums::Peer::User(data) => data.user_id,
        grammers_client::grammers_tl_types::enums::Peer::Chat(data) => -data.chat_id,
        grammers_client::grammers_tl_types::enums::Peer::Channel(data) => -data.channel_id,
    }
}

fn dialog_message_key(
    message: &grammers_client::grammers_tl_types::enums::Message,
) -> Option<(i64, i32)> {
    match message {
        grammers_client::grammers_tl_types::enums::Message::Message(data) => {
            Some((dialog_peer_key(&data.peer_id), data.id))
        }
        grammers_client::grammers_tl_types::enums::Message::Service(data) => {
            Some((dialog_peer_key(&data.peer_id), data.id))
        }
        grammers_client::grammers_tl_types::enums::Message::Empty(_) => None,
    }
}

fn dialog_unread_count(
    dialog: &grammers_client::grammers_tl_types::enums::Dialog,
) -> Result<u32, ListChatsSourceError> {
    let unread_raw = match dialog {
        grammers_client::grammers_tl_types::enums::Dialog::Dialog(data) => data.unread_count,
        grammers_client::grammers_tl_types::enums::Dialog::Folder(_data) => 0,
    };

    u32::try_from(unread_raw).map_err(|_| ListChatsSourceError::InvalidData)
}

fn dialog_is_pinned(dialog: &grammers_client::grammers_tl_types::enums::Dialog) -> bool {
    match dialog {
        grammers_client::grammers_tl_types::enums::Dialog::Dialog(data) => data.pinned,
        grammers_client::grammers_tl_types::enums::Dialog::Folder(_) => false,
    }
}

fn normalize_preview_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

async fn fetch_messages_from_chat(
    client: &grammers_client::Client,
    chat_id: i64,
    limit: usize,
    chat_cache: &RwLock<std::collections::HashMap<i64, grammers_client::types::PackedChat>>,
) -> Result<Vec<Message>, MessagesSourceError> {
    use grammers_client::grammers_tl_types::{
        enums::messages::Messages, functions::messages::GetHistory,
    };

    let packed_chat = {
        let cache = chat_cache.read().unwrap();
        cache.get(&chat_id).cloned()
    };

    let input_peer = match packed_chat {
        Some(packed) => packed.to_input_peer(),
        None => return Err(MessagesSourceError::ChatNotFound),
    };

    let limit = i32::try_from(limit.min(100)).map_err(|_| MessagesSourceError::InvalidData)?;

    let response = client
        .invoke(&GetHistory {
            peer: input_peer,
            offset_id: 0,
            offset_date: 0,
            add_offset: 0,
            limit,
            max_id: 0,
            min_id: 0,
            hash: 0,
        })
        .await
        .map_err(map_messages_invocation_error)?;

    let (raw_messages, users, chats) = match response {
        Messages::Messages(data) => (data.messages, data.users, data.chats),
        Messages::Slice(data) => (data.messages, data.users, data.chats),
        Messages::ChannelMessages(data) => (data.messages, data.users, data.chats),
        Messages::NotModified(_) => return Ok(Vec::new()),
    };

    let chat_map = grammers_client::types::ChatMap::new(users, chats);
    let mut messages = Vec::new();

    for raw_message in raw_messages {
        let (id, text, timestamp_ms, is_outgoing, sender_id, peer_id, media) = match &raw_message {
            grammers_client::grammers_tl_types::enums::Message::Message(data) => {
                let ts = data.date as i64 * 1000;
                let media_type = parse_message_media(&data.media);
                (
                    data.id,
                    data.message.clone(),
                    ts,
                    data.out,
                    peer_to_user_id(&data.from_id),
                    Some(data.peer_id.clone()),
                    media_type,
                )
            }
            grammers_client::grammers_tl_types::enums::Message::Service(data) => {
                let ts = data.date as i64 * 1000;
                (
                    data.id,
                    String::new(),
                    ts,
                    data.out,
                    peer_to_user_id(&data.from_id),
                    Some(data.peer_id.clone()),
                    MessageMedia::None,
                )
            }
            grammers_client::grammers_tl_types::enums::Message::Empty(_) => continue,
        };

        // Try to get sender name from from_id first, then fall back to peer_id
        // In private chats, from_id may be None, but peer_id points to the chat partner
        let sender_name = sender_id
            .and_then(|uid| {
                chat_map
                    .get(&grammers_client::grammers_tl_types::enums::Peer::User(
                        grammers_client::grammers_tl_types::types::PeerUser { user_id: uid },
                    ))
                    .map(|c| c.name().to_owned())
            })
            .or_else(|| {
                // Fallback: use peer_id for private chats (User peers only) when from_id is not available
                // Don't use this fallback for groups/channels to avoid showing group name as sender
                peer_id.as_ref().and_then(|peer| {
                    if matches!(
                        peer,
                        grammers_client::grammers_tl_types::enums::Peer::User(_)
                    ) {
                        chat_map.get(peer).map(|c| c.name().to_owned())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "Unknown".to_owned());

        messages.push(Message {
            id,
            sender_name,
            text,
            timestamp_ms,
            is_outgoing,
            media,
        });
    }

    messages.reverse();
    Ok(messages)
}

fn peer_to_user_id(peer: &Option<grammers_client::grammers_tl_types::enums::Peer>) -> Option<i64> {
    match peer {
        Some(grammers_client::grammers_tl_types::enums::Peer::User(u)) => Some(u.user_id),
        _ => None,
    }
}

fn parse_message_media(
    media: &Option<grammers_client::grammers_tl_types::enums::MessageMedia>,
) -> MessageMedia {
    use grammers_client::grammers_tl_types::enums::MessageMedia as TgMedia;

    let Some(media) = media else {
        return MessageMedia::None;
    };

    match media {
        TgMedia::Empty => MessageMedia::None,
        TgMedia::Photo(_) => MessageMedia::Photo,
        TgMedia::Geo(_) | TgMedia::GeoLive(_) | TgMedia::Venue(_) => MessageMedia::Location,
        TgMedia::Contact(_) => MessageMedia::Contact,
        TgMedia::Poll(_) => MessageMedia::Poll,
        TgMedia::Document(doc) => parse_document_media(doc),
        TgMedia::WebPage(_) => MessageMedia::None, // Web previews are not shown as media
        _ => MessageMedia::Other,
    }
}

fn parse_document_media(
    doc: &grammers_client::grammers_tl_types::types::MessageMediaDocument,
) -> MessageMedia {
    use grammers_client::grammers_tl_types::enums::{Document, DocumentAttribute};

    let Some(document) = &doc.document else {
        return MessageMedia::Document;
    };

    let Document::Document(data) = document else {
        return MessageMedia::Document;
    };

    // Check attributes to determine document type
    for attr in &data.attributes {
        match attr {
            DocumentAttribute::Sticker(_) => return MessageMedia::Sticker,
            DocumentAttribute::Video(v) if v.round_message => return MessageMedia::VideoNote,
            DocumentAttribute::Video(_) => return MessageMedia::Video,
            DocumentAttribute::Audio(a) if a.voice => return MessageMedia::Voice,
            DocumentAttribute::Audio(_) => return MessageMedia::Audio,
            DocumentAttribute::Animated => return MessageMedia::Animation,
            _ => {}
        }
    }

    // Check mime type for GIFs
    if data.mime_type == "image/gif" {
        return MessageMedia::Animation;
    }

    MessageMedia::Document
}

fn map_messages_invocation_error(error: impl std::fmt::Display) -> MessagesSourceError {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("unauthorized") || message.contains("auth") || message.contains("session") {
        return MessagesSourceError::Unauthorized;
    }
    if message.contains("channel") || message.contains("chat") || message.contains("peer") {
        return MessagesSourceError::ChatNotFound;
    }
    MessagesSourceError::Unavailable
}

fn map_list_chats_invocation_error(error: impl std::fmt::Display) -> ListChatsSourceError {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("unauthorized") || message.contains("auth") || message.contains("session") {
        return ListChatsSourceError::Unauthorized;
    }

    ListChatsSourceError::Unavailable
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

fn verify_password_with_token<T, F>(
    password_token: &mut Option<T>,
    state: &mut LoginState,
    password: &str,
    checker: F,
) -> Result<(), AuthBackendError>
where
    T: Clone,
    F: FnOnce(T, &str) -> Result<(), AuthBackendError>,
{
    let active_token = password_token
        .as_ref()
        .cloned()
        .ok_or(AuthBackendError::Transient {
            code: "AUTH_INVALID_FLOW",
            message: "password verification requested before password challenge".to_owned(),
        })?;

    let result = checker(active_token, password);
    apply_password_verification_outcome(password_token, state, &result);

    result
}

fn apply_password_verification_outcome<T>(
    password_token: &mut Option<T>,
    state: &mut LoginState,
    result: &Result<(), AuthBackendError>,
) {
    match result {
        Ok(()) => {
            *password_token = None;
            *state = LoginState::Authorized;
        }
        Err(_) => {
            *state = LoginState::PasswordRequired;
        }
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
    fn auth_runtime_enables_io_driver_for_tcp() {
        let rt = build_auth_runtime().expect("runtime should initialize");

        let bind_result = rt.block_on(async { tokio::net::TcpListener::bind("127.0.0.1:0").await });

        assert!(bind_result.is_ok(), "io driver should support tcp bind");
    }

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

    #[test]
    fn password_verification_error_keeps_password_token_for_retry() {
        let mut token = Some(7_u8);
        let mut state = LoginState::Connecting;

        apply_password_verification_outcome(
            &mut token,
            &mut state,
            &Err(AuthBackendError::WrongPassword),
        );

        assert_eq!(token, Some(7));
        assert_eq!(state, LoginState::PasswordRequired);
    }

    #[test]
    fn verify_password_path_rejects_missing_password_challenge() {
        let mut token = None::<u8>;
        let mut state = LoginState::CodeRequired;

        let result =
            verify_password_with_token(&mut token, &mut state, "secret", |_token, _| Ok(()));

        assert_eq!(
            result,
            Err(AuthBackendError::Transient {
                code: "AUTH_INVALID_FLOW",
                message: "password verification requested before password challenge".to_owned(),
            })
        );
        assert_eq!(state, LoginState::CodeRequired);
    }

    #[test]
    fn verify_password_path_keeps_token_after_failure_and_allows_retry() {
        let mut token = Some(42_u8);
        let mut state = LoginState::PasswordRequired;

        let first_attempt =
            verify_password_with_token(&mut token, &mut state, "wrong", |_token, _| {
                Err(AuthBackendError::WrongPassword)
            });

        assert_eq!(first_attempt, Err(AuthBackendError::WrongPassword));
        assert_eq!(token, Some(42));
        assert_eq!(state, LoginState::PasswordRequired);

        let second_attempt =
            verify_password_with_token(&mut token, &mut state, "correct", |_token, _| Ok(()));

        assert_eq!(second_attempt, Ok(()));
        assert_eq!(token, None);
        assert_eq!(state, LoginState::Authorized);
    }

    #[test]
    fn password_verification_success_clears_password_token() {
        let mut token = Some(7_u8);
        let mut state = LoginState::PasswordRequired;

        apply_password_verification_outcome(&mut token, &mut state, &Ok(()));

        assert_eq!(token, None);
        assert_eq!(state, LoginState::Authorized);
    }

    #[test]
    fn normalize_preview_text_trims_and_drops_empty_values() {
        assert_eq!(
            normalize_preview_text("  hello  "),
            Some("hello".to_owned())
        );
        assert_eq!(normalize_preview_text("   \n\t  "), None);
    }

    #[test]
    fn maps_list_chats_auth_errors_to_unauthorized() {
        let error = map_list_chats_invocation_error("AUTH_KEY_UNREGISTERED");
        assert_eq!(error, ListChatsSourceError::Unauthorized);
    }

    #[test]
    fn selects_main_folder_scope_when_custom_filters_exist() {
        let filters = vec![
            grammers_client::grammers_tl_types::enums::DialogFilter::Default,
            grammers_client::grammers_tl_types::enums::DialogFilter::Filter(
                grammers_client::grammers_tl_types::types::DialogFilter {
                    contacts: false,
                    non_contacts: false,
                    groups: false,
                    broadcasts: false,
                    bots: false,
                    exclude_muted: false,
                    exclude_read: false,
                    exclude_archived: false,
                    id: 1,
                    title: "Work".to_owned(),
                    emoticon: None,
                    color: None,
                    pinned_peers: Vec::new(),
                    include_peers: Vec::new(),
                    exclude_peers: Vec::new(),
                },
            ),
        ];

        let scope = dialog_fetch_scope_from_filters(&filters);

        assert_eq!(scope, DialogFetchScope::MainFolderOnly);
    }

    #[test]
    fn keeps_all_dialog_scope_when_only_default_filter_exists() {
        let filters = vec![grammers_client::grammers_tl_types::enums::DialogFilter::Default];

        let scope = dialog_fetch_scope_from_filters(&filters);

        assert_eq!(scope, DialogFetchScope::AllDialogs);
    }
}
