use crate::domain::{
    chat::ChatType,
    open_chat_state::{MessageSource, OpenChatUiState},
    shell_state::ActivePane,
};

use super::{chat_open, OrchestratorCtx};
use crate::usecases::background::TaskDispatcher;

/// Enters a forum chat: installs the topic-list panel in Loading and
/// dispatches the topic list load. Opens the parent chat in TDLib so that
/// `UpdateForumTopic*` events flow while the user browses topics.
///
/// Leaves the root `ChatListState` untouched — `leave_forum` just drops the
/// topic-list panel and the user returns to the same chat-list selection.
pub(super) fn enter_forum<D: TaskDispatcher>(
    ctx: &mut OrchestratorCtx<'_, D>,
    chat_id: i64,
    chat_title: String,
) {
    tracing::debug!(chat_id, "entering forum chat");

    // Cancel any in-flight prefetch (we're not opening a regular chat).
    *ctx.prefetch_in_flight = None;

    chat_open::close_tdlib_chat_if_needed(ctx, chat_id);
    ctx.dispatcher.dispatch_open_chat(chat_id);
    *ctx.tdlib_opened_chat_id = Some(chat_id);

    ctx.state.enter_forum(chat_id, chat_title);
    // dispatch_chat_list_action sets ActivePane::Messages before calling us;
    // for forums we keep the left panel focused — it now renders topics.
    ctx.state.set_active_pane(ActivePane::ChatList);
    ctx.dispatcher.dispatch_load_forum_topics(chat_id);
}

/// Drops the topic list panel and closes the parent chat in TDLib.
///
/// Used when the user presses `h` from the topic list (`Action::BackFromForum`)
/// — the root chat list reappears with selection preserved.
pub(super) fn leave_forum<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    tracing::debug!("leaving forum");
    chat_open::close_tdlib_chat(ctx);
    ctx.state.leave_forum();
    ctx.state.set_active_pane(ActivePane::ChatList);
}

/// Opens the currently selected forum topic in the messages panel.
///
/// The parent forum chat is already open in TDLib (set up by `enter_forum`);
/// this only loads the topic-scoped history and switches focus.
pub(super) fn open_selected_topic<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    let Some(forum_list) = ctx.state.forum_topic_list() else {
        tracing::debug!("open_selected_topic called without an active forum");
        return;
    };
    let Some(topic) = forum_list.selected_topic() else {
        return;
    };

    let chat_id = forum_list.parent_chat_id();
    let topic_id = topic.topic_id;
    let title = format!("{} > {}", forum_list.parent_chat_title(), topic.name);

    tracing::debug!(chat_id, topic_id, %title, "opening forum topic");

    // If the same topic is already open and Ready, just switch focus.
    if ctx.state.open_chat().chat_id() == Some(chat_id)
        && ctx.state.open_chat().topic_id() == Some(topic_id)
        && ctx.state.open_chat().ui_state() == OpenChatUiState::Ready
    {
        ctx.state.set_active_pane(ActivePane::Messages);
        return;
    }

    ctx.state.open_chat_mut().set_loading_with_topic(
        chat_id,
        Some(topic_id),
        title,
        ChatType::Group,
    );
    // Topic history bypasses the per-chat MessageCache (cache is keyed by
    // chat_id only); show Loading until the live load returns.
    ctx.state
        .open_chat_mut()
        .set_message_source(MessageSource::None);

    *ctx.messages_refresh_in_flight = true;
    ctx.dispatcher
        .dispatch_load_messages(chat_id, Some(topic_id));
    ctx.state.set_active_pane(ActivePane::Messages);
}

/// Returns from a topic's messages view to the topic list panel.
///
/// Differs from [`leave_forum`] in that the parent chat stays open in
/// TDLib — we're still browsing the forum.
pub(super) fn back_to_topic_list<D: TaskDispatcher>(ctx: &mut OrchestratorCtx<'_, D>) {
    tracing::debug!("returning from topic to topic list");
    ctx.state.open_chat_mut().clear();
    ctx.state.set_active_pane(ActivePane::ChatList);
}
