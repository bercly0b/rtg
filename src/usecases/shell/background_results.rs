use crate::{
    domain::open_chat_state::{MessageSource, OpenChatUiState},
    usecases::background::{BackgroundTaskResult, TaskDispatcher},
};

use super::{chat_list, chat_open, popups, OrchestratorCtx};

pub(super) fn handle_background_result<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    result: BackgroundTaskResult,
) {
    match result {
        BackgroundTaskResult::ChatListLoaded {
            list,
            result,
            all_loaded,
        } => chat_list::handle_chat_list_loaded(
            &mut ctx.shell,
            ctx.chat_list_loads,
            list,
            result,
            all_loaded,
        ),
        BackgroundTaskResult::ForumUnreadCountsLoaded { counts } => {
            chat_list::handle_forum_unread_counts_loaded(
                &mut ctx.shell,
                ctx.chat_list_loads,
                counts,
            )
        }
        BackgroundTaskResult::ForumTopicsLoaded { chat_id, result } => {
            let Some(forum_list) = ctx.shell.state.forum_topic_list_mut() else {
                tracing::debug!(
                    chat_id,
                    "background: forum topics result with no open forum"
                );
                return;
            };
            if forum_list.parent_chat_id() != chat_id {
                tracing::debug!(
                    chat_id,
                    open_forum = forum_list.parent_chat_id(),
                    "background: discarding stale forum topics result"
                );
                return;
            }
            match result {
                Ok(topics) => {
                    tracing::debug!(
                        chat_id,
                        count = topics.len(),
                        "background: forum topics loaded"
                    );
                    forum_list.set_ready(topics);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: forum topics load failed"
                    );
                    forum_list.set_error();
                }
            }
        }
        BackgroundTaskResult::MessagesLoaded {
            chat_id,
            topic_id,
            result,
        } => {
            ctx.open_chat_sync.messages_refresh_in_flight = false;

            // Cache only chat-scoped results — the per-chat MessageCache cannot
            // express topic scope, so writing topic history under chat_id would
            // poison reads for the parent chat.
            if topic_id.is_none() {
                if let Ok(ref messages) = result {
                    ctx.shell
                        .state
                        .message_cache_mut()
                        .put(chat_id, messages.clone(), true);
                }
            }

            if ctx.shell.state.open_chat().chat_id() != Some(chat_id)
                || ctx.shell.state.open_chat().topic_id() != topic_id
            {
                tracing::debug!(
                    chat_id,
                    ?topic_id,
                    open_topic_id = ?ctx.shell.state.open_chat().topic_id(),
                    "background: discarding stale messages result (user navigated away)"
                );
                return;
            }

            match result {
                Ok(messages) => {
                    tracing::debug!(
                        chat_id,
                        message_count = messages.len(),
                        "background: messages loaded"
                    );
                    if ctx.shell.state.open_chat().ui_state() == OpenChatUiState::Ready {
                        ctx.shell.state.open_chat_mut().update_messages(messages);
                    } else {
                        ctx.shell.state.open_chat_mut().set_ready(messages);
                        ctx.shell
                            .state
                            .open_chat_mut()
                            .set_message_source(MessageSource::Live);
                    }
                    chat_open::mark_open_chat_messages_as_read(&mut ctx.shell);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: messages load failed"
                    );
                    ctx.shell.state.open_chat_mut().set_error();
                }
            }
        }
        BackgroundTaskResult::OlderMessagesLoaded {
            chat_id,
            topic_id,
            result,
        } => {
            ctx.open_chat_sync.older_messages_in_flight = false;

            if ctx.shell.state.open_chat().chat_id() != Some(chat_id)
                || ctx.shell.state.open_chat().topic_id() != topic_id
            {
                return;
            }

            match result {
                Ok(messages) => {
                    tracing::debug!(
                        chat_id,
                        message_count = messages.len(),
                        "background: older messages loaded"
                    );
                    ctx.shell
                        .state
                        .open_chat_mut()
                        .prepend_older_messages(messages);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: older messages load failed"
                    );
                }
            }
        }
        BackgroundTaskResult::MessageSent {
            chat_id,
            topic_id,
            message,
            result,
        } => match result {
            Ok(()) => {
                tracing::debug!(chat_id, "background: message sent successfully");
            }
            Err(error) => {
                tracing::warn!(
                    chat_id,
                    code = error.code,
                    "background: send message failed"
                );
                ctx.shell
                    .state
                    .open_chat_mut()
                    .remove_message(message.pending_id);
                // The reply target exists only in its own conversation; restored
                // elsewhere, a retry would reply to an unrelated message id.
                let still_open = ctx.shell.state.open_chat().chat_id() == Some(chat_id)
                    && ctx.shell.state.open_chat().topic_id() == topic_id;
                ctx.shell
                    .state
                    .message_input_mut()
                    .restore_failed_send(&message.text, message.reply_to.filter(|_| still_open));
            }
        },
        BackgroundTaskResult::MessageEdited {
            chat_id,
            message_id,
            original_text,
            result,
        } => match result {
            Ok(()) => {
                tracing::debug!(
                    chat_id,
                    message_id,
                    "background: message edited successfully"
                );
            }
            Err(error) => {
                tracing::warn!(
                    chat_id,
                    message_id,
                    code = error.code,
                    "background: edit message failed"
                );
                if ctx.shell.state.open_chat().chat_id() == Some(chat_id) {
                    ctx.shell
                        .state
                        .open_chat_mut()
                        .update_message_text(message_id, original_text);
                }
                ctx.shell.state.set_notification("Edit failed");
            }
        },
        BackgroundTaskResult::MessageSentRefreshCompleted {
            chat_id,
            topic_id,
            result,
        } => {
            ctx.open_chat_sync.messages_refresh_in_flight = false;

            // Same caching rule as MessagesLoaded — topic-scoped refresh results
            // must not leak into the per-chat cache.
            if topic_id.is_none() {
                if let Ok(ref messages) = result {
                    ctx.shell
                        .state
                        .message_cache_mut()
                        .put(chat_id, messages.clone(), true);
                }
            }

            if ctx.shell.state.open_chat().chat_id() != Some(chat_id)
                || ctx.shell.state.open_chat().topic_id() != topic_id
            {
                return;
            }

            match result {
                Ok(messages) => {
                    tracing::debug!(
                        chat_id,
                        message_count = messages.len(),
                        "background: messages refreshed after send"
                    );
                    ctx.shell.state.open_chat_mut().set_ready(messages);
                    ctx.shell.state.open_chat_mut().set_refreshing(false);
                    ctx.shell
                        .state
                        .open_chat_mut()
                        .set_message_source(MessageSource::Live);
                    chat_open::mark_open_chat_messages_as_read(&mut ctx.shell);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: message refresh after send failed"
                    );
                    ctx.shell.state.open_chat_mut().set_refreshing(false);
                }
            }
        }
        BackgroundTaskResult::VoiceSendFailed {
            chat_id,
            pending_id,
        } => {
            tracing::warn!(chat_id, "background: voice send failed, rolling back");
            if ctx.shell.state.open_chat().chat_id() == Some(chat_id) {
                ctx.shell.state.open_chat_mut().remove_message(pending_id);
            }
        }
        BackgroundTaskResult::ChatSubtitleLoaded { chat_id, result } => {
            if ctx.shell.state.open_chat().chat_id() == Some(chat_id) {
                match result {
                    Ok(subtitle) => {
                        tracing::debug!(chat_id, ?subtitle, "chat subtitle resolved");
                        ctx.shell.state.open_chat_mut().set_chat_subtitle(subtitle);
                    }
                    Err(e) => {
                        tracing::debug!(
                            chat_id,
                            error = ?e,
                            "chat subtitle resolution failed"
                        );
                    }
                }
            }
        }
        BackgroundTaskResult::ChatInfoLoaded { chat_id, result } => {
            popups::handle_chat_info_loaded(&mut ctx.shell, chat_id, result)
        }
        BackgroundTaskResult::MessagesPrefetched { chat_id, result } => {
            if ctx.open_chat_sync.prefetch_in_flight == Some(chat_id) {
                ctx.open_chat_sync.prefetch_in_flight = None;
            }

            if let Ok(messages) = result {
                if !messages.is_empty() {
                    tracing::debug!(
                        chat_id,
                        count = messages.len(),
                        "background: prefetched messages cached"
                    );
                    ctx.shell
                        .state
                        .message_cache_mut()
                        .put(chat_id, messages, true);
                }
            }

            // If the user opened this chat while the prefetch was in-flight
            // and the chat is still in Loading state, populate it from cache
            // (only if it meets the smart display threshold).
            let min_msgs = ctx.open_chat_sync.min_display_messages;
            if ctx.shell.state.open_chat().chat_id() == Some(chat_id)
                && ctx.shell.state.open_chat().ui_state() == OpenChatUiState::Loading
            {
                if let Some(cached) = ctx
                    .shell
                    .state
                    .message_cache_mut()
                    .get(chat_id)
                    .filter(|m| m.len() >= min_msgs)
                {
                    let msgs = cached.to_vec();
                    ctx.shell.state.open_chat_mut().set_ready(msgs);
                    ctx.shell.state.open_chat_mut().set_refreshing(true);
                    ctx.shell
                        .state
                        .open_chat_mut()
                        .set_message_source(MessageSource::Cache);
                    chat_open::mark_open_chat_messages_as_read(&mut ctx.shell);
                }
            }
        }
        BackgroundTaskResult::MessageInfoLoaded {
            chat_id,
            message_id,
            result,
        } => popups::handle_message_info_loaded(&mut ctx.shell, chat_id, message_id, result),
        BackgroundTaskResult::AvailableReactionsLoaded {
            chat_id,
            message_id,
            result,
        } => popups::handle_available_reactions_loaded(&mut ctx.shell, chat_id, message_id, result),
        BackgroundTaskResult::OpenFileFailed { stderr } => {
            let hint = if stderr.is_empty() {
                "Failed to open file. Configure [open] in ~/.config/rtg/config.toml".to_owned()
            } else {
                format!("Open failed: {stderr}")
            };
            tracing::warn!(stderr, "background: open file failed");
            ctx.shell.state.set_notification(&hint);
        }
        BackgroundTaskResult::FileSaved { file_id, file_name } => {
            ctx.downloads.pending_saves.remove(&file_id);
            tracing::info!(file_name, "background: file saved to downloads");
            ctx.shell
                .state
                .set_notification(format!("Saved: {file_name}"));
        }
        BackgroundTaskResult::FileSaveFailed { file_id, error } => {
            ctx.downloads.pending_saves.remove(&file_id);
            tracing::warn!(error, "background: file save failed");
            ctx.shell
                .state
                .set_notification(format!("Save failed: {error}"));
        }
        BackgroundTaskResult::UpdateAvailable(update) => {
            ctx.shell.state.set_available_update(update);
        }
    }
}
