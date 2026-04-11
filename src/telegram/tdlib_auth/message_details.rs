use crate::domain::message_info_state::{MessageInfo, ReactionDetail, ViewerDetail};
use crate::usecases::message_info::MessageInfoQuery;

use super::TdLibAuthBackend;

impl TdLibAuthBackend {
    pub fn resolve_message_info(&self, query: &MessageInfoQuery) -> MessageInfo {
        let chat_id = query.chat_id;
        let message_id = query.message_id;

        let msg = self.client.get_message(chat_id, message_id).ok();
        let edit_date = msg.as_ref().and_then(|m| {
            if m.edit_date > 0 {
                Some(m.edit_date)
            } else {
                None
            }
        });

        let can_get_added_reactions = msg
            .as_ref()
            .and_then(|m| m.interaction_info.as_ref())
            .and_then(|info| info.reactions.as_ref())
            .is_some_and(|r| r.can_get_added_reactions);

        let reactions = if can_get_added_reactions {
            self.resolve_reactions(chat_id, message_id)
        } else {
            vec![]
        };

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
            reactions,
            viewers,
            read_date,
            edit_date,
        }
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
