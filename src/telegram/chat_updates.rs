//! Chat updates monitor for TDLib.
//!
//! Receives typed TDLib updates, maps message-carrying variants to domain
//! types via `MessageMapper`, and forwards `ChatUpdate` events downstream
//! for cache warming and UI refresh.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::domain::events::ChatUpdate;
use crate::domain::message::Message;

use super::tdlib_updates::TdLibUpdate;

const CHAT_UPDATES_MONITOR_STARTED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STARTED";
const CHAT_UPDATES_MONITOR_STOPPED: &str = "TELEGRAM_CHAT_UPDATES_MONITOR_STOPPED";
const CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED: &str =
    "TELEGRAM_CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED";

/// Timeout for receiving updates from TDLib channel.
const UPDATE_RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// Maps a raw TDLib message to a domain `Message`.
///
/// Implemented in the telegram layer where TDLib client access is available
/// for resolving sender names via `get_user`.
pub trait MessageMapper: Send + Sync {
    fn map_message(&self, raw: &tdlib_rs::types::Message) -> Message;
}

/// Monitor that converts TDLib typed updates to domain `ChatUpdate` events.
///
/// Runs a background thread that reads `TdLibUpdate` from a channel,
/// maps message data to domain types, and sends `ChatUpdate` events
/// for cache warming and UI refresh.
#[derive(Debug)]
pub struct TelegramChatUpdatesMonitor {
    /// Worker thread handle. Kept for debugging but not joined on drop.
    #[allow(dead_code)]
    worker: Option<JoinHandle<()>>,
}

impl TelegramChatUpdatesMonitor {
    /// Starts the chat updates monitor with a TDLib update receiver.
    ///
    /// # Arguments
    /// - `update_rx`: Receiver for typed TDLib updates from `TdLibClient::take_update_receiver()`
    /// - `signal_tx`: Sender for domain `ChatUpdate` events consumed by the event source
    /// - `mapper`: Maps raw TDLib messages to domain `Message` types
    pub fn start(
        update_rx: Receiver<TdLibUpdate>,
        signal_tx: Sender<ChatUpdate>,
        mapper: Arc<dyn MessageMapper>,
    ) -> Result<Self, ChatUpdatesMonitorStartError> {
        // Test switch for failure injection
        if std::env::var("RTG_TELEGRAM_CHAT_UPDATES_MONITOR_FAIL")
            .ok()
            .as_deref()
            == Some("1")
        {
            return Err(ChatUpdatesMonitorStartError::StartupRejected);
        }

        let worker = thread::Builder::new()
            .name("rtg-chat-updates".into())
            .spawn(move || {
                run_update_monitor(update_rx, signal_tx, &*mapper);
            })
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to spawn chat updates monitor thread");
                ChatUpdatesMonitorStartError::StartupRejected
            })?;

        tracing::info!(
            code = CHAT_UPDATES_MONITOR_STARTED,
            "telegram chat updates monitor started"
        );

        Ok(Self {
            worker: Some(worker),
        })
    }

    /// Creates an inert monitor for testing (no background thread).
    #[cfg(test)]
    pub fn inert() -> Self {
        Self { worker: None }
    }
}

impl Drop for TelegramChatUpdatesMonitor {
    fn drop(&mut self) {
        tracing::debug!("TelegramChatUpdatesMonitor dropped");
    }
}

/// Converts a `TdLibUpdate` into a domain `ChatUpdate`.
///
/// Message-carrying updates are mapped via the `MessageMapper`.
/// Non-message updates (chat metadata, read state, etc.) become
/// `ChatMetadataChanged`. User status updates are skipped (no chat_id).
fn map_update(update: TdLibUpdate, mapper: &dyn MessageMapper) -> Option<ChatUpdate> {
    match update {
        TdLibUpdate::NewChat { chat } => Some(ChatUpdate::ChatMetadataChanged { chat_id: chat.id }),
        TdLibUpdate::NewMessage { chat_id, message } => {
            let domain_msg = mapper.map_message(&message);
            Some(ChatUpdate::NewMessage {
                chat_id,
                message: Box::new(domain_msg),
            })
        }
        TdLibUpdate::DeleteMessages {
            chat_id,
            message_ids,
        } => Some(ChatUpdate::MessagesDeleted {
            chat_id,
            message_ids,
        }),
        TdLibUpdate::MessageContentChanged { chat_id, .. }
        | TdLibUpdate::ChatLastMessage { chat_id }
        | TdLibUpdate::ChatPosition { chat_id }
        | TdLibUpdate::ChatReadInbox { chat_id }
        | TdLibUpdate::ChatReadOutbox { chat_id }
        | TdLibUpdate::MessageSendSucceeded { chat_id, .. }
        | TdLibUpdate::ChatUnreadReactionCount { chat_id } => {
            Some(ChatUpdate::ChatMetadataChanged { chat_id })
        }
        TdLibUpdate::MessageInteractionInfoChanged {
            chat_id,
            message_id,
            reaction_count,
        } => Some(ChatUpdate::MessageReactionsChanged {
            chat_id,
            message_id,
            reaction_count,
        }),
        TdLibUpdate::FileUpdated {
            file_id,
            size,
            expected_size,
            local_path,
            is_downloading_active,
            is_downloading_completed,
            downloaded_size,
        } => {
            let effective_size = if size > 0 {
                size.max(0) as u64
            } else {
                expected_size.max(0) as u64
            };
            Some(ChatUpdate::FileUpdated {
                file_id,
                size: effective_size,
                local_path,
                is_downloading_active,
                is_downloading_completed,
                downloaded_size: downloaded_size.max(0) as u64,
            })
        }
        TdLibUpdate::UserStatus { user_id } => Some(ChatUpdate::UserStatusChanged { user_id }),
    }
}

/// Background loop that processes TDLib updates and sends domain events.
fn run_update_monitor(
    update_rx: Receiver<TdLibUpdate>,
    signal_tx: Sender<ChatUpdate>,
    mapper: &dyn MessageMapper,
) {
    loop {
        match update_rx.recv_timeout(UPDATE_RECV_TIMEOUT) {
            Ok(update) => {
                let kind = update.kind();
                tracing::debug!(
                    update_kind = kind,
                    "telegram update observed by chat monitor"
                );

                let Some(chat_update) = map_update(update, mapper) else {
                    tracing::debug!(update_kind = kind, "update has no chat_id, skipping");
                    continue;
                };

                if signal_tx.send(chat_update).is_err() {
                    tracing::warn!(
                        code = CHAT_UPDATES_MONITOR_SIGNAL_SEND_FAILED,
                        "chat updates monitor failed to send signal; stopping"
                    );
                    break;
                }

                tracing::debug!(update_kind = kind, "chat updates monitor forwarded update");
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                tracing::info!(
                    code = CHAT_UPDATES_MONITOR_STOPPED,
                    "telegram chat updates monitor stopped (channel closed)"
                );
                break;
            }
        }
    }
}

/// Error type for chat updates monitor startup.
#[derive(Debug)]
pub enum ChatUpdatesMonitorStartError {
    /// Monitor startup was rejected (test switch or spawn failure).
    StartupRejected,
}

impl std::fmt::Display for ChatUpdatesMonitorStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartupRejected => f.write_str("startup rejected"),
        }
    }
}

impl std::error::Error for ChatUpdatesMonitorStartError {}

/// Stub mapper for tests that creates a minimal domain Message.
#[cfg(test)]
pub struct StubMessageMapper;

#[cfg(test)]
impl MessageMapper for StubMessageMapper {
    fn map_message(&self, raw: &tdlib_rs::types::Message) -> Message {
        use super::tdlib_mappers;
        let text = tdlib_mappers::extract_message_text(&raw.content);
        let media = tdlib_mappers::extract_message_media(&raw.content);
        let file_info = tdlib_mappers::extract_file_info(&raw.content);
        Message {
            id: raw.id,
            sender_name: "TestUser".to_owned(),
            text,
            timestamp_ms: i64::from(raw.date) * 1000,
            is_outgoing: raw.is_outgoing,
            media,
            status: crate::domain::message::MessageStatus::Delivered,
            file_info,
            call_info: None,
            reply_to: None,
            forward_info: None,
            reaction_count: 0,
            links: Vec::new(),
            is_edited: raw.edit_date > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::{MessageMedia, MessageStatus};
    use std::sync::mpsc;

    fn make_test_td_message(id: i64, chat_id: i64, text: &str) -> tdlib_rs::types::Message {
        use tdlib_rs::enums::{MessageContent, MessageSender};
        tdlib_rs::types::Message {
            id,
            sender_id: MessageSender::User(tdlib_rs::types::MessageSenderUser { user_id: 1 }),
            chat_id,
            sending_state: None,
            scheduling_state: None,
            is_outgoing: false,
            is_pinned: false,
            is_from_offline: false,
            can_be_saved: true,
            has_timestamped_media: false,
            is_channel_post: false,
            is_paid_star_suggested_post: false,
            is_paid_ton_suggested_post: false,
            contains_unread_mention: false,
            date: 1609459200,
            edit_date: 0,
            forward_info: None,
            import_info: None,
            interaction_info: None,
            unread_reactions: vec![],
            fact_check: None,
            suggested_post_info: None,
            reply_to: None,
            topic_id: None,
            self_destruct_type: None,
            self_destruct_in: 0.0,
            auto_delete_in: 0.0,
            via_bot_user_id: 0,
            sender_business_bot_user_id: 0,
            sender_boost_count: 0,
            paid_message_star_count: 0,
            author_signature: String::new(),
            media_album_id: 0,
            effect_id: 0,
            restriction_info: None,
            summary_language_code: String::new(),
            content: MessageContent::MessageText(tdlib_rs::types::MessageText {
                text: tdlib_rs::types::FormattedText {
                    text: text.to_owned(),
                    entities: vec![],
                },
                link_preview: None,
                link_preview_options: None,
            }),
            reply_markup: None,
        }
    }

    #[test]
    fn monitor_maps_new_message_to_chat_update() {
        let (update_tx, update_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();
        let mapper = Arc::new(StubMessageMapper);

        let monitor = TelegramChatUpdatesMonitor::start(update_rx, signal_tx, mapper)
            .expect("monitor should start");

        let td_msg = make_test_td_message(42, 123, "Hello from push");
        update_tx
            .send(TdLibUpdate::NewMessage {
                chat_id: 123,
                message: Box::new(td_msg),
            })
            .expect("send should succeed");

        let result = signal_rx.recv_timeout(Duration::from_millis(500));
        match result {
            Ok(ChatUpdate::NewMessage { chat_id, message }) => {
                assert_eq!(chat_id, 123);
                assert_eq!(message.id, 42);
                assert_eq!(message.text, "Hello from push");
                assert_eq!(message.sender_name, "TestUser");
                assert_eq!(message.media, MessageMedia::None);
                assert_eq!(message.status, MessageStatus::Delivered);
            }
            other => panic!("expected NewMessage, got: {other:?}"),
        }

        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn monitor_maps_delete_messages() {
        let (update_tx, update_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();
        let mapper = Arc::new(StubMessageMapper);

        let monitor = TelegramChatUpdatesMonitor::start(update_rx, signal_tx, mapper)
            .expect("monitor should start");

        update_tx
            .send(TdLibUpdate::DeleteMessages {
                chat_id: 100,
                message_ids: vec![1, 2, 3],
            })
            .expect("send should succeed");

        let result = signal_rx.recv_timeout(Duration::from_millis(500));
        match result {
            Ok(ChatUpdate::MessagesDeleted {
                chat_id,
                message_ids,
            }) => {
                assert_eq!(chat_id, 100);
                assert_eq!(message_ids, vec![1, 2, 3]);
            }
            other => panic!("expected MessagesDeleted, got: {other:?}"),
        }

        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn monitor_maps_metadata_updates() {
        let (update_tx, update_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();
        let mapper = Arc::new(StubMessageMapper);

        let monitor = TelegramChatUpdatesMonitor::start(update_rx, signal_tx, mapper)
            .expect("monitor should start");

        update_tx
            .send(TdLibUpdate::ChatReadInbox { chat_id: 50 })
            .expect("send should succeed");

        let result = signal_rx.recv_timeout(Duration::from_millis(500));
        match result {
            Ok(ChatUpdate::ChatMetadataChanged { chat_id }) => {
                assert_eq!(chat_id, 50);
            }
            other => panic!("expected ChatMetadataChanged, got: {other:?}"),
        }

        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn monitor_forwards_user_status_updates() {
        let (update_tx, update_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();
        let mapper = Arc::new(StubMessageMapper);

        let monitor = TelegramChatUpdatesMonitor::start(update_rx, signal_tx, mapper)
            .expect("monitor should start");

        update_tx
            .send(TdLibUpdate::UserStatus { user_id: 456 })
            .expect("send should succeed");

        let result = signal_rx.recv_timeout(Duration::from_millis(500));
        match result {
            Ok(ChatUpdate::UserStatusChanged { user_id }) => {
                assert_eq!(user_id, 456);
            }
            other => panic!("expected UserStatusChanged, got: {other:?}"),
        }

        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn monitor_stops_when_channel_closed() {
        let (update_tx, update_rx) = mpsc::channel::<TdLibUpdate>();
        let (signal_tx, _signal_rx) = mpsc::channel();
        let mapper = Arc::new(StubMessageMapper);

        let monitor = TelegramChatUpdatesMonitor::start(update_rx, signal_tx, mapper)
            .expect("monitor should start");

        drop(update_tx);
        drop(monitor);
    }

    #[test]
    fn inert_monitor_has_no_worker() {
        let monitor = TelegramChatUpdatesMonitor::inert();
        assert!(monitor.worker.is_none());
    }

    #[test]
    fn map_chat_unread_reaction_count_to_metadata_changed() {
        let mapper = StubMessageMapper;
        let update = TdLibUpdate::ChatUnreadReactionCount { chat_id: 42 };

        let result = map_update(update, &mapper);

        assert!(
            matches!(
                result,
                Some(ChatUpdate::ChatMetadataChanged { chat_id: 42 })
            ),
            "expected ChatMetadataChanged, got: {result:?}"
        );
    }

    #[test]
    fn map_message_interaction_info_to_reactions_changed() {
        let mapper = StubMessageMapper;
        let update = TdLibUpdate::MessageInteractionInfoChanged {
            chat_id: 10,
            message_id: 20,
            reaction_count: 5,
        };

        let result = map_update(update, &mapper);

        match result {
            Some(ChatUpdate::MessageReactionsChanged {
                chat_id,
                message_id,
                reaction_count,
            }) => {
                assert_eq!(chat_id, 10);
                assert_eq!(message_id, 20);
                assert_eq!(reaction_count, 5);
            }
            other => panic!("expected MessageReactionsChanged, got: {other:?}"),
        }
    }

    #[test]
    fn map_user_status_to_user_status_changed() {
        let mapper = StubMessageMapper;
        let update = TdLibUpdate::UserStatus { user_id: 42 };

        let result = map_update(update, &mapper);

        assert!(
            matches!(result, Some(ChatUpdate::UserStatusChanged { user_id: 42 })),
            "expected UserStatusChanged, got: {result:?}"
        );
    }
}
