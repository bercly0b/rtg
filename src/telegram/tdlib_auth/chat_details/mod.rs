use tdlib_rs::types::{BasicGroupFullInfo, Chat, SupergroupFullInfo, User, UserFullInfo};

use crate::domain::chat::ChatType;
use crate::domain::chat_info_state::ChatInfo;
use crate::domain::chat_subtitle::ChatSubtitle;
use crate::telegram::tdlib_client::{TdLibClient, TdLibError};
use crate::usecases::chat_subtitle::ChatSubtitleError;

use super::TdLibAuthBackend;
use crate::telegram::tdlib_mappers;

/// TDLib requests behind the chat info popup, separated from `TdLibClient`
/// so the resolution logic can be unit-tested with a fake.
pub(super) trait ChatInfoResolver {
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError>;
    fn get_user(&self, user_id: i64) -> Result<User, TdLibError>;
    fn get_user_full_info(&self, user_id: i64) -> Result<UserFullInfo, TdLibError>;
    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError>;
    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError>;
}

impl ChatInfoResolver for TdLibClient {
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError> {
        self.get_chat(chat_id)
    }

    fn get_user(&self, user_id: i64) -> Result<User, TdLibError> {
        self.get_user(user_id)
    }

    fn get_user_full_info(&self, user_id: i64) -> Result<UserFullInfo, TdLibError> {
        self.get_user_full_info(user_id)
    }

    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError> {
        self.get_basic_group_full_info(basic_group_id)
    }

    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError> {
        self.get_supergroup_full_info(supergroup_id)
    }
}

impl TdLibAuthBackend {
    /// Resolves the chat subtitle (user status, member count, etc.).
    pub fn resolve_subtitle(&self, chat_id: i64) -> ChatSubtitle {
        let chat = match self.client.get_chat(chat_id) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(chat_id, error = ?e, "failed to get chat for subtitle");
                return ChatSubtitle::None;
            }
        };

        let chat_type = tdlib_mappers::map_chat_type(&chat.r#type);

        match chat_type {
            ChatType::Private => self.resolve_private_subtitle(&chat.r#type),
            ChatType::Group => self.resolve_group_subtitle(&chat.r#type),
            ChatType::Channel => self.resolve_channel_subtitle(&chat.r#type),
        }
    }

    fn resolve_private_subtitle(&self, td_type: &tdlib_rs::enums::ChatType) -> ChatSubtitle {
        let Some(user_id) = tdlib_mappers::get_private_chat_user_id(td_type) else {
            return ChatSubtitle::None;
        };
        let user = match self.client.get_user(user_id) {
            Ok(u) => u,
            Err(_) => return ChatSubtitle::None,
        };
        if matches!(user.r#type, tdlib_rs::enums::UserType::Bot(_)) {
            return ChatSubtitle::Bot;
        }
        tdlib_mappers::map_user_status_to_subtitle(&user.status)
    }

    fn resolve_group_subtitle(&self, td_type: &tdlib_rs::enums::ChatType) -> ChatSubtitle {
        match td_type {
            tdlib_rs::enums::ChatType::BasicGroup(bg) => {
                match self.client.get_basic_group_full_info(bg.basic_group_id) {
                    Ok(info) => ChatSubtitle::Members(info.members.len() as i32),
                    Err(_) => ChatSubtitle::None,
                }
            }
            tdlib_rs::enums::ChatType::Supergroup(sg) => {
                match self.client.get_supergroup_full_info(sg.supergroup_id) {
                    Ok(info) => ChatSubtitle::Members(info.member_count),
                    Err(_) => ChatSubtitle::None,
                }
            }
            _ => ChatSubtitle::None,
        }
    }

    fn resolve_channel_subtitle(&self, td_type: &tdlib_rs::enums::ChatType) -> ChatSubtitle {
        if let tdlib_rs::enums::ChatType::Supergroup(sg) = td_type {
            match self.client.get_supergroup_full_info(sg.supergroup_id) {
                Ok(info) => ChatSubtitle::Subscribers(info.member_count),
                Err(_) => ChatSubtitle::None,
            }
        } else {
            ChatSubtitle::None
        }
    }

    /// Resolves full chat info for the chat info popup.
    pub fn resolve_chat_info(
        &self,
        chat_id: i64,
        chat_type: ChatType,
        title: String,
    ) -> Result<ChatInfo, ChatSubtitleError> {
        resolve_chat_info(&self.client, chat_id, chat_type, title)
    }
}

/// Every request that feeds the status line is critical: if it fails the
/// popup shows an error rather than an empty status or "0 members". Only the
/// user bio may be missing, since the rest of a private chat's info is still
/// accurate without it.
pub(super) fn resolve_chat_info(
    resolver: &dyn ChatInfoResolver,
    chat_id: i64,
    chat_type: ChatType,
    title: String,
) -> Result<ChatInfo, ChatSubtitleError> {
    let chat = resolver
        .get_chat(chat_id)
        .map_err(|e| request_failed("getChat", chat_id, e))?;

    match chat_type {
        ChatType::Private => resolve_private_info(resolver, &chat.r#type, title),
        ChatType::Group => resolve_group_info(resolver, &chat.r#type, title),
        ChatType::Channel => resolve_channel_info(resolver, &chat.r#type, title),
    }
}

fn request_failed(request: &'static str, id: i64, error: TdLibError) -> ChatSubtitleError {
    tracing::warn!(request, id, error = %error, "chat info request failed");
    ChatSubtitleError::Unavailable
}

fn unexpected_chat_type(
    chat_type: ChatType,
    td_type: &tdlib_rs::enums::ChatType,
) -> ChatSubtitleError {
    tracing::warn!(
        ?chat_type,
        ?td_type,
        "chat info: TDLib chat type does not match"
    );
    ChatSubtitleError::Unavailable
}

fn non_empty_description(description: String) -> Option<String> {
    if description.trim().is_empty() {
        None
    } else {
        Some(description)
    }
}

fn resolve_private_info(
    resolver: &dyn ChatInfoResolver,
    td_type: &tdlib_rs::enums::ChatType,
    title: String,
) -> Result<ChatInfo, ChatSubtitleError> {
    use chrono::Local;

    let Some(user_id) = tdlib_mappers::get_private_chat_user_id(td_type) else {
        return Err(unexpected_chat_type(ChatType::Private, td_type));
    };

    let user = resolver
        .get_user(user_id)
        .map_err(|e| request_failed("getUser", user_id, e))?;

    let is_bot = matches!(user.r#type, tdlib_rs::enums::UserType::Bot(_));
    let status_line = if is_bot {
        "bot".to_owned()
    } else {
        let subtitle = tdlib_mappers::map_user_status_to_subtitle(&user.status);
        subtitle.format(Local::now())
    };

    let username = user
        .usernames
        .and_then(|u| u.active_usernames.into_iter().next())
        .map(|u| format!("@{u}"));

    let description = match resolver.get_user_full_info(user_id) {
        Ok(info) => info
            .bio
            .and_then(|ft| non_empty_description(ft.text.trim().to_owned())),
        Err(e) => {
            tracing::debug!(user_id, error = %e, "chat info: user bio unavailable, showing without it");
            None
        }
    };

    Ok(ChatInfo {
        title,
        chat_type: ChatType::Private,
        status_line,
        username,
        description,
    })
}

fn resolve_group_info(
    resolver: &dyn ChatInfoResolver,
    td_type: &tdlib_rs::enums::ChatType,
    title: String,
) -> Result<ChatInfo, ChatSubtitleError> {
    let (member_count, description) = match td_type {
        tdlib_rs::enums::ChatType::BasicGroup(bg) => {
            let info = resolver
                .get_basic_group_full_info(bg.basic_group_id)
                .map_err(|e| request_failed("getBasicGroupFullInfo", bg.basic_group_id, e))?;
            (info.members.len() as i32, info.description)
        }
        tdlib_rs::enums::ChatType::Supergroup(sg) => {
            let info = resolver
                .get_supergroup_full_info(sg.supergroup_id)
                .map_err(|e| request_failed("getSupergroupFullInfo", sg.supergroup_id, e))?;
            (info.member_count, info.description)
        }
        _ => return Err(unexpected_chat_type(ChatType::Group, td_type)),
    };

    Ok(ChatInfo {
        title,
        chat_type: ChatType::Group,
        status_line: ChatSubtitle::Members(member_count).format(chrono::Local::now()),
        username: None,
        description: non_empty_description(description),
    })
}

fn resolve_channel_info(
    resolver: &dyn ChatInfoResolver,
    td_type: &tdlib_rs::enums::ChatType,
    title: String,
) -> Result<ChatInfo, ChatSubtitleError> {
    let tdlib_rs::enums::ChatType::Supergroup(sg) = td_type else {
        return Err(unexpected_chat_type(ChatType::Channel, td_type));
    };

    let info = resolver
        .get_supergroup_full_info(sg.supergroup_id)
        .map_err(|e| request_failed("getSupergroupFullInfo", sg.supergroup_id, e))?;

    Ok(ChatInfo {
        title,
        chat_type: ChatType::Channel,
        status_line: ChatSubtitle::Subscribers(info.member_count).format(chrono::Local::now()),
        username: None,
        description: non_empty_description(info.description),
    })
}

#[cfg(test)]
mod tests;
