use tdlib_rs::enums::{ChatType as TdChatType, MessageSender, MessageTopic};
use tdlib_rs::types::{Chat, Message, Usernames};

use crate::domain::global_search_state::{
    ChatScope, FoundChat, FoundMessage, FoundMessagesPage, CHATS_LIMIT, MESSAGES_PAGE_SIZE,
};
use crate::telegram::tdlib_client::TdLibError;
use crate::telegram::tdlib_mappers;
use crate::usecases::global_search::GlobalSearchError;

use super::chat_list::{resolve_user, ChatDataResolver};
use super::TdLibAuthBackend;

impl TdLibAuthBackend {
    /// Finds the chats of `scope` matching `query`: `searchChats`,
    /// `searchChatsOnServer`, `searchContacts` or `searchPublicChats`.
    pub fn search_chats(
        &self,
        query: &str,
        scope: ChatScope,
    ) -> Result<Vec<FoundChat>, GlobalSearchError> {
        let limit = i32::try_from(CHATS_LIMIT).unwrap_or(i32::MAX);
        let chat_ids = match scope {
            ChatScope::Known => self.client.search_chats(query, limit),
            ChatScope::Server => self.client.search_chats_on_server(query, limit),
            ChatScope::Public => self.client.search_public_chats(query),
            ChatScope::Contacts => self
                .client
                .search_contacts(query, limit)
                .map(|user_ids| self.private_chat_ids(user_ids)),
        }
        .map_err(|error| unavailable(&error))?;

        Ok(chat_ids
            .into_iter()
            .filter_map(|chat_id| found_chat(&self.client, chat_id))
            .collect())
    }

    /// Finds a page of messages matching `query` via `searchMessages`.
    pub fn search_messages(
        &self,
        query: &str,
        offset: &str,
    ) -> Result<FoundMessagesPage, GlobalSearchError> {
        let limit = i32::try_from(MESSAGES_PAGE_SIZE).unwrap_or(i32::MAX);
        let found = self
            .client
            .search_messages(query, offset, limit)
            .map_err(|error| unavailable(&error))?;

        Ok(FoundMessagesPage {
            messages: found
                .messages
                .iter()
                .filter_map(|message| found_message(&self.client, message))
                .collect(),
            next_offset: Some(found.next_offset).filter(|offset| !offset.is_empty()),
        })
    }

    /// A contact is a user, and a user is opened as a private chat; a
    /// contact whose chat TDLib cannot make is skipped.
    fn private_chat_ids(&self, user_ids: Vec<i64>) -> Vec<i64> {
        user_ids
            .into_iter()
            .filter_map(|user_id| match self.client.create_private_chat_offline(user_id) {
                Ok(chat) => Some(chat.id),
                Err(error) => {
                    tracing::debug!(user_id, error = %error, "no private chat for a found contact");
                    None
                }
            })
            .collect()
    }
}

fn unavailable(error: &TdLibError) -> GlobalSearchError {
    tracing::warn!(error = %error, "global search request failed");
    GlobalSearchError::Unavailable
}

/// Describes a found chat; `None` when TDLib does not know the chat.
fn found_chat(resolver: &dyn ChatDataResolver, chat_id: i64) -> Option<FoundChat> {
    let chat = resolve_chat(resolver, chat_id)?;
    let cache = resolver.cache();
    let (usernames, is_forum) = match &chat.r#type {
        TdChatType::Supergroup(supergroup) => cache
            .get_supergroup(supergroup.supergroup_id)
            .map_or((None, false), |sg| (sg.usernames, sg.is_forum)),
        other => (
            tdlib_mappers::get_private_chat_user_id(other)
                .and_then(|user_id| resolve_user(resolver, cache, user_id))
                .and_then(|user| user.usernames),
            false,
        ),
    };

    Some(FoundChat {
        chat_id,
        title: tdlib_mappers::chat_display_title(&chat),
        username: usernames.and_then(first_active_username),
        chat_type: tdlib_mappers::map_chat_type(&chat.r#type),
        is_forum,
    })
}

fn first_active_username(usernames: Usernames) -> Option<String> {
    usernames.active_usernames.into_iter().next()
}

/// Describes a found message; `None` when TDLib does not know its chat.
fn found_message(resolver: &dyn ChatDataResolver, message: &Message) -> Option<FoundMessage> {
    let chat = found_chat(resolver, message.chat_id)?;
    let topic_id = match &message.topic_id {
        Some(MessageTopic::Forum(topic)) if chat.is_forum => Some(topic.forum_topic_id),
        _ => None,
    };

    Some(FoundMessage {
        message_id: message.id,
        topic_id,
        sender_name: sender_name(resolver, message),
        timestamp_ms: i64::from(message.date) * 1000,
        preview: tdlib_mappers::extract_message_preview(&message.content).unwrap_or_default(),
        chat,
    })
}

fn sender_name(resolver: &dyn ChatDataResolver, message: &Message) -> String {
    if message.is_outgoing {
        return "You".to_owned();
    }
    match &message.sender_id {
        MessageSender::User(sender) => resolve_user(resolver, resolver.cache(), sender.user_id)
            .map_or_else(
                || "Unknown".to_owned(),
                |user| tdlib_mappers::format_user_name(&user),
            ),
        MessageSender::Chat(sender) => resolve_chat(resolver, sender.chat_id).map_or_else(
            || "Channel".to_owned(),
            |chat| tdlib_mappers::chat_display_title(&chat),
        ),
    }
}

/// Reads a chat from the cache, falling back to TDLib's own storage.
fn resolve_chat(resolver: &dyn ChatDataResolver, chat_id: i64) -> Option<Chat> {
    let cache = resolver.cache();
    cache.get_chat(chat_id).or_else(|| {
        let chat = resolver
            .get_chat(chat_id)
            .inspect_err(
                |error| tracing::debug!(chat_id, error = %error, "found chat is unknown to TDLib"),
            )
            .ok()?;
        cache.upsert_chat(chat.clone());
        Some(chat)
    })
}

#[cfg(test)]
mod tests {
    use tdlib_rs::types::{
        ChatTypeSupergroup, MessageSenderChat, MessageSenderUser, MessageTopicForum, User,
    };

    use super::*;
    use crate::domain::chat::ChatType;
    use crate::telegram::tdlib_cache::tests::{
        make_test_chat, make_test_supergroup, make_test_topic_message, make_test_user,
    };
    use crate::telegram::tdlib_cache::TdLibCache;

    /// Knows only what its cache holds.
    struct CachedOnly(TdLibCache);

    impl ChatDataResolver for CachedOnly {
        fn cache(&self) -> &TdLibCache {
            &self.0
        }

        fn get_chat(&self, _chat_id: i64) -> Result<Chat, TdLibError> {
            Err(unknown())
        }

        fn get_user(&self, _user_id: i64) -> Result<User, TdLibError> {
            Err(unknown())
        }
    }

    fn unknown() -> TdLibError {
        TdLibError::Request {
            code: 400,
            message: "not found".to_owned(),
        }
    }

    fn usernames(active: &str) -> Option<Usernames> {
        Some(Usernames {
            active_usernames: vec![active.to_owned()],
            disabled_usernames: Vec::new(),
            editable_username: active.to_owned(),
            collectible_usernames: Vec::new(),
        })
    }

    /// A supergroup chat `chat_id` backed by supergroup `chat_id`.
    fn supergroup_chat(chat_id: i64, title: &str, is_channel: bool) -> Chat {
        Chat {
            r#type: TdChatType::Supergroup(ChatTypeSupergroup {
                supergroup_id: chat_id,
                is_channel,
            }),
            ..make_test_chat(chat_id, title)
        }
    }

    fn resolver_with_forum(chat_id: i64) -> CachedOnly {
        let resolver = CachedOnly(TdLibCache::new());
        resolver
            .0
            .upsert_chat(supergroup_chat(chat_id, "Forum", false));
        resolver
            .0
            .upsert_supergroup(make_test_supergroup(chat_id, true));
        resolver
    }

    fn message_in(chat_id: i64, message_id: i64) -> Message {
        Message {
            chat_id,
            ..make_test_topic_message(message_id)
        }
    }

    #[test]
    fn a_private_chat_is_found_with_the_username_of_its_user() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver.0.upsert_chat(make_test_chat(7, "Alice"));
        resolver.0.upsert_user(User {
            usernames: usernames("alice"),
            ..make_test_user(7, "Alice")
        });

        assert_eq!(
            found_chat(&resolver, 7),
            Some(FoundChat {
                chat_id: 7,
                title: "Alice".to_owned(),
                username: Some("alice".to_owned()),
                chat_type: ChatType::Private,
                is_forum: false,
            })
        );
    }

    #[test]
    fn a_chat_without_a_username_is_found_without_one() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver.0.upsert_chat(make_test_chat(7, "Alice"));
        resolver.0.upsert_user(make_test_user(7, "Alice"));

        assert_eq!(found_chat(&resolver, 7).unwrap().username, None);
    }

    #[test]
    fn a_channel_is_found_with_the_username_of_its_supergroup() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver
            .0
            .upsert_chat(supergroup_chat(9, "Rust News", true));
        resolver.0.upsert_supergroup(tdlib_rs::types::Supergroup {
            usernames: usernames("rust_news"),
            ..make_test_supergroup(9, false)
        });

        let found = found_chat(&resolver, 9).unwrap();

        assert_eq!(found.username.as_deref(), Some("rust_news"));
        assert_eq!(found.chat_type, ChatType::Channel);
        assert!(!found.is_forum);
    }

    #[test]
    fn a_forum_is_found_as_a_forum() {
        let found = found_chat(&resolver_with_forum(9), 9).unwrap();

        assert_eq!(found.chat_type, ChatType::Group);
        assert!(found.is_forum);
    }

    #[test]
    fn a_chat_unknown_to_tdlib_is_not_found() {
        assert_eq!(found_chat(&CachedOnly(TdLibCache::new()), 7), None);
    }

    #[test]
    fn a_message_is_found_with_its_chat_sender_date_and_chat_list_preview() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver
            .0
            .upsert_chat(supergroup_chat(9, "Rust chat", false));
        resolver.0.upsert_user(make_test_user(1, "Bob"));

        let found = found_message(&resolver, &message_in(9, 100)).unwrap();

        assert_eq!(found.chat.chat_id, 9);
        assert_eq!(found.chat.title, "Rust chat");
        assert_eq!(found.message_id, 100);
        assert_eq!(found.sender_name, "Bob");
        assert_eq!(found.timestamp_ms, 1_700_000_000_000);
        assert_eq!(found.preview, "test");
        assert_eq!(found.topic_id, None);
    }

    #[test]
    fn an_outgoing_message_is_from_you() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver.0.upsert_chat(make_test_chat(7, "Alice"));
        let message = Message {
            is_outgoing: true,
            ..message_in(7, 100)
        };

        assert_eq!(
            found_message(&resolver, &message).unwrap().sender_name,
            "You"
        );
    }

    #[test]
    fn a_channel_post_is_from_the_channel() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver
            .0
            .upsert_chat(supergroup_chat(9, "Rust News", true));
        let message = Message {
            sender_id: MessageSender::Chat(MessageSenderChat { chat_id: 9 }),
            ..message_in(9, 100)
        };

        assert_eq!(
            found_message(&resolver, &message).unwrap().sender_name,
            "Rust News"
        );
    }

    #[test]
    fn a_message_of_a_forum_topic_carries_the_topic() {
        let message = Message {
            topic_id: Some(MessageTopic::Forum(MessageTopicForum { forum_topic_id: 5 })),
            sender_id: MessageSender::User(MessageSenderUser { user_id: 1 }),
            ..message_in(9, 100)
        };

        let found = found_message(&resolver_with_forum(9), &message).unwrap();

        assert_eq!(found.topic_id, Some(5));
    }

    #[test]
    fn a_topic_outside_a_forum_is_ignored() {
        let resolver = CachedOnly(TdLibCache::new());
        resolver.0.upsert_chat(make_test_chat(7, "Bot"));
        let message = Message {
            topic_id: Some(MessageTopic::Forum(MessageTopicForum { forum_topic_id: 5 })),
            ..message_in(7, 100)
        };

        assert_eq!(found_message(&resolver, &message).unwrap().topic_id, None);
    }

    #[test]
    fn a_message_of_a_chat_unknown_to_tdlib_is_not_found() {
        let resolver = CachedOnly(TdLibCache::new());

        assert_eq!(found_message(&resolver, &message_in(9, 100)), None);
    }
}
