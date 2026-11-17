//! Whose chat info is resolved: a chat, a user or a mention.

use super::*;
use crate::domain::chat_info_state::ChatInfoKind;

fn resolve(
    resolver: &FakeResolver,
    subject: ChatInfoSubject,
) -> Result<ChatInfo, ChatSubtitleError> {
    resolve_subject(resolver, subject, ChatFolders::default())
}

#[test]
fn title_and_template_come_from_the_tdlib_chat() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, supergroup_chat(7, 70, true));
    resolver
        .supergroup_full_infos
        .insert(70, supergroup_full_info(10, ""));

    let info = resolve(&resolver, ChatInfoSubject::Chat(7)).unwrap();

    assert_eq!(info.title, "Supergroup");
    assert!(matches!(info.kind, ChatInfoKind::Channel(_)));
}

#[test]
fn chat_with_a_deleted_account_is_titled_deleted() {
    let mut resolver = FakeResolver::default();
    let mut chat = private_chat(1, 1);
    chat.title = String::new();
    resolver.chats.insert(1, chat);
    resolver.users.insert(1, user(1));

    let info = resolve(&resolver, ChatInfoSubject::Chat(1)).unwrap();

    assert_eq!(info.title, "Deleted");
}

#[test]
fn user_is_resolved_through_create_private_chat() {
    let mut resolver = FakeResolver::default();
    resolver.private_chats.insert(5, private_chat(5, 5));
    resolver.users.insert(5, user(5));

    let info = resolve(&resolver, ChatInfoSubject::User(5)).unwrap();

    assert_eq!(info.title, "Private");
    assert!(matches!(info.kind, ChatInfoKind::User(_)));
    assert_eq!(info.chat.chat_id, 5);
    assert!(resolver.requested("createPrivateChat"));
    assert!(!resolver.requested("getChat"));
}

#[test]
fn user_without_a_reachable_chat_returns_error() {
    let resolver = FakeResolver::default();

    let result = resolve(&resolver, ChatInfoSubject::User(5));

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}

#[test]
fn mention_is_resolved_through_search_public_chat() {
    let mut resolver = FakeResolver::default();
    resolver
        .public_chats
        .insert("rust_talks".to_owned(), supergroup_chat(6, 60, false));
    resolver
        .supergroup_full_infos
        .insert(60, supergroup_full_info(3, ""));

    let info = resolve(&resolver, ChatInfoSubject::Mention("rust_talks".to_owned())).unwrap();

    assert_eq!(info.title, "Supergroup");
    assert!(matches!(info.kind, ChatInfoKind::Group(_)));
}

#[test]
fn mention_of_an_unoccupied_username_is_not_found() {
    let resolver = FakeResolver::default();

    let result = resolve(&resolver, ChatInfoSubject::Mention("nobody".to_owned()));

    assert_eq!(result, Err(ChatSubtitleError::NotFound));
}

#[test]
fn failed_mention_lookup_returns_error() {
    let resolver = FakeResolver {
        search_public_chat_fails: true,
        ..FakeResolver::default()
    };

    let result = resolve(&resolver, ChatInfoSubject::Mention("alice".to_owned()));

    assert_eq!(result, Err(ChatSubtitleError::Unavailable));
}
