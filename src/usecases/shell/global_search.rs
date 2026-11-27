use std::time::Instant;

use crate::{
    domain::{
        events::KeyInput,
        global_search_state::{ChatScope, FoundChat, FoundMessage, FoundMessagesPage, SearchHit},
    },
    usecases::background::{BackgroundError, TaskDispatcher},
};

use super::{
    chat_open::{self, OpenChatSync},
    forum, ShellCtx,
};

/// What Enter opens, taken out of the popup before it closes.
enum Target {
    Chat(FoundChat),
    Message(FoundMessage),
}

pub(super) fn handle_global_search_key<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    key: &KeyInput,
) {
    let Some(search) = ctx.state.global_search_mut() else {
        return;
    };
    match (key.key.as_str(), key.ctrl) {
        ("j", true) | ("down", _) => {
            search.select_next();
            if let (Some(offset), Some(query)) =
                (search.request_more_messages(), search.search_text())
            {
                ctx.dispatcher
                    .dispatch_search_messages(query.to_owned(), offset);
            }
        }
        ("k", true) | ("up", _) => search.select_previous(),
        ("backspace", _) => {
            search.delete_char_before(Instant::now());
            search_known_chats(ctx);
        }
        ("esc", _) => ctx.state.close_global_search(),
        ("enter", _) => {
            let target = search.selected().map(|hit| match hit {
                SearchHit::Chat(chat) => Target::Chat(chat.clone()),
                SearchHit::Message(message) => Target::Message(message.clone()),
            });
            if let Some(target) = target {
                ctx.state.close_global_search();
                open_target(ctx, sync, target);
            }
        }
        (ch, false) if ch.chars().count() == 1 => {
            if let Some(ch) = ch.chars().next() {
                search.insert_char(ch, Instant::now());
                search_known_chats(ctx);
            }
        }
        _ => {}
    }
}

/// Known chats are found without the server, so they are searched for on
/// every change of the query.
fn search_known_chats<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    if let Some(query) = ctx.state.global_search().and_then(|s| s.search_text()) {
        ctx.dispatcher
            .dispatch_search_chats(query.to_owned(), ChatScope::Known);
    }
}

/// Sends the requests that need the server once the user has stopped typing.
pub(super) fn send_due_network_search<D: TaskDispatcher>(ctx: &mut ShellCtx<'_, D>) {
    let Some(search) = ctx.state.global_search_mut() else {
        return;
    };
    if !search.take_due_network_search(Instant::now()) {
        return;
    }
    let Some(query) = search.search_text() else {
        return;
    };
    for scope in ChatScope::NETWORK {
        ctx.dispatcher
            .dispatch_search_chats(query.to_owned(), scope);
    }
    ctx.dispatcher
        .dispatch_search_messages(query.to_owned(), String::new());
}

/// A failed request counts as one that found no chats, so that the other
/// sections stay as they are; the failure is reported in the status bar.
pub(super) fn handle_chats_found<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    query: &str,
    scope: ChatScope,
    result: Result<Vec<FoundChat>, BackgroundError>,
) {
    let Some(search) = ctx.state.global_search_mut() else {
        return;
    };
    let failure = result.as_ref().err().map(|error| error.code);
    if !search.apply_chats(query, scope, result.unwrap_or_default()) {
        tracing::debug!(
            ?scope,
            "background: discarding stale result of a search for chats"
        );
        return;
    }
    if let Some(code) = failure {
        tracing::warn!(?scope, code, "background: search for chats failed");
        ctx.state.set_notification(match scope {
            ChatScope::Known | ChatScope::Server => "Could not search chats",
            ChatScope::Contacts => "Could not search contacts",
            ChatScope::Public => "Could not search public chats",
        });
    }
}

pub(super) fn handle_messages_found<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    query: &str,
    offset: &str,
    result: Result<FoundMessagesPage, BackgroundError>,
) {
    let Some(search) = ctx.state.global_search_mut() else {
        return;
    };
    let page = match result {
        Ok(page) => page,
        Err(error) => {
            if search.fail_messages(query, offset) {
                tracing::warn!(code = error.code, "background: message search failed");
                ctx.state.set_notification("Could not search messages");
            }
            return;
        }
    };
    let found_nothing = page.messages.is_empty();
    if !search.apply_messages(query, offset, page) {
        tracing::debug!("background: discarding stale message search result");
        return;
    }
    // TDLib may return a page without messages that is not the last one;
    // the user has no row to walk to in order to ask for the next.
    if found_nothing {
        if let Some(offset) = search.request_more_messages() {
            ctx.dispatcher
                .dispatch_search_messages(query.to_owned(), offset);
        }
    }
}

/// Opens a found chat the way a mention is opened: the chat may be in no
/// chat list, and when the active list holds it the selection moves to it.
/// A message opens its chat at the latest messages, or its forum topic.
fn open_target<D: TaskDispatcher>(
    ctx: &mut ShellCtx<'_, D>,
    sync: &mut OpenChatSync,
    target: Target,
) {
    let (chat, topic_id) = match target {
        Target::Chat(chat) => (chat, None),
        Target::Message(message) => (message.chat, message.topic_id),
    };
    ctx.state.chat_list_mut().select_chat(chat.chat_id);
    match topic_id.filter(|_| chat.is_forum) {
        Some(topic_id) => {
            forum::enter_forum(ctx, sync, chat.chat_id, chat.title.clone());
            // The topic's name comes with the topic list `enter_forum` asked for.
            forum::open_topic(ctx, sync, chat.chat_id, topic_id, chat.title);
        }
        None => chat_open::open_chat(
            ctx,
            sync,
            chat.chat_id,
            chat.title,
            chat.chat_type,
            chat.is_forum,
        ),
    }
}
