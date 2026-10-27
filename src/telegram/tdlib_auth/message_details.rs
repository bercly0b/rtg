use crate::domain::message_info_state::{
    MessageAuthor, MessageInfo, ReactionCount, ReactionDetail, ViewerDetail,
};
use crate::telegram::tdlib_mappers;
use crate::usecases::message_info::MessageInfoQuery;

use super::TdLibAuthBackend;

impl TdLibAuthBackend {
    pub fn resolve_message_info(&self, query: &MessageInfoQuery) -> MessageInfo {
        let chat_id = query.chat_id;
        let message_id = query.message_id;

        let msg = self.client.get_message(chat_id, message_id).ok();
        let author = msg.as_ref().map(|m| self.resolve_author(&m.sender_id));
        let edit_date = msg.as_ref().and_then(|m| {
            if m.edit_date > 0 {
                Some(m.edit_date)
            } else {
                None
            }
        });

        let msg_reactions = msg
            .as_ref()
            .and_then(|m| m.interaction_info.as_ref())
            .and_then(|info| info.reactions.as_ref());

        let reactions = if msg_reactions.is_some_and(|r| r.can_get_added_reactions) {
            self.resolve_reactions(chat_id, message_id)
        } else {
            self.extract_inline_reactions(msg_reactions)
        };

        let reaction_counts = extract_reaction_counts(msg_reactions);

        let props = self.client.get_message_properties(chat_id, message_id).ok();

        let viewers = if props.as_ref().is_some_and(|p| p.can_get_viewers) {
            self.resolve_viewers(chat_id, message_id)
        } else {
            vec![]
        };

        let read_date = if props.as_ref().is_some_and(|p| p.can_get_read_date) {
            self.resolve_read_date(chat_id, message_id)
        } else {
            None
        };

        MessageInfo {
            author,
            reactions,
            reaction_counts,
            viewers,
            read_date,
            edit_date,
        }
    }

    fn extract_inline_reactions(
        &self,
        msg_reactions: Option<&tdlib_rs::types::MessageReactions>,
    ) -> Vec<ReactionDetail> {
        extract_inline_reactions(msg_reactions, |s| self.resolve_sender_name(s))
    }

    fn resolve_reactions(&self, chat_id: i64, message_id: i64) -> Vec<ReactionDetail> {
        let Ok(added) = self.client.get_message_added_reactions(chat_id, message_id) else {
            return vec![];
        };

        added
            .reactions
            .into_iter()
            .map(|r| {
                let emoji = match r.r#type {
                    tdlib_rs::enums::ReactionType::Emoji(e) => e.emoji,
                    tdlib_rs::enums::ReactionType::CustomEmoji(_) => "⭐".to_owned(),
                    tdlib_rs::enums::ReactionType::Paid => "💎".to_owned(),
                };
                let sender_name = self.resolve_sender_name(&r.sender_id);
                ReactionDetail { emoji, sender_name }
            })
            .collect()
    }

    fn resolve_viewers(&self, chat_id: i64, message_id: i64) -> Vec<ViewerDetail> {
        let Ok(viewers) = self.client.get_message_viewers(chat_id, message_id) else {
            return vec![];
        };

        viewers
            .viewers
            .into_iter()
            .map(|v| {
                let name = self.resolve_user_name(v.user_id);
                ViewerDetail {
                    name,
                    view_date: v.view_date,
                }
            })
            .collect()
    }

    fn resolve_read_date(&self, chat_id: i64, message_id: i64) -> Option<i32> {
        let Ok(read_date) = self.client.get_message_read_date(chat_id, message_id) else {
            return None;
        };

        match read_date {
            tdlib_rs::enums::MessageReadDate::Read(r) => Some(r.read_date),
            _ => None,
        }
    }

    fn resolve_author(&self, sender_id: &tdlib_rs::enums::MessageSender) -> MessageAuthor {
        let cache = self.client.cache();
        match sender_id {
            tdlib_rs::enums::MessageSender::User(u) => cache
                .get_user(u.user_id)
                .or_else(|| self.client.get_user(u.user_id).ok())
                .map(|user| user_author(&user))
                .unwrap_or_else(|| MessageAuthor {
                    name: "Unknown".to_owned(),
                    username: None,
                }),
            tdlib_rs::enums::MessageSender::Chat(c) => {
                let chat = cache
                    .get_chat(c.chat_id)
                    .or_else(|| self.client.get_chat(c.chat_id).ok());
                let supergroup = chat.as_ref().and_then(|chat| match &chat.r#type {
                    tdlib_rs::enums::ChatType::Supergroup(s) => {
                        cache.get_supergroup(s.supergroup_id)
                    }
                    _ => None,
                });
                match chat {
                    Some(chat) => chat_author(&chat, supergroup.as_ref()),
                    None => MessageAuthor {
                        name: "Channel".to_owned(),
                        username: None,
                    },
                }
            }
        }
    }

    fn resolve_sender_name(&self, sender_id: &tdlib_rs::enums::MessageSender) -> String {
        match sender_id {
            tdlib_rs::enums::MessageSender::User(u) => self.resolve_user_name(u.user_id),
            tdlib_rs::enums::MessageSender::Chat(c) => self
                .client
                .get_chat(c.chat_id)
                .map(|chat| chat.title)
                .unwrap_or_else(|_| format!("Chat#{}", c.chat_id)),
        }
    }

    fn resolve_user_name(&self, user_id: i64) -> String {
        let user = self
            .client
            .cache()
            .get_user(user_id)
            .or_else(|| self.client.get_user(user_id).ok());

        match user {
            Some(u) => {
                let name = format!("{} {}", u.first_name, u.last_name)
                    .trim()
                    .to_owned();
                if name.is_empty() {
                    format!("User#{user_id}")
                } else {
                    name
                }
            }
            None => format!("User#{user_id}"),
        }
    }
}

fn user_author(user: &tdlib_rs::types::User) -> MessageAuthor {
    MessageAuthor {
        name: tdlib_mappers::format_user_name(user),
        username: first_active_username(user.usernames.as_ref()),
    }
}

fn chat_author(
    chat: &tdlib_rs::types::Chat,
    supergroup: Option<&tdlib_rs::types::Supergroup>,
) -> MessageAuthor {
    MessageAuthor {
        name: chat.title.clone(),
        username: supergroup.and_then(|s| first_active_username(s.usernames.as_ref())),
    }
}

fn first_active_username(usernames: Option<&tdlib_rs::types::Usernames>) -> Option<String> {
    usernames.and_then(|u| u.active_usernames.first().cloned())
}

fn extract_inline_reactions(
    msg_reactions: Option<&tdlib_rs::types::MessageReactions>,
    resolve_name: impl Fn(&tdlib_rs::enums::MessageSender) -> String,
) -> Vec<ReactionDetail> {
    let Some(reactions) = msg_reactions else {
        return vec![];
    };

    reactions
        .reactions
        .iter()
        .flat_map(|r| {
            let emoji = match &r.r#type {
                tdlib_rs::enums::ReactionType::Emoji(e) => e.emoji.clone(),
                tdlib_rs::enums::ReactionType::CustomEmoji(_) => "⭐".to_owned(),
                tdlib_rs::enums::ReactionType::Paid => "💎".to_owned(),
            };

            r.recent_sender_ids
                .iter()
                .map(|s| ReactionDetail {
                    emoji: emoji.clone(),
                    sender_name: resolve_name(s),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn extract_reaction_counts(
    msg_reactions: Option<&tdlib_rs::types::MessageReactions>,
) -> Vec<ReactionCount> {
    let Some(reactions) = msg_reactions else {
        return vec![];
    };

    reactions
        .reactions
        .iter()
        .filter(|r| r.total_count > 0)
        .map(|r| {
            let emoji = match &r.r#type {
                tdlib_rs::enums::ReactionType::Emoji(e) => e.emoji.clone(),
                tdlib_rs::enums::ReactionType::CustomEmoji(_) => "⭐".to_owned(),
                tdlib_rs::enums::ReactionType::Paid => "💎".to_owned(),
            };
            ReactionCount {
                emoji,
                count: r.total_count,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_resolve_name(sender: &tdlib_rs::enums::MessageSender) -> String {
        match sender {
            tdlib_rs::enums::MessageSender::User(u) => format!("User#{}", u.user_id),
            tdlib_rs::enums::MessageSender::Chat(c) => format!("Chat#{}", c.chat_id),
        }
    }

    fn make_emoji_reaction(emoji: &str, sender_ids: Vec<i64>) -> tdlib_rs::types::MessageReaction {
        tdlib_rs::types::MessageReaction {
            r#type: tdlib_rs::enums::ReactionType::Emoji(tdlib_rs::types::ReactionTypeEmoji {
                emoji: emoji.to_owned(),
            }),
            total_count: sender_ids.len() as i32,
            is_chosen: false,
            used_sender_id: None,
            recent_sender_ids: sender_ids
                .into_iter()
                .map(|id| {
                    tdlib_rs::enums::MessageSender::User(tdlib_rs::types::MessageSenderUser {
                        user_id: id,
                    })
                })
                .collect(),
        }
    }

    fn make_reactions(
        reactions: Vec<tdlib_rs::types::MessageReaction>,
    ) -> tdlib_rs::types::MessageReactions {
        tdlib_rs::types::MessageReactions {
            reactions,
            are_tags: false,
            paid_reactors: vec![],
            can_get_added_reactions: false,
        }
    }

    fn make_usernames(active: &[&str]) -> tdlib_rs::types::Usernames {
        tdlib_rs::types::Usernames {
            active_usernames: active.iter().map(|u| (*u).to_owned()).collect(),
            disabled_usernames: vec![],
            editable_username: active.first().map(|u| (*u).to_owned()).unwrap_or_default(),
            collectible_usernames: vec![],
        }
    }

    #[test]
    fn user_author_uses_display_name_and_first_active_username() {
        let mut user = crate::telegram::tdlib_cache::tests::make_test_user(1, "Alice");
        user.last_name = "Smith".to_owned();
        user.usernames = Some(make_usernames(&["alice", "alice_alt"]));

        let author = user_author(&user);

        assert_eq!(author.name, "Alice Smith");
        assert_eq!(author.username.as_deref(), Some("alice"));
    }

    #[test]
    fn user_author_without_usernames_has_no_username() {
        let user = crate::telegram::tdlib_cache::tests::make_test_user(1, "Alice");

        let author = user_author(&user);

        assert_eq!(author.name, "Alice");
        assert_eq!(author.username, None);
    }

    #[test]
    fn user_author_with_only_disabled_usernames_has_no_username() {
        let mut user = crate::telegram::tdlib_cache::tests::make_test_user(1, "Alice");
        user.usernames = Some(make_usernames(&[]));

        assert_eq!(user_author(&user).username, None);
    }

    #[test]
    fn chat_author_uses_title_and_supergroup_username() {
        let chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "News");
        let mut supergroup = crate::telegram::tdlib_cache::tests::make_test_supergroup(2, false);
        supergroup.usernames = Some(make_usernames(&["news"]));

        let author = chat_author(&chat, Some(&supergroup));

        assert_eq!(author.name, "News");
        assert_eq!(author.username.as_deref(), Some("news"));
    }

    #[test]
    fn chat_author_without_supergroup_has_no_username() {
        let chat = crate::telegram::tdlib_cache::tests::make_test_chat(1, "News");

        let author = chat_author(&chat, None);

        assert_eq!(author.name, "News");
        assert_eq!(author.username, None);
    }

    #[test]
    fn none_reactions_returns_empty() {
        let result = extract_inline_reactions(None, stub_resolve_name);
        assert!(result.is_empty());
    }

    #[test]
    fn emoji_reaction_with_recent_sender() {
        let reactions = make_reactions(vec![make_emoji_reaction("👍", vec![42])]);
        let result = extract_inline_reactions(Some(&reactions), stub_resolve_name);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].emoji, "👍");
        assert_eq!(result[0].sender_name, "User#42");
    }

    #[test]
    fn no_recent_senders_skips_reaction() {
        let reactions = make_reactions(vec![make_emoji_reaction("🔥", vec![])]);
        let result = extract_inline_reactions(Some(&reactions), stub_resolve_name);

        assert!(result.is_empty());
    }

    #[test]
    fn custom_emoji_mapped_to_star() {
        let reaction = tdlib_rs::types::MessageReaction {
            r#type: tdlib_rs::enums::ReactionType::CustomEmoji(
                tdlib_rs::types::ReactionTypeCustomEmoji {
                    custom_emoji_id: 123,
                },
            ),
            total_count: 1,
            is_chosen: false,
            used_sender_id: None,
            recent_sender_ids: vec![tdlib_rs::enums::MessageSender::User(
                tdlib_rs::types::MessageSenderUser { user_id: 1 },
            )],
        };

        let reactions = make_reactions(vec![reaction]);
        let result = extract_inline_reactions(Some(&reactions), stub_resolve_name);

        assert_eq!(result[0].emoji, "⭐");
    }

    #[test]
    fn paid_reaction_mapped_to_gem() {
        let reaction = tdlib_rs::types::MessageReaction {
            r#type: tdlib_rs::enums::ReactionType::Paid,
            total_count: 1,
            is_chosen: false,
            used_sender_id: None,
            recent_sender_ids: vec![tdlib_rs::enums::MessageSender::User(
                tdlib_rs::types::MessageSenderUser { user_id: 1 },
            )],
        };

        let reactions = make_reactions(vec![reaction]);
        let result = extract_inline_reactions(Some(&reactions), stub_resolve_name);

        assert_eq!(result[0].emoji, "💎");
    }

    #[test]
    fn counts_none_returns_empty() {
        let result = extract_reaction_counts(None);
        assert!(result.is_empty());
    }

    #[test]
    fn counts_extracted_without_recent_senders() {
        let reactions = make_reactions(vec![tdlib_rs::types::MessageReaction {
            r#type: tdlib_rs::enums::ReactionType::Emoji(tdlib_rs::types::ReactionTypeEmoji {
                emoji: "👍".to_owned(),
            }),
            total_count: 5,
            is_chosen: false,
            used_sender_id: None,
            recent_sender_ids: vec![],
        }]);
        let result = extract_reaction_counts(Some(&reactions));

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].emoji, "👍");
        assert_eq!(result[0].count, 5);
    }

    #[test]
    fn counts_skip_zero_total() {
        let reactions = make_reactions(vec![make_emoji_reaction("🔥", vec![])]);
        let result = extract_reaction_counts(Some(&reactions));

        assert!(result.is_empty());
    }

    #[test]
    fn counts_multiple_reactions() {
        let reactions = make_reactions(vec![
            make_emoji_reaction("👍", vec![1, 2]),
            make_emoji_reaction("❤", vec![3]),
        ]);
        let result = extract_reaction_counts(Some(&reactions));

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].emoji, "👍");
        assert_eq!(result[0].count, 2);
        assert_eq!(result[1].emoji, "❤");
        assert_eq!(result[1].count, 1);
    }

    #[test]
    fn multiple_reactions_flattened() {
        let reactions = make_reactions(vec![
            make_emoji_reaction("👍", vec![1, 2]),
            make_emoji_reaction("❤", vec![3]),
        ]);
        let result = extract_inline_reactions(Some(&reactions), stub_resolve_name);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].emoji, "👍");
        assert_eq!(result[0].sender_name, "User#1");
        assert_eq!(result[1].emoji, "👍");
        assert_eq!(result[1].sender_name, "User#2");
        assert_eq!(result[2].emoji, "❤");
        assert_eq!(result[2].sender_name, "User#3");
    }
}
