use crate::{
    domain::{
        events::BackgroundTaskResult,
        open_chat_state::{MessageSource, OpenChatUiState},
    },
    usecases::background::TaskDispatcher,
};

use super::{chat_open, OrchestratorCtx};

pub(super) fn handle_background_result<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    result: BackgroundTaskResult,
) {
    match result {
        BackgroundTaskResult::ChatListLoaded { result } => {
            *ctx.chat_list_in_flight = false;
            let was_pending = std::mem::take(ctx.chat_list_refresh_pending);
            let pending_force = std::mem::take(ctx.chat_list_pending_force);

            // When a re-dispatch is about to happen, defer the user notification
            // to the next completion — the pending result will be more up-to-date.
            let user_requested = if was_pending {
                false
            } else {
                std::mem::take(ctx.user_requested_chat_refresh)
            };

            match result {
                Ok(chats) => {
                    tracing::debug!(chat_count = chats.len(), "background: chat list loaded");
                    if user_requested {
                        ctx.state.set_notification("Chat list refreshed");
                    }
                    ctx.state.chat_list_mut().set_ready(chats);
                }
                Err(error) => {
                    tracing::warn!(code = error.code, "background: chat list load failed");
                    if user_requested {
                        ctx.state.set_notification("Chat list refresh failed");
                    }
                    ctx.state.chat_list_mut().set_error();
                }
            }

            if was_pending {
                tracing::debug!(
                    pending_force,
                    "re-dispatching chat list refresh (pending flag was set)"
                );
                super::chat_list::dispatch_chat_list_refresh(ctx, pending_force);
            }
        }
        BackgroundTaskResult::MessagesLoaded { chat_id, result } => {
            *ctx.messages_refresh_in_flight = false;

            // Always cache successful results, even if the user navigated away.
            if let Ok(ref messages) = result {
                ctx.state
                    .message_cache_mut()
                    .put(chat_id, messages.clone(), true);
            }

            if ctx.state.open_chat().chat_id() != Some(chat_id) {
                tracing::debug!(
                    chat_id,
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
                    if ctx.state.open_chat().ui_state() == OpenChatUiState::Ready {
                        ctx.state.open_chat_mut().update_messages(messages);
                    } else {
                        ctx.state.open_chat_mut().set_ready(messages);
                        ctx.state
                            .open_chat_mut()
                            .set_message_source(MessageSource::Live);
                    }
                    chat_open::mark_open_chat_messages_as_read(ctx);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: messages load failed"
                    );
                    ctx.state.open_chat_mut().set_error();
                }
            }
        }
        BackgroundTaskResult::MessageSent {
            chat_id,
            original_text,
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
                ctx.state.open_chat_mut().remove_pending_messages();
                ctx.state.message_input_mut().set_text(&original_text);
            }
        },
        BackgroundTaskResult::MessageSentRefreshCompleted { chat_id, result } => {
            *ctx.messages_refresh_in_flight = false;

            if let Ok(ref messages) = result {
                ctx.state
                    .message_cache_mut()
                    .put(chat_id, messages.clone(), true);
            }

            if ctx.state.open_chat().chat_id() != Some(chat_id) {
                return;
            }

            match result {
                Ok(messages) => {
                    tracing::debug!(
                        chat_id,
                        message_count = messages.len(),
                        "background: messages refreshed after send"
                    );
                    ctx.state.open_chat_mut().set_ready(messages);
                    ctx.state.open_chat_mut().set_refreshing(false);
                    ctx.state
                        .open_chat_mut()
                        .set_message_source(MessageSource::Live);
                    chat_open::mark_open_chat_messages_as_read(ctx);
                }
                Err(error) => {
                    tracing::warn!(
                        chat_id,
                        code = error.code,
                        "background: message refresh after send failed"
                    );
                    ctx.state.open_chat_mut().set_refreshing(false);
                }
            }
        }
        BackgroundTaskResult::VoiceSendFailed { chat_id } => {
            tracing::warn!(chat_id, "background: voice send failed, rolling back");
            if ctx.state.open_chat().chat_id() == Some(chat_id) {
                ctx.state.open_chat_mut().remove_pending_messages();
            }
        }
        BackgroundTaskResult::ChatSubtitleLoaded { chat_id, result } => {
            if ctx.state.open_chat().chat_id() == Some(chat_id) {
                match result {
                    Ok(subtitle) => {
                        tracing::debug!(chat_id, ?subtitle, "chat subtitle resolved");
                        ctx.state.open_chat_mut().set_chat_subtitle(subtitle);
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
            use crate::domain::chat_info_state::ChatInfoPopupState;

            let popup_chat_id = ctx.state.chat_info_popup().and_then(|p| p.chat_id());
            if popup_chat_id == Some(chat_id) {
                match result {
                    Ok(info) => {
                        tracing::debug!(chat_id, "chat info resolved for popup");
                        ctx.state
                            .set_chat_info_loaded(ChatInfoPopupState::Loaded(info));
                    }
                    Err(e) => {
                        tracing::debug!(chat_id, code = e.code, "chat info resolution failed");
                        let title = ctx
                            .state
                            .chat_info_popup()
                            .map(|p| p.title().to_owned())
                            .unwrap_or_default();
                        ctx.state
                            .set_chat_info_loaded(ChatInfoPopupState::Error { title });
                    }
                }
            }
        }
        BackgroundTaskResult::MessagesPrefetched { chat_id, result } => {
            if *ctx.prefetch_in_flight == Some(chat_id) {
                *ctx.prefetch_in_flight = None;
            }

            if let Ok(messages) = result {
                if !messages.is_empty() {
                    tracing::debug!(
                        chat_id,
                        count = messages.len(),
                        "background: prefetched messages cached"
                    );
                    ctx.state.message_cache_mut().put(chat_id, messages, true);
                }
            }

            // If the user opened this chat while the prefetch was in-flight
            // and the chat is still in Loading state, populate it from cache
            // (only if it meets the smart display threshold).
            let min_msgs = ctx.min_display_messages;
            if ctx.state.open_chat().chat_id() == Some(chat_id)
                && ctx.state.open_chat().ui_state() == OpenChatUiState::Loading
            {
                if let Some(cached) = ctx
                    .state
                    .message_cache_mut()
                    .get(chat_id)
                    .filter(|m| m.len() >= min_msgs)
                {
                    let msgs = cached.to_vec();
                    ctx.state.open_chat_mut().set_ready(msgs);
                    ctx.state.open_chat_mut().set_refreshing(true);
                    ctx.state
                        .open_chat_mut()
                        .set_message_source(MessageSource::Cache);
                    chat_open::mark_open_chat_messages_as_read(ctx);
                }
            }
        }
        BackgroundTaskResult::OpenFileFailed { stderr } => {
            let hint = if stderr.is_empty() {
                "Failed to open file. Configure [open] in ~/.config/rtg/config.toml".to_owned()
            } else {
                format!("Open failed: {stderr}")
            };
            tracing::warn!(stderr, "background: open file failed");
            ctx.state.set_notification(&hint);
        }
    }
}
