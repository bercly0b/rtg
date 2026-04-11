mod chat_info;
mod chat_list;
mod chat_open;
mod chat_updates;
mod help_popup;
mod lifecycle;
mod message_actions;
mod message_cache;
mod message_input;
mod playback;
mod voice;

use std::cell::RefCell;

use super::*;
use crate::{
    domain::{
        chat::ChatSummary,
        chat_list_state::ChatListUiState,
        events::{AppEvent, BackgroundError, BackgroundTaskResult, ConnectivityStatus, KeyInput},
        message::Message,
        open_chat_state::OpenChatUiState,
        shell_state::ShellState,
    },
    infra::{contracts::ExternalOpener, stubs::StubStorageAdapter},
};

// ── Recording opener for tests ──

#[derive(Debug, Default)]
struct RecordingOpener {
    opened: RefCell<Vec<String>>,
}

impl ExternalOpener for RecordingOpener {
    fn open(&self, target: &str) -> anyhow::Result<()> {
        self.opened.borrow_mut().push(target.to_owned());
        Ok(())
    }
}

impl RecordingOpener {
    fn opened_urls(&self) -> Vec<String> {
        self.opened.borrow().clone()
    }
}

// ── Domain helpers ──

fn chat(chat_id: i64, title: &str) -> ChatSummary {
    use crate::domain::chat::{ChatType, OutgoingReadStatus};
    ChatSummary {
        chat_id,
        title: title.to_owned(),
        unread_count: 0,
        last_message_preview: None,
        last_message_unix_ms: None,
        is_pinned: false,
        chat_type: ChatType::Private,
        last_message_sender: None,
        is_online: None,
        is_bot: false,
        outgoing_status: OutgoingReadStatus::default(),
        last_message_id: None,
        unread_reaction_count: 0,
    }
}

fn message(id: i64, text: &str) -> Message {
    Message {
        id,
        sender_name: "User".to_owned(),
        text: text.to_owned(),
        timestamp_ms: 1000,
        is_outgoing: false,
        media: crate::domain::message::MessageMedia::None,
        status: crate::domain::message::MessageStatus::Delivered,
        file_info: None,
        call_info: None,
        reply_to: None,
        reaction_count: 0,
        links: Vec::new(),
        is_edited: false,
    }
}

// ── Recording task dispatcher for tests ──

/// Records what the orchestrator dispatched and allows inspection.
struct RecordingDispatcher {
    dispatched_chat_list_count: RefCell<usize>,
    dispatched_chat_list_force: RefCell<Vec<bool>>,
    dispatched_messages: RefCell<Vec<i64>>,
    dispatched_sends: RefCell<Vec<(i64, String, Option<i64>)>>,
    dispatched_open_chats: RefCell<Vec<i64>>,
    dispatched_close_chats: RefCell<Vec<i64>>,
    dispatched_mark_as_read: RefCell<Vec<(i64, Vec<i64>)>>,
    dispatched_mark_chat_as_read: RefCell<Vec<(i64, i64)>>,
    dispatched_prefetches: RefCell<Vec<i64>>,
    dispatched_deletes: RefCell<Vec<(i64, i64)>>,
    dispatched_voice_sends: RefCell<Vec<(i64, String)>>,
    dispatched_subtitles: RefCell<Vec<ChatSubtitleQuery>>,
}

impl RecordingDispatcher {
    fn new() -> Self {
        Self {
            dispatched_chat_list_count: RefCell::new(0),
            dispatched_chat_list_force: RefCell::new(Vec::new()),
            dispatched_messages: RefCell::new(Vec::new()),
            dispatched_sends: RefCell::new(Vec::new()),
            dispatched_open_chats: RefCell::new(Vec::new()),
            dispatched_close_chats: RefCell::new(Vec::new()),
            dispatched_mark_as_read: RefCell::new(Vec::new()),
            dispatched_mark_chat_as_read: RefCell::new(Vec::new()),
            dispatched_prefetches: RefCell::new(Vec::new()),
            dispatched_deletes: RefCell::new(Vec::new()),
            dispatched_voice_sends: RefCell::new(Vec::new()),
            dispatched_subtitles: RefCell::new(Vec::new()),
        }
    }

    fn chat_list_dispatch_count(&self) -> usize {
        *self.dispatched_chat_list_count.borrow()
    }

    fn last_chat_list_force(&self) -> Option<bool> {
        self.dispatched_chat_list_force.borrow().last().copied()
    }

    fn messages_dispatch_count(&self) -> usize {
        self.dispatched_messages.borrow().len()
    }

    fn send_dispatch_count(&self) -> usize {
        self.dispatched_sends.borrow().len()
    }

    fn last_send(&self) -> Option<(i64, String, Option<i64>)> {
        self.dispatched_sends.borrow().last().cloned()
    }

    fn open_chat_dispatch_count(&self) -> usize {
        self.dispatched_open_chats.borrow().len()
    }

    fn close_chat_dispatch_count(&self) -> usize {
        self.dispatched_close_chats.borrow().len()
    }

    fn mark_as_read_dispatch_count(&self) -> usize {
        self.dispatched_mark_as_read.borrow().len()
    }

    fn last_mark_as_read(&self) -> Option<(i64, Vec<i64>)> {
        self.dispatched_mark_as_read.borrow().last().cloned()
    }

    fn mark_chat_as_read_dispatch_count(&self) -> usize {
        self.dispatched_mark_chat_as_read.borrow().len()
    }

    fn last_mark_chat_as_read(&self) -> Option<(i64, i64)> {
        self.dispatched_mark_chat_as_read.borrow().last().cloned()
    }

    fn prefetch_dispatch_count(&self) -> usize {
        self.dispatched_prefetches.borrow().len()
    }

    fn last_prefetch_chat_id(&self) -> Option<i64> {
        self.dispatched_prefetches.borrow().last().copied()
    }

    fn delete_dispatch_count(&self) -> usize {
        self.dispatched_deletes.borrow().len()
    }

    fn last_delete(&self) -> Option<(i64, i64)> {
        self.dispatched_deletes.borrow().last().copied()
    }

    fn voice_send_dispatch_count(&self) -> usize {
        self.dispatched_voice_sends.borrow().len()
    }

    fn last_voice_send(&self) -> Option<(i64, String)> {
        self.dispatched_voice_sends.borrow().last().cloned()
    }

    fn subtitle_dispatch_count(&self) -> usize {
        self.dispatched_subtitles.borrow().len()
    }

    fn last_subtitle_query(&self) -> Option<ChatSubtitleQuery> {
        self.dispatched_subtitles.borrow().last().cloned()
    }
}

impl TaskDispatcher for RecordingDispatcher {
    fn dispatch_chat_list(&self, force: bool) {
        *self.dispatched_chat_list_count.borrow_mut() += 1;
        self.dispatched_chat_list_force.borrow_mut().push(force);
    }

    fn dispatch_load_messages(&self, chat_id: i64) {
        self.dispatched_messages.borrow_mut().push(chat_id);
    }

    fn dispatch_send_message(&self, chat_id: i64, text: String, reply_to_message_id: Option<i64>) {
        self.dispatched_sends
            .borrow_mut()
            .push((chat_id, text, reply_to_message_id));
    }

    fn dispatch_open_chat(&self, chat_id: i64) {
        self.dispatched_open_chats.borrow_mut().push(chat_id);
    }

    fn dispatch_close_chat(&self, chat_id: i64) {
        self.dispatched_close_chats.borrow_mut().push(chat_id);
    }

    fn dispatch_mark_as_read(&self, chat_id: i64, message_ids: Vec<i64>) {
        self.dispatched_mark_as_read
            .borrow_mut()
            .push((chat_id, message_ids));
    }

    fn dispatch_mark_chat_as_read(&self, chat_id: i64, last_message_id: i64) {
        self.dispatched_mark_chat_as_read
            .borrow_mut()
            .push((chat_id, last_message_id));
    }

    fn dispatch_prefetch_messages(&self, chat_id: i64) {
        self.dispatched_prefetches.borrow_mut().push(chat_id);
    }

    fn dispatch_delete_message(&self, chat_id: i64, message_id: i64) {
        self.dispatched_deletes
            .borrow_mut()
            .push((chat_id, message_id));
    }

    fn dispatch_chat_subtitle(&self, query: ChatSubtitleQuery) {
        self.dispatched_subtitles.borrow_mut().push(query);
    }

    fn dispatch_send_voice(&self, chat_id: i64, file_path: String) {
        self.dispatched_voice_sends
            .borrow_mut()
            .push((chat_id, file_path));
    }

    fn dispatch_download_file(&self, _file_id: i32) {
        // Recording: no-op for now
    }

    fn dispatch_chat_info(&self, _query: crate::usecases::chat_subtitle::ChatInfoQuery) {
        // Recording: no-op for now
    }

    fn dispatch_open_file(&self, _cmd_template: String, _file_path: String) {
        // Recording: no-op for now
    }

    fn dispatch_save_file(&self, _file_id: i32, _local_path: String, _file_name: Option<String>) {
        // Recording: no-op for now
    }
}

// ── Test orchestrator factory ──

type TestOrchestrator =
    DefaultShellOrchestrator<StubStorageAdapter, RecordingOpener, RecordingDispatcher>;

fn make_orchestrator() -> TestOrchestrator {
    let mut o = DefaultShellOrchestrator::new(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
    );
    // Use min threshold of 1 for existing tests — any cached message triggers display.
    // Tests for the threshold itself use make_orchestrator_with_threshold().
    o.min_display_messages = 1;
    o
}

/// Helper: pre-populate the chat list as if a background load completed.
fn inject_chat_list(orchestrator: &mut TestOrchestrator, chats: Vec<ChatSummary>) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded { result: Ok(chats) },
        ))
        .unwrap();
}

/// Helper: inject messages as if a background load completed for given chat.
fn inject_messages(orchestrator: &mut TestOrchestrator, chat_id: i64, messages: Vec<Message>) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id,
                result: Ok(messages),
            },
        ))
        .unwrap();
}

/// Helper: set up orchestrator with a loaded chat list (skip the dispatch+result dance).
fn orchestrator_with_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
    let mut o = make_orchestrator();
    inject_chat_list(&mut o, chats);
    o
}

/// Helper: set up orchestrator with a loaded chat list and an opened chat.
fn orchestrator_with_open_chat(
    chats: Vec<ChatSummary>,
    chat_id: i64,
    messages: Vec<Message>,
) -> TestOrchestrator {
    let mut o = orchestrator_with_chats(chats);
    // Press enter to open the first chat (dispatches load_messages)
    o.handle_event(AppEvent::InputKey(KeyInput::new("enter", false)))
        .unwrap();
    // Inject the messages result
    inject_messages(&mut o, chat_id, messages);
    o
}

// ── Stub cache source for tests ──

struct StubCacheSource {
    messages: std::sync::Mutex<std::collections::HashMap<i64, Vec<Message>>>,
}

impl StubCacheSource {
    fn empty() -> Self {
        Self {
            messages: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn with_messages(entries: Vec<(i64, Vec<Message>)>) -> Self {
        let map = entries.into_iter().collect();
        Self {
            messages: std::sync::Mutex::new(map),
        }
    }
}

impl CachedMessagesSource for StubCacheSource {
    fn list_cached_messages(
        &self,
        chat_id: i64,
        _limit: usize,
    ) -> Result<Vec<Message>, crate::usecases::load_messages::MessagesSourceError> {
        let map = self.messages.lock().unwrap();
        Ok(map.get(&chat_id).cloned().unwrap_or_default())
    }
}

// ── Orchestrator factory with pre-populated cache ──

fn make_orchestrator_with_cached_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
    let state = ShellState::with_initial_chat_list(chats);
    DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        state,
        None,
        1, // No threshold in most tests — any cached message triggers instant display
        crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        std::collections::HashMap::new(),
        10_000_000,
    )
}

fn make_orchestrator_with_cache(
    chats: Vec<ChatSummary>,
    cache: StubCacheSource,
) -> TestOrchestrator {
    let state = ShellState::with_initial_chat_list(chats);
    DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        state,
        Some(Arc::new(cache)),
        1, // No threshold in most tests — any cached message triggers instant display
        crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        std::collections::HashMap::new(),
        10_000_000,
    )
}

fn make_orchestrator_with_threshold(
    chats: Vec<ChatSummary>,
    min_display_messages: usize,
) -> TestOrchestrator {
    let state = ShellState::with_initial_chat_list(chats);
    DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        state,
        None,
        min_display_messages,
        crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        std::collections::HashMap::new(),
        10_000_000,
    )
}

fn make_orchestrator_with_cache_and_threshold(
    chats: Vec<ChatSummary>,
    cache: StubCacheSource,
    min_display_messages: usize,
) -> TestOrchestrator {
    let state = ShellState::with_initial_chat_list(chats);
    DefaultShellOrchestrator::new_with_initial_state(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
        state,
        Some(Arc::new(cache)),
        min_display_messages,
        crate::usecases::voice_recording::DEFAULT_RECORD_CMD.to_owned(),
        std::collections::HashMap::new(),
        10_000_000,
    )
}

// ── Tests ──
