use anyhow::Result;

use crate::{
    domain::{chat::ChatType, events::ConnectivityStatus, keymap::Action, shell_state::ActivePane},
    usecases::background::TaskDispatcher,
};

use super::{
    chat_list,
    chat_open::{self, OpenChatSync},
    chat_search, command_popup,
    file_updates::Downloads,
    forum, links, message_actions, message_input, popups, self_update, voice, OrchestratorCtx,
    ShellCtx,
};

/// Routes a key to the surface that captures all input while it is open:
/// chat search, a popup or the message input. Returns `false` when none is
/// open and the key goes through the keymap.
pub(super) fn dispatch_modal_key<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    key: &str,
) -> Result<bool> {
    let state = &ctx.shell.state;
    if state.chat_search().is_some() {
        chat_search::handle_chat_search_key(&mut ctx.shell, key);
    } else if state.command_popup().is_some() {
        command_popup::handle_command_popup_key(&mut ctx.shell, ctx.media, key);
    } else if state.chat_info_popup().is_some() {
        popups::handle_chat_info_popup_key(&mut ctx.shell, key);
    } else if state.message_info_popup().is_some() {
        popups::handle_message_info_popup_key(&mut ctx.shell, key);
    } else if state.reaction_picker().is_some() {
        popups::handle_reaction_picker_key(&mut ctx.shell, key);
    } else if state.chat_folder_picker().is_some() {
        chat_list::handle_chat_folder_picker_key(&mut ctx.shell, ctx.chat_list_loads, key);
    } else if state.link_picker().is_some() {
        links::handle_link_picker_key(&mut ctx.shell, ctx.open_chat_sync, ctx.opener, key)?;
    } else if state.help_visible() {
        popups::handle_help_key(&mut ctx.shell, key);
    } else if state.active_pane() == ActivePane::MessageInput {
        message_input::handle_message_input_key(&mut ctx.shell, key);
    } else {
        return Ok(false);
    }
    Ok(true)
}

pub(super) fn dispatch_chat_list_action<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    action: Action,
) -> Result<bool> {
    match action {
        Action::SelectNextChat => {
            ctx.shell.state.chat_list_mut().select_next();
            chat_open::maybe_prefetch_selected_chat(&mut ctx.shell, ctx.open_chat_sync);
            if ctx.shell.state.chat_list().needs_more_chats() && !ctx.chat_list_loads.in_flight {
                ctx.shell.state.chat_list_mut().request_more_chats();
                chat_list::dispatch_chat_list_refresh(&mut ctx.shell, ctx.chat_list_loads, false);
            }
        }
        Action::SelectPreviousChat => {
            ctx.shell.state.chat_list_mut().select_previous();
            chat_open::maybe_prefetch_selected_chat(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::SelectFirstChat => {
            ctx.shell.state.chat_list_mut().select_first();
            chat_open::maybe_prefetch_selected_chat(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::RefreshChatList => match ctx.shell.state.connectivity_status() {
            ConnectivityStatus::Connected | ConnectivityStatus::Updating => {
                ctx.chat_list_loads.user_requested_refresh = true;
                ctx.shell.state.set_notification("Refreshing chat list...");
                chat_list::dispatch_chat_list_refresh(&mut ctx.shell, ctx.chat_list_loads, true);
            }
            ConnectivityStatus::Connecting | ConnectivityStatus::Disconnected => {
                ctx.shell
                    .state
                    .set_notification("Cannot refresh: not connected");
            }
        },
        Action::MarkChatAsRead => {
            chat_list::mark_selected_chat_as_read(&mut ctx.shell, ctx.open_chat_sync)
        }
        Action::ShowChatInfo => popups::show_chat_info_popup(&mut ctx.shell),
        Action::SearchChats => ctx.shell.state.open_chat_search(),
        Action::ShowChatFolders => ctx.shell.state.open_chat_folder_picker(),
        Action::ToggleChatArchive => {
            chat_list::toggle_selected_chat_archive(&mut ctx.shell);
            chat_open::maybe_prefetch_selected_chat(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::OpenChat if ctx.shell.state.chat_list().selected_chat().is_some() => {
            // Pane management lives inside open_selected_chat — forum chats
            // need the ChatList pane to stay active, regular chats switch to
            // Messages. Setting it here first would briefly show the wrong
            // pane for forum chats.
            chat_open::open_selected_chat(&mut ctx.shell, ctx.open_chat_sync);
            return Ok(true);
        }
        Action::Quit => {
            chat_open::close_tdlib_chat(&mut ctx.shell, ctx.open_chat_sync);
            ctx.shell.state.stop();
        }
        Action::ShowHelp => {
            ctx.shell.state.show_help();
        }
        Action::InstallUpdate => self_update::start_self_update(&mut ctx.shell),
        _ => {}
    }
    Ok(false)
}

pub(super) fn dispatch_messages_action<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    action: Action,
) -> Result<()> {
    match action {
        Action::ScrollNextMessage => {
            let moved = ctx.shell.state.open_chat_mut().select_next();
            if !moved && !ctx.shell.state.open_chat().messages().is_empty() {
                ctx.shell.state.set_notification("End of messages");
            }
        }
        Action::ScrollPreviousMessage => {
            let moved = ctx.shell.state.open_chat_mut().select_previous();
            if !moved && !ctx.shell.state.open_chat().messages().is_empty() {
                ctx.shell.state.set_notification("Beginning of messages");
            }
            maybe_load_older_messages(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::ScrollToLastMessage => {
            ctx.shell.state.open_chat_mut().select_last();
        }
        Action::BackToChatList => {
            // A topic-open state implies an active forum_topic_list panel —
            // they're installed and dropped together. If a topic is open, `h`
            // returns to the topic list without closing the parent chat.
            if ctx.shell.state.open_chat().topic_id().is_some() {
                debug_assert!(
                    ctx.shell.state.forum_topic_list().is_some(),
                    "topic_id set without forum_topic_list panel — invariant broken"
                );
                forum::back_to_topic_list(&mut ctx.shell);
            } else {
                chat_open::close_tdlib_chat(&mut ctx.shell, ctx.open_chat_sync);
                ctx.shell.state.set_active_pane(ActivePane::ChatList);
            }
        }
        Action::EnterMessageInput if ctx.shell.state.open_chat().is_open() => {
            if ctx.shell.state.open_chat().chat_type() == ChatType::Channel {
                ctx.shell.state.set_notification("Channel is read-only");
            } else if ctx.shell.state.open_topic_is_closed() {
                ctx.shell.state.set_notification("Topic is closed");
            } else {
                ctx.shell.state.set_active_pane(ActivePane::MessageInput);
            }
        }
        Action::CopyMessage => {
            if let Some(msg) = ctx.shell.state.open_chat().selected_message() {
                let text = msg.display_content();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if clipboard.set_text(text).is_ok() {
                        ctx.shell.state.set_notification("Copied to clipboard");
                    }
                }
            }
        }
        Action::OpenLink => {
            links::open_message_link(&mut ctx.shell, ctx.open_chat_sync, ctx.opener)?
        }
        Action::OpenMessage if ctx.shell.state.open_chat().is_open() => {
            message_actions::open_selected_message(&mut ctx.shell, ctx.media);
        }
        Action::RecordVoice if ctx.shell.state.open_chat().is_open() => {
            voice::start_voice_recording(&mut ctx.shell, ctx.media);
        }
        Action::ShowMessageInfo => {
            popups::show_message_info_popup(&mut ctx.shell);
        }
        Action::AddReaction => {
            popups::show_reaction_picker(&mut ctx.shell);
        }
        Action::DownloadFile => {
            download_selected_message_file(&mut ctx.shell, ctx.downloads);
        }
        Action::SaveFile => {
            message_actions::save_selected_message_file(&mut ctx.shell, ctx.downloads);
        }
        Action::ReplyToMessage => {
            message_actions::reply_to_selected_message(&mut ctx.shell);
        }
        Action::EditMessage => {
            message_actions::edit_selected_message(&mut ctx.shell);
        }
        Action::DeleteMessage => {
            message_actions::delete_selected_message(&mut ctx.shell);
        }
        Action::Quit => {
            chat_open::close_tdlib_chat(&mut ctx.shell, ctx.open_chat_sync);
            ctx.shell.state.stop();
        }
        Action::ShowHelp => {
            ctx.shell.state.show_help();
        }
        Action::InstallUpdate => self_update::start_self_update(&mut ctx.shell),
        _ => {}
    }
    Ok(())
}

fn download_selected_message_file<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    downloads: &mut Downloads,
) {
    use crate::domain::message::DownloadStatus;

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(msg) = ctx.state.open_chat().selected_message() else {
        return;
    };

    let Some(ref fi) = msg.file_info else {
        return;
    };

    if fi.download_status != DownloadStatus::NotStarted {
        return;
    }

    if downloads.active_downloads.contains_key(&fi.file_id) {
        return;
    }

    let file_id = fi.file_id;
    let message_id = msg.id;

    tracing::info!(
        file_id,
        chat_id,
        message_id,
        "manual file download triggered"
    );
    downloads
        .active_downloads
        .insert(file_id, (chat_id, message_id));
    ctx.dispatcher.dispatch_download_file(file_id);
}

fn maybe_load_older_messages<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
) {
    if sync.older_messages_in_flight {
        return;
    }

    if !ctx.state.open_chat().needs_more_messages() {
        return;
    }

    let Some(chat_id) = ctx.state.open_chat().chat_id() else {
        return;
    };

    let Some(from_message_id) = ctx.state.open_chat().oldest_message_id() else {
        return;
    };

    sync.older_messages_in_flight = true;
    let topic_id = ctx.state.open_chat().topic_id();
    ctx.dispatcher
        .dispatch_load_older_messages(chat_id, topic_id, from_message_id);
}

pub(super) fn dispatch_forum_topic_list_action<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    action: Action,
) -> Result<()> {
    match action {
        Action::SelectNextTopic => {
            if let Some(list) = ctx.shell.state.forum_topic_list_mut() {
                list.select_next();
            }
        }
        Action::SelectPreviousTopic => {
            if let Some(list) = ctx.shell.state.forum_topic_list_mut() {
                list.select_previous();
            }
        }
        Action::SelectFirstTopic => {
            if let Some(list) = ctx.shell.state.forum_topic_list_mut() {
                list.select_first();
            }
        }
        Action::OpenForumTopic => {
            forum::open_selected_topic(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::BackFromForum => {
            forum::leave_forum(&mut ctx.shell, ctx.open_chat_sync);
        }
        Action::ReloadForumTopics => {
            forum::reload_topics(&mut ctx.shell);
        }
        Action::Quit => {
            chat_open::close_tdlib_chat(&mut ctx.shell, ctx.open_chat_sync);
            ctx.shell.state.stop();
        }
        Action::ShowHelp => {
            ctx.shell.state.show_help();
        }
        Action::InstallUpdate => self_update::start_self_update(&mut ctx.shell),
        _ => {}
    }
    Ok(())
}
