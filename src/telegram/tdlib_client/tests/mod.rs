use std::path::PathBuf;
use std::sync::mpsc;

use super::types::{TdLibConfig, TdLibError, TDLIB_ERROR_ALL_CHATS_LOADED};
use super::update_loop::map_connection_state;
use super::TdLibClient;
use crate::domain::events::ConnectivityStatus;
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
