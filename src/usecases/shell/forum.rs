use crate::domain::{
    chat::ChatType,
    open_chat_state::{MessageSource, OpenChatUiState},
    shell_state::ActivePane,
};

use super::{
    chat_open::{self, OpenChatSync},
    ShellCtx,
};
use crate::usecases::background::TaskDispatcher;

/// Enters a forum chat: installs the topic-list panel in Loading and
/// dispatches the topic list load. Opens the parent chat in TDLib so that
/// `UpdateForumTopic*` events flow while the user browses topics.
///
/// Leaves the root `ChatListState` untouched — `leave_forum` just drops the
/// topic-list panel and the user returns to the same chat-list selection.
pub(super) fn enter_forum<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    chat_id: i64,
    chat_title: String,
) {
    tracing::debug!(chat_id, "entering forum chat");

    // Cancel any in-flight prefetch (we're not opening a regular chat).
    sync.prefetch_in_flight = None;

    chat_open::close_tdlib_chat_if_needed(ctx, sync, chat_id);
    ctx.dispatcher.dispatch_open_chat(chat_id);
    sync.tdlib_opened_chat_id = Some(chat_id);

    ctx.state.enter_forum(chat_id, chat_title);
    // Forum chats keep the left pane active — it now renders topics.
    ctx.state.set_active_pane(ActivePane::ChatList);
    ctx.dispatcher.dispatch_load_forum_topics(chat_id);
}

/// Drops the topic list panel and closes the parent chat in TDLib.
///
/// Used when the user presses `h` from the topic list (`Action::BackFromForum`)
/// — the root chat list reappears with selection preserved.
pub(super) fn leave_forum<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>, sync: &mut OpenChatSync) {
    tracing::debug!("leaving forum");
    chat_open::close_tdlib_chat(ctx, sync);

    // Reconcile the forum's root-list badge from the topic list we already hold,
    // so topics read while browsing are reflected immediately without a fetch.
    // Only when the list is Ready — Loading/Empty/Error would yield a bogus 0.
    use crate::domain::forum_topic_list_state::ForumTopicListUiState;
    let recomputed = ctx.state.forum_topic_list().and_then(|forum_list| {
        (forum_list.ui_state() == ForumTopicListUiState::Ready).then(|| {
            let unread = forum_list
                .topics()
                .iter()
                .filter(|t| t.unread_count > 0)
                .count() as u32;
            (forum_list.parent_chat_id(), unread)
        })
    });
    if let Some((chat_id, unread)) = recomputed {
        ctx.state
            .chat_list_mut()
            .set_forum_unread_topic_count(chat_id, unread);
    }

    ctx.state.leave_forum();
    ctx.state.set_active_pane(ActivePane::ChatList);
}

/// Opens the currently selected forum topic in the messages panel.
///
/// The parent forum chat is already open in TDLib (set up by `enter_forum`);
/// this only loads the topic-scoped history and switches focus.
pub(super) fn open_selected_topic<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
) {
    let Some(forum_list) = ctx.state.forum_topic_list() else {
        tracing::debug!("open_selected_topic called without an active forum");
        return;
    };
    let Some(topic) = forum_list.selected_topic() else {
        return;
    };

    let chat_id = forum_list.parent_chat_id();
    let topic_id = topic.topic_id;
    let topic_unread = topic.unread_count;
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

    // The forum's root-list badge shows the number of unread topics. Opening a
    // topic that had unread messages reads it, so optimistically drop the parent
    // forum's unread-topic count by one — TDLib pushes no chat-level read for
    // forums, and the next chat-list refresh reconciles from getForumTopics.
    if topic_unread > 0 {
        ctx.state.chat_list_mut().mark_forum_topic_read(chat_id);
    }

    open_topic(ctx, sync, chat_id, topic_id, title);
}

/// Shows the topic `topic_id` of the entered forum `chat_id` in the messages
/// panel under `title` and loads its history.
pub(super) fn open_topic<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    chat_id: i64,
    topic_id: i32,
    title: String,
) {
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

    sync.messages_refresh_in_flight = true;
    ctx.dispatcher
        .dispatch_load_messages(chat_id, Some(topic_id));
    ctx.state.set_active_pane(ActivePane::Messages);
}

/// Returns from a topic's messages view to the topic list panel.
///
/// Differs from [`leave_forum`] in that the parent chat stays open in
/// TDLib — we're still browsing the forum.
///
/// `open_chat` is intentionally not cleared so that the topic's messages
/// remain visible in the messages pane until the user opens a different
/// chat or topic — mirroring the behavior of `h` on a regular chat.
pub(super) fn back_to_topic_list<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    tracing::debug!("returning from topic to topic list");
    ctx.state.set_active_pane(ActivePane::ChatList);
}

/// Re-dispatches the topic list load and flips the panel back to Loading.
///
/// Bound to `R` in `KeyContext::ForumTopicList` so the user can recover from
/// a transient load failure without leaving the forum.
pub(super) fn reload_topics<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(list) = ctx.state.forum_topic_list_mut() else {
        return;
    };
    let chat_id = list.parent_chat_id();
    list.set_loading();
    ctx.dispatcher.dispatch_load_forum_topics(chat_id);
}

/// Titles the open topic after the loaded topic list. A topic opened from a
/// found message is shown under the forum's title until then: the search
/// result does not name the topic.
pub(super) fn title_open_topic<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(forum_list) = ctx.state.forum_topic_list() else {
        return;
    };
    if ctx.state.open_chat().chat_id() != Some(forum_list.parent_chat_id()) {
        return;
    }
    let title = ctx
        .state
        .open_chat()
        .topic_id()
        .and_then(|topic_id| forum_list.find_topic(topic_id))
        .map(|topic| format!("{} > {}", forum_list.parent_chat_title(), topic.name));
    if let Some(title) = title {
        ctx.state.open_chat_mut().set_chat_title(title);
    }
}
