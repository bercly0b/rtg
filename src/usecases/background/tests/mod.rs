mod error_mapping;
mod file_ops;

use crate::domain::message_input_state::OutgoingMessage;
use crate::usecases::{
    background::TaskDispatcher,
    chat_subtitle::{ChatInfoQuery, ChatSubtitleQuery},
    message_info::MessageInfoQuery,
    message_reactions::AvailableReactionsQuery,
};

/// Stub dispatcher that ignores every dispatched operation.
pub struct StubTaskDispatcher;

impl TaskDispatcher for StubTaskDispatcher {
    fn dispatch_chat_list(&self, _force: bool, _limit: usize) {}

    fn dispatch_load_forum_topics(&self, _chat_id: i64) {}

    fn dispatch_forum_unread_counts(&self, _chat_ids: Vec<i64>) {}

    fn dispatch_load_messages(&self, _chat_id: i64, _topic_id: Option<i32>) {}

    fn dispatch_load_older_messages(
        &self,
        _chat_id: i64,
        _topic_id: Option<i32>,
        _from_message_id: i64,
    ) {
    }

    fn dispatch_send_message(
        &self,
        _chat_id: i64,
        _topic_id: Option<i32>,
        _message: OutgoingMessage,
    ) {
    }

    fn dispatch_edit_message(&self, _chat_id: i64, _message_id: i64, _text: String) {}

    fn dispatch_open_chat(&self, _chat_id: i64) {}

    fn dispatch_close_chat(&self, _chat_id: i64) {}

    fn dispatch_mark_as_read(&self, _chat_id: i64, _topic_id: Option<i32>, _message_ids: Vec<i64>) {
    }

    fn dispatch_mark_chat_as_read(&self, _chat_id: i64, _last_message_id: i64) {}

    fn dispatch_prefetch_messages(&self, _chat_id: i64, _topic_id: Option<i32>) {}

    fn dispatch_delete_message(&self, _chat_id: i64, _message_id: i64) {}

    fn dispatch_chat_subtitle(&self, _query: ChatSubtitleQuery) {}

    fn dispatch_send_voice(
        &self,
        _chat_id: i64,
        _topic_id: Option<i32>,
        _file_path: String,
        _pending_id: i64,
    ) {
    }

    fn dispatch_download_file(&self, _file_id: i32) {}

    fn dispatch_chat_info(&self, _query: ChatInfoQuery) {}

    fn dispatch_open_file(&self, _cmd_template: String, _file_path: String) {}

    fn dispatch_save_file(&self, _file_id: i32, _local_path: String, _file_name: Option<String>) {}

    fn dispatch_message_info(&self, _query: MessageInfoQuery) {}

    fn dispatch_available_reactions(&self, _query: AvailableReactionsQuery) {}

    fn dispatch_add_reaction(&self, _chat_id: i64, _message_id: i64, _emoji: String) {}

    fn dispatch_remove_reaction(&self, _chat_id: i64, _message_id: i64, _emoji: String) {}
}

// ── dispatch_forum_unread_counts: the badge warm-up sweep ──

mod forum_unread_counts {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::domain::events::BackgroundTaskResult;
    use crate::domain::forum_topic::ForumTopicSummary;
    use crate::usecases::background::lifecycle::dispatch_forum_unread_counts;
    use crate::usecases::list_forum_topics::{ForumTopicsSource, ListForumTopicsSourceError};

    struct StubForumTopicsSource {
        topics_by_chat: HashMap<i64, Vec<ForumTopicSummary>>,
    }

    impl ForumTopicsSource for StubForumTopicsSource {
        fn list_forum_topics(
            &self,
            chat_id: i64,
            _limit: usize,
        ) -> Result<Vec<ForumTopicSummary>, ListForumTopicsSourceError> {
            self.topics_by_chat
                .get(&chat_id)
                .cloned()
                .ok_or(ListForumTopicsSourceError::Unavailable)
        }
    }

    fn topic(chat_id: i64, topic_id: i32, unread_count: u32) -> ForumTopicSummary {
        ForumTopicSummary {
            chat_id,
            topic_id,
            name: format!("Topic {topic_id}"),
            is_general: false,
            is_closed: false,
            is_hidden: false,
            is_pinned: false,
            unread_count,
            last_message_preview: None,
            last_message_unix_ms: None,
            last_message_id: None,
            order: i64::from(topic_id),
        }
    }

    fn run_sweep(
        topics_by_chat: HashMap<i64, Vec<ForumTopicSummary>>,
        chat_ids: Vec<i64>,
    ) -> BackgroundTaskResult {
        let source = Arc::new(StubForumTopicsSource { topics_by_chat });
        let (tx, rx) = std::sync::mpsc::channel();

        dispatch_forum_unread_counts(&source, &tx, chat_ids);

        rx.recv_timeout(Duration::from_secs(5))
            .expect("sweep must always deliver a result")
    }

    #[test]
    fn counts_unread_topics_per_chat() {
        let topics_by_chat = HashMap::from([
            (1, vec![topic(1, 1, 3), topic(1, 2, 0), topic(1, 3, 1)]),
            (2, vec![topic(2, 1, 0)]),
        ]);

        let result = run_sweep(topics_by_chat, vec![1, 2]);

        match result {
            BackgroundTaskResult::ForumUnreadCountsLoaded { counts } => {
                assert_eq!(counts, vec![(1, 2), (2, 0)]);
            }
            other => panic!("expected ForumUnreadCountsLoaded, got: {other:?}"),
        }
    }

    #[test]
    fn failed_chat_is_skipped_but_result_still_sent() {
        let topics_by_chat = HashMap::from([(1, vec![topic(1, 1, 5)])]);

        // Chat 2 is unknown to the stub -> fetch fails; the sweep must still
        // deliver chat 1 and send the result so the in-flight guard clears.
        let result = run_sweep(topics_by_chat, vec![1, 2]);

        match result {
            BackgroundTaskResult::ForumUnreadCountsLoaded { counts } => {
                assert_eq!(counts, vec![(1, 1)]);
            }
            other => panic!("expected ForumUnreadCountsLoaded, got: {other:?}"),
        }
    }

    #[test]
    fn empty_input_sends_empty_result() {
        let result = run_sweep(HashMap::new(), vec![]);

        match result {
            BackgroundTaskResult::ForumUnreadCountsLoaded { counts } => {
                assert!(counts.is_empty());
            }
            other => panic!("expected ForumUnreadCountsLoaded, got: {other:?}"),
        }
    }
}

// ── dispatch_send_message / dispatch_send_voice: rollback payload ──

mod send_rollback {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::domain::events::{BackgroundError, BackgroundTaskResult};
    use crate::domain::message::Message;
    use crate::domain::message_input_state::{OutgoingMessage, ReplyContext};
    use crate::usecases::background::messaging::{dispatch_send_message, dispatch_send_voice};
    use crate::usecases::load_messages::{MessagesSource, MessagesSourceError};
    use crate::usecases::send_message::{MessageSender, SendMessageSourceError};
    use crate::usecases::send_voice::VoiceNoteSender;

    #[derive(Default)]
    struct FailingSender {
        reply_to_message_id: Mutex<Option<i64>>,
    }

    impl MessageSender for FailingSender {
        fn send_message(
            &self,
            _chat_id: i64,
            _topic_id: Option<i32>,
            _text: &str,
            reply_to_message_id: Option<i64>,
        ) -> Result<(), SendMessageSourceError> {
            *self.reply_to_message_id.lock().unwrap() = reply_to_message_id;
            Err(SendMessageSourceError::Unavailable)
        }
    }

    impl VoiceNoteSender for FailingSender {
        fn send_voice_note(
            &self,
            _chat_id: i64,
            _topic_id: Option<i32>,
            _file_path: &str,
            _duration: i32,
            _waveform: &str,
        ) -> Result<(), SendMessageSourceError> {
            Err(SendMessageSourceError::Unavailable)
        }
    }

    struct NoMessages;

    impl MessagesSource for NoMessages {
        fn list_messages(
            &self,
            _chat_id: i64,
            _topic_id: Option<i32>,
            _limit: usize,
            _from_message_id: i64,
        ) -> Result<Vec<Message>, MessagesSourceError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn failed_send_passes_reply_id_and_echoes_rollback_payload() {
        let sender = Arc::new(FailingSender::default());
        let (tx, rx) = std::sync::mpsc::channel();
        let message = OutgoingMessage {
            pending_id: -5,
            text: "Yo".to_owned(),
            reply_to: Some(ReplyContext {
                message_id: 7,
                sender_name: "Bob".to_owned(),
                text: "Hello".to_owned(),
            }),
        };

        dispatch_send_message(
            &sender,
            &Arc::new(NoMessages),
            &tx,
            1,
            Some(3),
            message.clone(),
        );

        let result = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("send must always deliver a result");
        assert_eq!(*sender.reply_to_message_id.lock().unwrap(), Some(7));
        assert_eq!(
            result,
            BackgroundTaskResult::MessageSent {
                chat_id: 1,
                topic_id: Some(3),
                message,
                result: Err(BackgroundError::new("SEND_UNAVAILABLE")),
            }
        );
    }

    #[test]
    fn failed_voice_send_echoes_pending_id() {
        let (tx, rx) = std::sync::mpsc::channel();

        dispatch_send_voice(
            &Arc::new(FailingSender::default()),
            &Arc::new(NoMessages),
            &tx,
            1,
            None,
            "/nonexistent/rtg_test_voice.oga".to_owned(),
            -9,
        );

        let result = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("voice send must report its failure");
        assert_eq!(
            result,
            BackgroundTaskResult::VoiceSendFailed {
                chat_id: 1,
                pending_id: -9,
            }
        );
    }
}
