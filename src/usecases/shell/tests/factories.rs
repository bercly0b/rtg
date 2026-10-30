use super::*;

// ── Test orchestrator factory ──

pub(super) type TestOrchestrator =
    DefaultShellOrchestrator<StubStorageAdapter, RecordingOpener, RecordingDispatcher>;

pub(super) fn make_orchestrator() -> TestOrchestrator {
    let mut o = DefaultShellOrchestrator::new(
        StubStorageAdapter::default(),
        RecordingOpener::default(),
        RecordingDispatcher::new(),
    );
    // Use min threshold of 1 for existing tests — any cached message triggers display.
    // Tests for the threshold itself use make_orchestrator_with_threshold().
    o.min_display_messages = 1;
    // Default to a connected session so tests exercising server-bound
    // actions (refresh, send) don't have to inject this themselves. Tests
    // that need a different connectivity state inject it explicitly.
    o.handle_event(AppEvent::ConnectivityChanged(ConnectivityStatus::Connected))
        .expect("connecting test orchestrator should not fail");
    o
}

/// Helper: pre-populate the chat list as if a background load completed.
pub(super) fn inject_chat_list(orchestrator: &mut TestOrchestrator, chats: Vec<ChatSummary>) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ChatListLoaded {
                result: Ok(chats),
                all_loaded: false,
            },
        ))
        .unwrap();
}

/// Helper: inject forum topic list as if a background load completed.
#[allow(dead_code)]
pub(super) fn inject_forum_topics(
    orchestrator: &mut TestOrchestrator,
    chat_id: i64,
    topics: Vec<crate::domain::forum_topic::ForumTopicSummary>,
) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ForumTopicsLoaded {
                chat_id,
                result: Ok(topics),
            },
        ))
        .unwrap();
}

/// Helper: inject a failed forum topics load (transient error).
#[allow(dead_code)]
pub(super) fn inject_forum_topics_error(orchestrator: &mut TestOrchestrator, chat_id: i64) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::ForumTopicsLoaded {
                chat_id,
                result: Err(crate::domain::events::BackgroundError::new(
                    "FORUM_TOPICS_UNAVAILABLE",
                )),
            },
        ))
        .unwrap();
}

/// Helper: inject messages as if a background load completed for given chat.
///
/// Defaults `topic_id` to the currently open topic for this chat (if any) so
/// that tests which open a forum topic and then inject can stay terse. Use
/// [`inject_topic_messages`] when you need to target a specific topic
/// explicitly (e.g. to simulate a stale result from another topic).
pub(super) fn inject_messages(
    orchestrator: &mut TestOrchestrator,
    chat_id: i64,
    messages: Vec<Message>,
) {
    let topic_id = if orchestrator.state().open_chat().chat_id() == Some(chat_id) {
        orchestrator.state().open_chat().topic_id()
    } else {
        None
    };
    inject_topic_messages(orchestrator, chat_id, topic_id, messages);
}

/// Helper: inject a topic-scoped (or chat-scoped) messages background result.
pub(super) fn inject_topic_messages(
    orchestrator: &mut TestOrchestrator,
    chat_id: i64,
    topic_id: Option<i32>,
    messages: Vec<Message>,
) {
    orchestrator
        .handle_event(AppEvent::BackgroundTaskCompleted(
            BackgroundTaskResult::MessagesLoaded {
                chat_id,
                topic_id,
                result: Ok(messages),
            },
        ))
        .unwrap();
}

/// Helper: set up orchestrator with a loaded chat list (skip the dispatch+result dance).
pub(super) fn orchestrator_with_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
    let mut o = make_orchestrator();
    inject_chat_list(&mut o, chats);
    o
}

/// Helper: set up orchestrator with a loaded chat list and an opened chat.
pub(super) fn orchestrator_with_open_chat(
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

// ── Orchestrator factory with pre-populated cache ──

pub(super) fn make_orchestrator_with_cached_chats(chats: Vec<ChatSummary>) -> TestOrchestrator {
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
        std::collections::HashMap::new(),
    )
}

pub(super) fn make_orchestrator_with_cache(
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
        std::collections::HashMap::new(),
    )
}

pub(super) fn make_orchestrator_with_threshold(
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
        std::collections::HashMap::new(),
    )
}

pub(super) fn make_orchestrator_with_cache_and_threshold(
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
        std::collections::HashMap::new(),
    )
}
