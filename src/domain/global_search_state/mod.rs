use std::time::{Duration, Instant};

use super::chat::ChatType;

/// The most chats the "Chats" section shows.
pub const CHATS_LIMIT: usize = 10;
/// The most chats the "Global" section shows.
pub const PUBLIC_CHATS_LIMIT: usize = 5;
/// How many messages one page of the "Messages" section asks for.
pub const MESSAGES_PAGE_SIZE: usize = 20;
/// How long the user must stop typing before the requests that need the
/// server are sent.
const NETWORK_SEARCH_DELAY: Duration = Duration::from_millis(300);

/// A chat found by global search, with what is needed to show and open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundChat {
    pub chat_id: i64,
    pub title: String,
    /// Without the leading `@`.
    pub username: Option<String>,
    pub chat_type: ChatType,
    pub is_forum: bool,
}

/// A message found by global search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundMessage {
    pub chat: FoundChat,
    pub message_id: i64,
    /// The forum topic the message lies in; `None` outside forums.
    pub topic_id: Option<i32>,
    pub sender_name: String,
    pub timestamp_ms: i64,
    /// The same one-line preview the chat list shows for a last message.
    pub preview: String,
}

/// One page of found messages; `next_offset` is `None` on the last page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoundMessagesPage {
    pub messages: Vec<FoundMessage>,
    pub next_offset: Option<String>,
}

/// Where chats are looked for. Each scope is one request, and the requests
/// run side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatScope {
    /// Chats this device already knows; answered without the server.
    Known,
    /// The user's chats, as the server knows them.
    Server,
    Contacts,
    /// Public chats the user is not in.
    Public,
}

impl ChatScope {
    pub const ALL: [Self; 4] = [Self::Known, Self::Server, Self::Contacts, Self::Public];
    /// The scopes whose requests wait for a pause in typing.
    pub const NETWORK: [Self; 3] = [Self::Server, Self::Contacts, Self::Public];
}

/// A row of the results, whatever its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchHit<'a> {
    Chat(&'a FoundChat),
    Message(&'a FoundMessage),
}

/// What makes a row the same row after more results arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitKey {
    /// A chat keeps its key when it moves between "Chats" and "Global".
    Chat(i64),
    Message(i64, i64),
}

/// The global search popup: the query and what Telegram has found for it so
/// far. Every result shown belongs to the current query: editing the query
/// drops the results, and an answer to an earlier query is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalSearchState {
    query: String,
    known_chats: Vec<FoundChat>,
    server_chats: Vec<FoundChat>,
    contacts: Vec<FoundChat>,
    public_chats: Vec<FoundChat>,
    messages: Vec<FoundMessage>,
    /// Offset of the messages page after the loaded ones.
    next_offset: Option<String>,
    /// Chat scopes that have not answered the current query yet.
    awaited_chats: Vec<ChatScope>,
    /// Offset of the messages page that has not arrived yet.
    awaited_messages: Option<String>,
    /// When the requests that need the server are to be sent.
    network_search_due: Option<Instant>,
    /// Index into `hits()`.
    selected: usize,
}

impl GlobalSearchState {
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The text to search for; `None` while the query is blank.
    pub fn search_text(&self) -> Option<&str> {
        Some(self.query.trim()).filter(|text| !text.is_empty())
    }

    pub fn insert_char(&mut self, ch: char, now: Instant) {
        self.query.push(ch);
        self.restart(now);
    }

    pub fn delete_char_before(&mut self, now: Instant) {
        if self.query.pop().is_some() {
            self.restart(now);
        }
    }

    /// Whether the pause in typing is over. Says so once per query: the
    /// caller sends the requests that need the server.
    pub fn take_due_network_search(&mut self, now: Instant) -> bool {
        if self.network_search_due.is_some_and(|due| due <= now) {
            self.network_search_due = None;
            return true;
        }
        false
    }

    /// Takes the chats `scope` found for `query`. Returns `false`, leaving
    /// the results alone, when the query has changed since or the scope has
    /// already answered it.
    pub fn apply_chats(&mut self, query: &str, scope: ChatScope, chats: Vec<FoundChat>) -> bool {
        let awaited = self.awaited_chats.iter().position(|&s| s == scope);
        let (Some(awaited), true) = (awaited, self.search_text() == Some(query)) else {
            return false;
        };
        self.awaited_chats.swap_remove(awaited);

        let followed = self.followed_key();
        match scope {
            ChatScope::Known => self.known_chats = chats,
            ChatScope::Server => self.server_chats = chats,
            ChatScope::Contacts => self.contacts = chats,
            ChatScope::Public => self.public_chats = chats,
        }
        self.restore_selection(followed);
        true
    }

    /// Takes the page of messages found for `query` from `offset`. Returns
    /// `false`, leaving the results alone, unless that page is the awaited
    /// one.
    pub fn apply_messages(&mut self, query: &str, offset: &str, page: FoundMessagesPage) -> bool {
        if self.search_text() != Some(query) || self.awaited_messages.as_deref() != Some(offset) {
            return false;
        }
        self.awaited_messages = None;
        self.messages.extend(page.messages);
        self.next_offset = page.next_offset;
        true
    }

    /// Records that the page of messages for `query` from `offset` could not
    /// be loaded: the same page is asked for again at the next request for
    /// more. Returns `false` unless that page is the awaited one.
    pub fn fail_messages(&mut self, query: &str, offset: &str) -> bool {
        if self.search_text() != Some(query) || self.awaited_messages.as_deref() != Some(offset) {
            return false;
        }
        self.next_offset = self.awaited_messages.take();
        true
    }

    /// Whether some request for the current query has not answered yet.
    pub fn is_searching(&self) -> bool {
        !self.awaited_chats.is_empty() || self.awaited_messages.is_some()
    }

    /// The "Chats" section: known chats, then the server's, then contacts,
    /// each chat once.
    pub fn chats(&self) -> Vec<&FoundChat> {
        let mut chats: Vec<&FoundChat> = Vec::new();
        for chat in self
            .known_chats
            .iter()
            .chain(&self.server_chats)
            .chain(&self.contacts)
        {
            if chats.len() == CHATS_LIMIT {
                break;
            }
            if chats.iter().all(|shown| shown.chat_id != chat.chat_id) {
                chats.push(chat);
            }
        }
        chats
    }

    /// The "Global" section: public chats that "Chats" does not show.
    pub fn public_chats(&self) -> Vec<&FoundChat> {
        let chats = self.chats();
        self.public_chats
            .iter()
            .filter(|chat| chats.iter().all(|shown| shown.chat_id != chat.chat_id))
            .take(PUBLIC_CHATS_LIMIT)
            .collect()
    }

    /// The "Messages" section, newest first.
    pub fn messages(&self) -> &[FoundMessage] {
        &self.messages
    }

    /// Every row in the order shown: "Chats", "Global", "Messages".
    pub fn hits(&self) -> Vec<SearchHit<'_>> {
        self.chats()
            .into_iter()
            .chain(self.public_chats())
            .map(SearchHit::Chat)
            .chain(self.messages.iter().map(SearchHit::Message))
            .collect()
    }

    pub fn selected_index(&self) -> Option<usize> {
        (!self.hits().is_empty()).then_some(self.selected)
    }

    pub fn selected(&self) -> Option<SearchHit<'_>> {
        self.hits().get(self.selected).copied()
    }

    pub fn select_next(&mut self) {
        let last = self.hits().len().saturating_sub(1);
        self.selected = (self.selected + 1).min(last);
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// The offset of the next messages page, when it is time to load it:
    /// the selection is on the last row, more messages exist and no page is
    /// on its way. The page then counts as awaited.
    pub fn request_more_messages(&mut self) -> Option<String> {
        if self.awaited_messages.is_some() || self.selected + 1 < self.hits().len() {
            return None;
        }
        let offset = self.next_offset.take()?;
        self.awaited_messages = Some(offset.clone());
        Some(offset)
    }

    #[cfg(test)]
    pub fn end_typing_pause(&mut self) {
        self.network_search_due = self.network_search_due.map(|_| Instant::now());
    }

    /// Starts over for the edited query.
    fn restart(&mut self, now: Instant) {
        let searching = self.search_text().is_some();
        *self = Self {
            query: std::mem::take(&mut self.query),
            awaited_chats: if searching {
                ChatScope::ALL.to_vec()
            } else {
                Vec::new()
            },
            awaited_messages: searching.then(String::new),
            network_search_due: searching.then(|| now + NETWORK_SEARCH_DELAY),
            ..Self::default()
        };
    }

    /// The selected row, once the user has moved the selection off the
    /// first one: until then the selection stays on top.
    fn followed_key(&self) -> Option<HitKey> {
        (self.selected > 0)
            .then(|| self.keys().get(self.selected).copied())
            .flatten()
    }

    fn restore_selection(&mut self, followed: Option<HitKey>) {
        let keys = self.keys();
        self.selected = followed
            .and_then(|key| keys.iter().position(|&k| k == key))
            .unwrap_or(self.selected)
            .min(keys.len().saturating_sub(1));
    }

    fn keys(&self) -> Vec<HitKey> {
        self.chats()
            .into_iter()
            .chain(self.public_chats())
            .map(|chat| HitKey::Chat(chat.chat_id))
            .chain(
                self.messages
                    .iter()
                    .map(|m| HitKey::Message(m.chat.chat_id, m.message_id)),
            )
            .collect()
    }
}

#[cfg(test)]
mod tests;
