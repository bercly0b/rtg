//! Chat lifecycle management: open/close and mark-as-read.
//!
//! Provides traits for TDLib chat lifecycle operations that are
//! managed by the orchestrator rather than individual use cases.

/// Error type for chat lifecycle operations.
///
/// These are best-effort operations — failures are logged but do not
/// prevent the application from continuing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChatLifecycleError {
    Unavailable,
    ChatNotFound,
}

/// Manages the TDLib `openChat`/`closeChat` lifecycle.
///
/// While a chat is open in TDLib:
/// - All updates for the chat are delivered (important for supergroups/channels)
/// - `viewMessages` with `force_read: false` can mark messages as read
///
/// Every `open_chat` call must be paired with a `close_chat` call.
pub trait ChatLifecycle: Send + Sync {
    fn open_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError>;
    fn close_chat(&self, chat_id: i64) -> Result<(), ChatLifecycleError>;
}

/// Deletes messages from a chat via TDLib.
///
/// When `revoke` is true, the messages are deleted for all participants
/// (if allowed by Telegram). When false, only for the current user.
pub trait MessageDeleter: Send + Sync {
    fn delete_messages(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
        revoke: bool,
    ) -> Result<(), ChatLifecycleError>;
}

/// Triggers an asynchronous file download in TDLib.
///
/// Progress is delivered via `updateFile` events. This is a fire-and-forget
/// operation — the caller does not wait for the download to complete.
pub trait FileDownloader: Send + Sync {
    fn download_file(&self, file_id: i32) -> Result<(), ChatLifecycleError>;
}

/// Marks messages as viewed/read in a chat.
///
/// The chat should be opened via [`ChatLifecycle::open_chat`] before
/// calling this. TDLib will send `Update::ChatReadInbox` when the read
/// state changes, which triggers a reactive chat list refresh with
/// updated `unread_count`.
pub trait ChatReadMarker: Send + Sync {
    fn mark_messages_read(
        &self,
        chat_id: i64,
        message_ids: Vec<i64>,
    ) -> Result<(), ChatLifecycleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubLifecycle;

    impl ChatLifecycle for StubLifecycle {
        fn open_chat(&self, _chat_id: i64) -> Result<(), ChatLifecycleError> {
            Ok(())
        }
        fn close_chat(&self, _chat_id: i64) -> Result<(), ChatLifecycleError> {
            Ok(())
        }
    }

    struct StubReadMarker;

    impl ChatReadMarker for StubReadMarker {
        fn mark_messages_read(
            &self,
            _chat_id: i64,
            _message_ids: Vec<i64>,
        ) -> Result<(), ChatLifecycleError> {
            Ok(())
        }
    }

    struct StubDeleter;

    impl MessageDeleter for StubDeleter {
        fn delete_messages(
            &self,
            _chat_id: i64,
            _message_ids: Vec<i64>,
            _revoke: bool,
        ) -> Result<(), ChatLifecycleError> {
            Ok(())
        }
    }

    #[test]
    fn stub_lifecycle_succeeds() {
        let lifecycle = StubLifecycle;
        assert!(lifecycle.open_chat(1).is_ok());
        assert!(lifecycle.close_chat(1).is_ok());
    }

    #[test]
    fn stub_read_marker_succeeds() {
        let marker = StubReadMarker;
        assert!(marker.mark_messages_read(1, vec![1, 2, 3]).is_ok());
    }

    #[test]
    fn stub_deleter_succeeds() {
        let deleter = StubDeleter;
        assert!(deleter.delete_messages(1, vec![1, 2], true).is_ok());
        assert!(deleter.delete_messages(1, vec![1, 2], false).is_ok());
    }
}
