use std::sync::mpsc;

use tdlib_rs::enums::{AuthorizationState, ConnectionState, Update};

use super::types::{AuthStateUpdate, UPDATE_POLL_INTERVAL};
use super::TdLibClient;
use crate::domain::events::ConnectivityStatus;
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_mappers::sum_reaction_counts;
use crate::telegram::tdlib_updates::TdLibUpdate;

impl TdLibClient {
    pub(super) fn publish_unread_reaction_count(
        update_tx: &mpsc::Sender<TdLibUpdate>,
        cache: &TdLibCache,
        chat_id: i64,
        unread_reaction_count: i32,
    ) {
        cache.update_chat_unread_reaction_count(chat_id, unread_reaction_count);
        let _ = update_tx.send(TdLibUpdate::ChatUnreadReactionCount { chat_id });
    }

    /// Background loop that receives and processes TDLib updates.
    ///
    /// This is a fully synchronous function that runs in a dedicated thread.
    /// It continuously polls `tdlib_rs::receive()` and dispatches updates
    /// through the appropriate channels.
    pub(super) fn run_update_loop(
        client_id: i32,
        auth_state_tx: mpsc::Sender<AuthStateUpdate>,
        update_tx: mpsc::Sender<TdLibUpdate>,
        connectivity_tx: mpsc::Sender<ConnectivityStatus>,
        cache: TdLibCache,
    ) {
        tracing::debug!(client_id, "Starting TDLib update loop");

        loop {
            match tdlib_rs::receive() {
                Some((update, received_client_id)) => {
                    if received_client_id != client_id {
                        continue;
                    }

                    match update {
                        // Connection state updates — drive the connectivity
                        // status indicator in the UI status bar.
                        Update::ConnectionState(u) => {
                            let status = map_connection_state(&u.state);
                            tracing::debug!(?u.state, ?status, "TDLib connection state changed");
                            if connectivity_tx.send(status).is_err() {
                                tracing::debug!(
                                    "Connectivity receiver dropped, ignoring further state changes"
                                );
                            }
                        }

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

                        // Cache population: TDLib guarantees these arrive before
                        // the corresponding IDs appear in any response.
                        Update::NewChat(u) => {
                            cache.upsert_chat(u.chat.clone());
                            let _ = update_tx.send(TdLibUpdate::NewChat {
                                chat: Box::new(u.chat),
                            });
                        }
                        Update::User(u) => {
                            cache.upsert_user(u.user);
                        }

                        // TDLib option updates (e.g. "my_id" for current user)
                        Update::Option(u) => {
                            if u.name == "my_id" {
                                if let tdlib_rs::enums::OptionValue::Integer(v) = u.value {
                                    cache.set_my_user_id(v.value);
                                }
                            }
                        }

                        // Message updates
                        Update::NewMessage(u) => {
                            let _ = update_tx.send(TdLibUpdate::NewMessage {
                                chat_id: u.message.chat_id,
                                message: Box::new(u.message),
                            });
                        }
                        Update::MessageContent(u) => {
                            let _ = update_tx.send(TdLibUpdate::MessageContentChanged {
                                chat_id: u.chat_id,
                                message_id: u.message_id,
                                new_content: Box::new(u.new_content),
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

                        // Chat list updates — also write through to cache
                        Update::ChatLastMessage(u) => {
                            cache.update_chat_last_message(u.chat_id, u.last_message, u.positions);
                            let _ =
                                update_tx.send(TdLibUpdate::ChatLastMessage { chat_id: u.chat_id });
                        }
                        Update::ChatPosition(u) => {
                            cache.update_chat_position(u.chat_id, u.position);
                            let _ =
                                update_tx.send(TdLibUpdate::ChatPosition { chat_id: u.chat_id });
                        }

                        // Read status updates — also write through to cache
                        Update::ChatReadInbox(u) => {
                            cache.update_chat_read_inbox(
                                u.chat_id,
                                u.unread_count,
                                u.last_read_inbox_message_id,
                            );
                            let _ =
                                update_tx.send(TdLibUpdate::ChatReadInbox { chat_id: u.chat_id });
                        }
                        Update::ChatReadOutbox(u) => {
                            cache.update_chat_read_outbox(u.chat_id, u.last_read_outbox_message_id);
                            let _ =
                                update_tx.send(TdLibUpdate::ChatReadOutbox { chat_id: u.chat_id });
                        }

                        // User status updates — write through to cache
                        Update::UserStatus(u) => {
                            cache.update_user_status(u.user_id, u.status);
                            let _ = update_tx.send(TdLibUpdate::UserStatus { user_id: u.user_id });
                        }

                        // Reaction updates
                        Update::ChatUnreadReactionCount(u) => {
                            Self::publish_unread_reaction_count(
                                &update_tx,
                                &cache,
                                u.chat_id,
                                u.unread_reaction_count,
                            );
                        }
                        Update::MessageInteractionInfo(u) => {
                            let reaction_count = sum_reaction_counts(u.interaction_info.as_ref());
                            let _ = update_tx.send(TdLibUpdate::MessageInteractionInfoChanged {
                                chat_id: u.chat_id,
                                message_id: u.message_id,
                                reaction_count,
                            });
                        }
                        Update::MessageUnreadReactions(u) => {
                            Self::publish_unread_reaction_count(
                                &update_tx,
                                &cache,
                                u.chat_id,
                                u.unread_reaction_count,
                            );
                        }

                        // File download progress updates
                        Update::File(u) => {
                            let _ = update_tx.send(TdLibUpdate::FileUpdated {
                                file_id: u.file.id,
                                size: u.file.size,
                                expected_size: u.file.expected_size,
                                local_path: u.file.local.path,
                                is_downloading_active: u.file.local.is_downloading_active,
                                is_downloading_completed: u.file.local.is_downloading_completed,
                                downloaded_size: u.file.local.downloaded_size,
                            });
                        }

                        // Chat action updates (typing indicators)
                        Update::ChatAction(u) => {
                            let sender_user_id = match u.sender_id {
                                tdlib_rs::enums::MessageSender::User(ref s) => s.user_id,
                                _ => 0,
                            };
                            if sender_user_id != 0 {
                                let is_cancel =
                                    matches!(u.action, tdlib_rs::enums::ChatAction::Cancel);
                                let action_label = map_chat_action_label(&u.action);
                                let sender_name = cache
                                    .get_user(sender_user_id)
                                    .map(|user| {
                                        crate::telegram::tdlib_mappers::format_user_name(&user)
                                    })
                                    .unwrap_or_default();
                                let _ = update_tx.send(TdLibUpdate::ChatAction {
                                    chat_id: u.chat_id,
                                    sender_user_id,
                                    sender_name,
                                    action_label: action_label.to_owned(),
                                    is_cancel,
                                });
                            }
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
}

/// Maps TDLib's `ConnectionState` to the domain `ConnectivityStatus`.
///
/// `WaitingForNetwork` reflects an unavailable device network (Telegram
/// considers no path even possible), so it is surfaced as `Disconnected`.
/// `ConnectingToProxy` and `Connecting` both mean a connection attempt is
/// in flight, including the case where Telegram domains are blocked and
/// retries keep failing — those map to `Connecting`. `Updating` reflects
/// catch-up of missed updates, distinct from a steady `Connected` state.
pub(super) fn map_connection_state(state: &ConnectionState) -> ConnectivityStatus {
    match state {
        ConnectionState::WaitingForNetwork => ConnectivityStatus::Disconnected,
        ConnectionState::ConnectingToProxy | ConnectionState::Connecting => {
            ConnectivityStatus::Connecting
        }
        ConnectionState::Updating => ConnectivityStatus::Updating,
        ConnectionState::Ready => ConnectivityStatus::Connected,
    }
}

fn map_chat_action_label(action: &tdlib_rs::enums::ChatAction) -> &'static str {
    use tdlib_rs::enums::ChatAction;
    match action {
        ChatAction::Typing => "typing",
        ChatAction::RecordingVideo => "recording video",
        ChatAction::UploadingVideo(_) => "uploading video",
        ChatAction::RecordingVoiceNote => "recording voice",
        ChatAction::UploadingVoiceNote(_) => "uploading voice",
        ChatAction::UploadingPhoto(_) => "uploading photo",
        ChatAction::UploadingDocument(_) => "uploading document",
        ChatAction::ChoosingSticker => "choosing sticker",
        ChatAction::ChoosingLocation => "choosing location",
        ChatAction::ChoosingContact => "choosing contact",
        ChatAction::StartPlayingGame => "playing game",
        ChatAction::RecordingVideoNote => "recording video",
        ChatAction::UploadingVideoNote(_) => "uploading video",
        ChatAction::WatchingAnimations(_) => "watching animation",
        ChatAction::Cancel => "cancel",
    }
}
