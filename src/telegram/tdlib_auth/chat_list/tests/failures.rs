use std::sync::{Arc, Mutex, OnceLock};

use tracing::subscriber::NoSubscriber;
use tracing::Dispatch;

use super::*;

// ---------------------------------------------------------------------------
// Failure scenarios
// ---------------------------------------------------------------------------

#[test]
fn double_miss_skips_chat() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![999], NO_FORCE);

    assert!(summaries.is_empty(), "unresolvable chat should be skipped");
    assert_eq!(
        resolver.get_chat_call_count(),
        1,
        "get_chat called as fallback"
    );
}

#[test]
fn all_chats_fail_returns_empty_vec() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![10, 20, 30], NO_FORCE);

    assert!(summaries.is_empty());
    assert_eq!(resolver.get_chat_call_count(), 3);
}

#[test]
fn partial_failure_returns_resolved_chats_only() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Good Chat"));

    let summaries = build_summaries_from_ids(&resolver, vec![1, 2], NO_FORCE);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].chat_id, 1);
}

// ---------------------------------------------------------------------------
// Unresolved-chat logging
// ---------------------------------------------------------------------------

#[test]
fn unresolved_chats_log_one_summary_warning() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Good Chat"));

    let (summaries, logs) =
        capture_logs(|| build_summaries_from_ids(&resolver, vec![1, 2, 3], NO_FORCE));

    assert_eq!(summaries.len(), 1);
    let warnings = log_lines(&logs, "WARN");
    assert_eq!(warnings.len(), 1, "expected one summary warning: {logs}");
    assert!(warnings[0].contains("unresolved=2"), "{logs}");
    assert!(warnings[0].contains("requested=3"), "{logs}");
    let per_chat = log_lines(&logs, "DEBUG")
        .into_iter()
        .filter(|line| line.contains("chat missing from cache and TDLib"))
        .count();
    assert_eq!(per_chat, 2, "{logs}");
}

#[test]
fn force_unresolved_chats_log_one_summary_warning() {
    let resolver = FakeResolver::new();

    let (summaries, logs) = capture_logs(|| build_summaries_from_ids(&resolver, vec![1, 2], true));

    assert!(summaries.is_empty());
    let warnings = log_lines(&logs, "WARN");
    assert_eq!(warnings.len(), 1, "expected one summary warning: {logs}");
    assert!(warnings[0].contains("unresolved=2"), "{logs}");
    let per_chat = log_lines(&logs, "DEBUG")
        .into_iter()
        .filter(|line| line.contains("chat missing from TDLib (force refresh)"))
        .count();
    assert_eq!(per_chat, 2, "{logs}");
}

#[test]
fn resolved_chats_log_no_warning() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Good Chat"));

    let (_, logs) = capture_logs(|| build_summaries_from_ids(&resolver, vec![1], NO_FORCE));

    assert!(log_lines(&logs, "WARN").is_empty(), "{logs}");
}

// ---------------------------------------------------------------------------
// Empty input
// ---------------------------------------------------------------------------

#[test]
fn empty_chat_ids_returns_empty_vec() {
    let resolver = FakeResolver::new();

    let summaries = build_summaries_from_ids(&resolver, vec![], NO_FORCE);

    assert!(summaries.is_empty());
    assert_eq!(resolver.get_chat_call_count(), 0);
}

// ---------------------------------------------------------------------------
// Chat list availability
// ---------------------------------------------------------------------------

#[test]
fn unresolvable_chat_ids_report_unavailable() {
    let resolver = FakeResolver::new();

    let result = resolve_chat_list(&resolver, vec![10, 20], true, NO_FORCE);

    assert_eq!(result, Err(ListChatsSourceError::Unavailable));
}

#[test]
fn zero_chat_ids_before_all_loaded_report_unavailable() {
    let resolver = FakeResolver::new();

    let result = resolve_chat_list(&resolver, vec![], false, NO_FORCE);

    assert_eq!(result, Err(ListChatsSourceError::Unavailable));
}

#[test]
fn zero_chat_ids_after_all_loaded_is_empty_list() {
    let resolver = FakeResolver::new();

    let result = resolve_chat_list(&resolver, vec![], true, NO_FORCE);

    assert_eq!(result, Ok((vec![], true)));
}

#[test]
fn partially_resolved_chat_ids_return_resolved_chats() {
    let resolver = FakeResolver::new().with_cached_chat(make_test_chat(1, "Good Chat"));

    let (summaries, all_loaded) =
        resolve_chat_list(&resolver, vec![1, 2], false, NO_FORCE).expect("partial list is ok");

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].chat_id, 1);
    assert!(!all_loaded);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_logs<T>(f: impl FnOnce() -> T) -> (T, String) {
    // While a single dispatcher is registered, tracing caches a callsite's
    // interest from the default subscriber of whichever thread hits it first,
    // so a parallel test without a subscriber could disable these events.
    // A second live dispatcher makes tracing combine all registered ones.
    static SECOND_DISPATCHER: OnceLock<Dispatch> = OnceLock::new();
    SECOND_DISPATCHER.get_or_init(|| Dispatch::new(NoSubscriber::default()));

    let buffer = LogBuffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    let logs = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    (result, logs)
}

fn log_lines<'a>(logs: &'a str, level: &str) -> Vec<&'a str> {
    logs.lines()
        .filter(|line| line.trim_start().starts_with(level))
        .collect()
}
