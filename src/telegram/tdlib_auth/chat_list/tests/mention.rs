use crate::usecases::resolve_mention::MentionSourceError;

use super::*;

fn request_error(code: i32, message: &str) -> TdLibError {
    TdLibError::Request {
        code,
        message: message.to_owned(),
    }
}

#[test]
fn found_chat_is_summarized_without_another_tdlib_call() {
    let resolver = FakeResolver::new();

    let summary = summarize_public_chat(&resolver, Ok(make_test_chat(7, "Alice")))
        .expect("found chat resolves")
        .expect("found chat has a summary");

    assert_eq!(summary.chat_id, 7);
    assert_eq!(summary.title, "Alice");
    assert_eq!(resolver.get_chat_call_count(), 0);
}

#[test]
fn unoccupied_username_resolves_to_no_chat() {
    let resolver = FakeResolver::new();

    let resolved =
        summarize_public_chat(&resolver, Err(request_error(400, "USERNAME_NOT_OCCUPIED")));

    assert_eq!(resolved, Ok(None));
}

#[test]
fn other_failures_are_reported_as_unavailable() {
    let resolver = FakeResolver::new();

    let resolved = summarize_public_chat(
        &resolver,
        Err(TdLibError::Timeout {
            message: "searchPublicChat".to_owned(),
        }),
    );

    assert_eq!(resolved, Err(MentionSourceError::Unavailable));
}
