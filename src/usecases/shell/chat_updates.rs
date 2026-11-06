use std::collections::HashSet;

use crate::{
    domain::{chat::ChatType, events::ChatUpdate},
    usecases::{background::TaskDispatcher, chat_subtitle::ChatSubtitleQuery},
};

use super::{chat_list, chat_open::OpenChatSync, file_updates, OrchestratorCtx, ShellCtx};

/// Processes push updates from TDLib for cache warming and UI refresh.
///
/// - `NewMessage`: inserts into `MessageCache` for any chat (warm cache passively)
/// - `MessagesDeleted`: removes from `MessageCache`
/// - `ChatMetadataChanged`: triggers chat list refresh
///
/// For the currently open chat, also dispatches a message refresh.
pub(super) fn handle_chat_updates<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    updates: Vec<ChatUpdate>,
) {
    let mut reload_chat_ids = HashSet::new();
    let mut should_refresh_chat_list = false;

    for update in updates {
        match update {
            ChatUpdate::NewMessage { chat_id, message } => {
                tracing::debug!(chat_id, message_id = message.id, "caching pushed message");
                file_updates::maybe_auto_download(&mut ctx.shell, ctx.downloads, chat_id, &message);
                ctx.shell
                    .state
                    .message_cache_mut()
                    .add_message(chat_id, *message);
                reload_chat_ids.insert(chat_id);
                should_refresh_chat_list = true;
            }
            ChatUpdate::MessagesDeleted {
                chat_id,
                message_ids,
            } => {
                tracing::debug!(
                    chat_id,
                    count = message_ids.len(),
                    "removing deleted messages from cache"
                );
                ctx.shell
                    .state
                    .message_cache_mut()
                    .remove_messages(chat_id, &message_ids);
                reload_chat_ids.insert(chat_id);
                should_refresh_chat_list = true;
            }
            ChatUpdate::ChatMetadataChanged { chat_id } => {
                should_refresh_chat_list = true;
                reload_chat_ids.insert(chat_id);
            }
            ChatUpdate::MessageReactionsChanged {
                chat_id,
                message_id,
                reaction_count,
            } => {
                ctx.shell.state.message_cache_mut().update_reaction_count(
                    chat_id,
                    message_id,
                    reaction_count,
                );
                if ctx.shell.state.open_chat().chat_id() == Some(chat_id) {
                    ctx.shell
                        .state
                        .open_chat_mut()
                        .update_message_reaction_count(message_id, reaction_count);
                }
            }
            ChatUpdate::UserStatusChanged { user_id } => {
                // Re-resolve subtitle for the open private chat if it belongs
                // to the user whose status changed.
                if let Some(chat_id) = ctx.shell.state.open_chat().chat_id() {
                    let chat_type = ctx.shell.state.open_chat().chat_type();
                    if chat_type == ChatType::Private {
                        ctx.shell
                            .dispatcher
                            .dispatch_chat_subtitle(ChatSubtitleQuery { chat_id, chat_type });
                        tracing::debug!(
                            user_id,
                            chat_id,
                            "user status changed, re-resolving chat subtitle"
                        );
                    }
                }
                should_refresh_chat_list = true;
            }
            ChatUpdate::ChatActionChanged {
                chat_id,
                sender_user_id,
                sender_name,
                action_label,
                is_cancel,
            } => {
                if ctx.shell.state.open_chat().chat_id() == Some(chat_id) {
                    if is_cancel {
                        ctx.shell
                            .state
                            .open_chat_mut()
                            .typing_state_mut()
                            .remove_action(sender_user_id);
                    } else {
                        ctx.shell
                            .state
                            .open_chat_mut()
                            .typing_state_mut()
                            .add_action(sender_user_id, sender_name, action_label);
                    }
                }
            }
            ChatUpdate::FileUpdated {
                file_id,
                size,
                downloaded_size,
                local_path,
                download_status,
            } => {
                file_updates::handle_file_update(
                    &mut ctx.shell,
                    ctx.downloads,
                    file_id,
                    size,
                    downloaded_size,
                    local_path,
                    download_status,
                );
            }
            ChatUpdate::ForumTopicChanged {
                chat_id,
                topic_id: _,
                unread_topic_count,
            } => {
                // Patch the forum's root-list badge in place — no full chat-list
                // refresh. The count comes from the update-driven topic cache;
                // `None` means the cache isn't seeded for this chat yet, and the
                // badge warm-up dispatched on chat-list load covers it.
                if let Some(count) = unread_topic_count {
                    ctx.shell
                        .state
                        .chat_list_mut()
                        .set_forum_unread_topic_count(chat_id, count);
                }
                // Only reload the topic panel when the user is browsing this very
                // forum — no point refreshing topics they can't see right now.
                if ctx
                    .shell
                    .state
                    .forum_topic_list()
                    .map(|f| f.parent_chat_id())
                    == Some(chat_id)
                {
                    ctx.shell.dispatcher.dispatch_load_forum_topics(chat_id);
                }
            }
            ChatUpdate::ChatFoldersChanged { folders } => {
                tracing::debug!(count = folders.folders.len(), "chat folders changed");
                ctx.shell.state.set_chat_folders(folders);
            }
        }
    }

    if should_refresh_chat_list {
        chat_list::dispatch_chat_list_refresh(&mut ctx.shell, ctx.chat_list_loads, false);
    }
    if !reload_chat_ids.is_empty() {
        maybe_refresh_open_chat_messages(&mut ctx.shell, ctx.open_chat_sync, &reload_chat_ids);
    }
}

pub(super) fn maybe_refresh_open_chat_messages<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    affected_chat_ids: &HashSet<i64>,
) {
    use crate::domain::open_chat_state::OpenChatUiState;

    if sync.messages_refresh_in_flight {
        return;
    }

    let Some(open_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    if ctx.state.open_chat().ui_state() != OpenChatUiState::Ready {
        return;
    }

    if !affected_chat_ids.contains(&open_id) {
        return;
    }

    tracing::debug!(
        chat_id = open_id,
        "refreshing open chat messages from update"
    );
    sync.messages_refresh_in_flight = true;
    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher.dispatch_load_messages(open_id, topic_id);
}
