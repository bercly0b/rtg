use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use super::types::{TdLibConfig, TdLibError, TDLIB_ERROR_ALL_CHATS_LOADED};
use super::update_loop::map_connection_state;
use super::TdLibClient;
use crate::domain::events::ConnectivityStatus;
use crate::telegram::tdlib_cache::TdLibCache;
use crate::telegram::tdlib_updates::TdLibUpdate;

#[test]
fn config_stores_all_fields() {
    let config = TdLibConfig {
        api_id: 12345,
        api_hash: "test_hash".into(),
        database_directory: PathBuf::from("/tmp/test_db"),
        files_directory: PathBuf::from("/tmp/test_files"),
        log_file: PathBuf::from("/tmp/test_logs/tdlib.log"),
        verbose: false,
    };

    assert_eq!(config.api_id, 12345);
    assert_eq!(config.api_hash, "test_hash");
    assert_eq!(config.database_directory, PathBuf::from("/tmp/test_db"));
    assert_eq!(config.files_directory, PathBuf::from("/tmp/test_files"));
    assert_eq!(config.log_file, PathBuf::from("/tmp/test_logs/tdlib.log"));
}

#[test]
fn tdlib_error_all_chats_loaded_code_is_404() {
    assert_eq!(TDLIB_ERROR_ALL_CHATS_LOADED, 404);
}

#[test]
fn request_error_displays_code_and_message() {
    let error = TdLibError::Request {
        code: 404,
        message: "Chat list loading completed".to_owned(),
    };
    let display = format!("{error}");
    assert!(display.contains("404"));
    assert!(display.contains("Chat list loading completed"));
}

#[test]
fn tdlib_rs_error_converts_into_request_error() {
    let error = TdLibError::from(tdlib_rs::types::Error {
        code: 400,
        message: "Chat not found".to_owned(),
    });

    assert!(matches!(
        error,
        TdLibError::Request { code: 400, ref message } if message == "Chat not found"
    ));
}

#[test]
fn recv_auth_state_reports_poisoned_lock_as_internal_error() {
    let (_auth_state_tx, auth_state_rx) = mpsc::channel();
    let client = TdLibClient {
        client_id: 0,
        config: TdLibConfig {
            api_id: 1,
            api_hash: "hash".into(),
            database_directory: PathBuf::from("/tmp/test_db"),
            files_directory: PathBuf::from("/tmp/test_files"),
            log_file: PathBuf::from("/tmp/test_logs/tdlib.log"),
            verbose: false,
        },
        rt: Arc::new(tokio::runtime::Runtime::new().expect("runtime should build")),
        auth_state_rx: Mutex::new(auth_state_rx),
        update_rx: Mutex::new(None),
        connectivity_rx: Mutex::new(None),
        update_thread: Mutex::new(None),
        is_closed: AtomicBool::new(true),
        cache: TdLibCache::new(),
    };
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = client.auth_state_rx.lock();
        panic!("poison the auth state receiver lock");
    }));

    let result = client.recv_auth_state(Duration::from_millis(1));

    assert!(matches!(result, Err(TdLibError::Internal { .. })));
}

#[test]
fn config_debug_redacts_api_hash() {
    let config = TdLibConfig {
        api_id: 12345,
        api_hash: "secret_hash".into(),
        database_directory: PathBuf::from("/tmp/test_db"),
        files_directory: PathBuf::from("/tmp/test_files"),
        log_file: PathBuf::from("/tmp/test_logs/tdlib.log"),
        verbose: false,
    };

    let debug_output = format!("{:?}", config);
    assert!(debug_output.contains("[REDACTED]"));
    assert!(!debug_output.contains("secret_hash"));
}

#[test]
fn map_connection_state_covers_all_tdlib_variants() {
    use tdlib_rs::enums::ConnectionState;

    assert_eq!(
        map_connection_state(&ConnectionState::WaitingForNetwork),
        ConnectivityStatus::Disconnected
    );
    assert_eq!(
        map_connection_state(&ConnectionState::ConnectingToProxy),
        ConnectivityStatus::Connecting
    );
    assert_eq!(
        map_connection_state(&ConnectionState::Connecting),
        ConnectivityStatus::Connecting
    );
    assert_eq!(
        map_connection_state(&ConnectionState::Updating),
        ConnectivityStatus::Updating
    );
    assert_eq!(
        map_connection_state(&ConnectionState::Ready),
        ConnectivityStatus::Connected
    );
}

#[test]
fn publish_unread_reaction_count_updates_cache_and_emits_update() {
    let cache = crate::telegram::tdlib_cache::TdLibCache::new();
    let chat = crate::telegram::tdlib_cache::tests::make_test_chat(42, "General");
    cache.upsert_chat(chat);

    let (tx, rx) = mpsc::channel();

    TdLibClient::publish_unread_reaction_count(&tx, &cache, 42, 0);

    let cached = cache.get_chat(42).expect("chat should exist in cache");
    assert_eq!(cached.unread_reaction_count, 0);

    match rx.recv().expect("should emit unread reaction update") {
        TdLibUpdate::ChatUnreadReactionCount { chat_id } => assert_eq!(chat_id, 42),
        other => panic!("unexpected update kind: {}", other.kind()),
    }
}

#[test]
fn forum_topic_id_extracts_forum_topic() {
    use super::update_loop::forum_topic_id;

    let mut message = crate::telegram::tdlib_cache::tests::make_test_topic_message(1);
    assert_eq!(forum_topic_id(&message), None);

    message.topic_id = Some(tdlib_rs::enums::MessageTopic::Forum(
        tdlib_rs::types::MessageTopicForum { forum_topic_id: 7 },
    ));
    assert_eq!(forum_topic_id(&message), Some(7));

    message.topic_id = Some(tdlib_rs::enums::MessageTopic::SavedMessages(
        tdlib_rs::types::MessageTopicSavedMessages {
            saved_messages_topic_id: 3,
        },
    ));
    assert_eq!(forum_topic_id(&message), None);
}

#[test]
fn forward_update_delivers_to_live_receiver() {
    use super::update_loop::forward_update;

    let (tx, rx) = mpsc::channel();

    forward_update(&tx, TdLibUpdate::UserStatus { user_id: 7 });

    match rx.try_recv().expect("update should be delivered") {
        TdLibUpdate::UserStatus { user_id } => assert_eq!(user_id, 7),
        other => panic!("unexpected update kind: {}", other.kind()),
    }
}

#[test]
fn forward_update_tolerates_dropped_receiver() {
    use super::update_loop::forward_update;

    let (tx, rx) = mpsc::channel();
    drop(rx);

    forward_update(&tx, TdLibUpdate::UserStatus { user_id: 7 });
}

fn auth_state(state: tdlib_rs::enums::AuthorizationState) -> super::types::AuthStateUpdate {
    super::types::AuthStateUpdate { state }
}

#[test]
fn await_closed_and_join_waits_for_closed_then_joins_update_loop() {
    use super::shutdown::await_closed_and_join;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tdlib_rs::enums::AuthorizationState;

    let (tx, rx) = mpsc::channel();
    let loop_finished = Arc::new(AtomicBool::new(false));
    let update_loop = {
        let loop_finished = Arc::clone(&loop_finished);
        std::thread::spawn(move || {
            tx.send(auth_state(AuthorizationState::Ready)).unwrap();
            tx.send(auth_state(AuthorizationState::Closing)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(50));
            tx.send(auth_state(AuthorizationState::Closed)).unwrap();
            loop_finished.store(true, Ordering::Release);
        })
    };

    await_closed_and_join(&rx, Some(update_loop), std::time::Duration::from_secs(5))
        .expect("close should complete once Closed arrives");

    assert!(loop_finished.load(Ordering::Acquire));
}

#[test]
fn await_closed_and_join_times_out_without_closed() {
    use super::shutdown::await_closed_and_join;
    use tdlib_rs::enums::AuthorizationState;

    let (tx, rx) = mpsc::channel();
    tx.send(auth_state(AuthorizationState::Closing)).unwrap();

    let error = await_closed_and_join(&rx, None, std::time::Duration::from_millis(50))
        .expect_err("close must not complete without Closed");

    assert!(matches!(error, TdLibError::Timeout { .. }), "got: {error}");
}

#[test]
fn await_closed_and_join_accepts_update_loop_that_already_stopped() {
    use super::shutdown::await_closed_and_join;

    let (tx, rx) = mpsc::channel::<super::types::AuthStateUpdate>();
    let update_loop = std::thread::spawn(move || drop(tx));

    await_closed_and_join(&rx, Some(update_loop), std::time::Duration::from_secs(5))
        .expect("a stopped update loop means TDLib already reported Closed");
}
