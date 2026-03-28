use std::sync::mpsc;

use tdlib_rs::enums::{AuthorizationState, Update};

use super::types::{AuthStateUpdate, UPDATE_POLL_INTERVAL};
use super::TdLibClient;
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
