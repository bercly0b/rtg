use chrono::{DateTime, Local};
use tdlib_rs::enums::NotificationSettingsScope;
use tdlib_rs::types::{
    BasicGroup, BasicGroupFullInfo, Chat, CountryInfo, ScopeNotificationSettings, SecretChat,
    Supergroup, SupergroupFullInfo, User, UserFullInfo, VerificationStatus,
};

use crate::domain::chat::ChatType;
use crate::domain::chat_info_state::{Badge, ChatInfo};
use crate::domain::chat_subtitle::ChatSubtitle;
use crate::telegram::tdlib_client::{TdLibClient, TdLibError};
use crate::usecases::chat_subtitle::{ChatInfoQuery, ChatSubtitleError};

use super::TdLibAuthBackend;
use crate::telegram::tdlib_mappers;

mod chat_block;
mod group_info;
mod user_info;

/// TDLib requests behind the chat info popup, separated from `TdLibClient`
/// so the resolution logic can be unit-tested with a fake.
pub(super) trait ChatInfoResolver {
    fn get_chat(&self, chat_id: i64) -> Result<Chat, TdLibError>;
    fn get_user(&self, user_id: i64) -> Result<User, TdLibError>;
    fn get_user_full_info(&self, user_id: i64) -> Result<UserFullInfo, TdLibError>;
    fn get_basic_group(&self, basic_group_id: i64) -> Result<BasicGroup, TdLibError>;
    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError>;
    fn get_supergroup(&self, supergroup_id: i64) -> Result<Supergroup, TdLibError>;
    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError>;
    fn get_secret_chat(&self, secret_chat_id: i32) -> Result<SecretChat, TdLibError>;
    fn get_scope_notification_settings(
        &self,
        scope: NotificationSettingsScope,
    ) -> Result<ScopeNotificationSettings, TdLibError>;
    fn get_countries(&self) -> Result<Vec<CountryInfo>, TdLibError>;
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

    fn get_basic_group(&self, basic_group_id: i64) -> Result<BasicGroup, TdLibError> {
        self.get_basic_group(basic_group_id)
    }

    fn get_basic_group_full_info(
        &self,
        basic_group_id: i64,
    ) -> Result<BasicGroupFullInfo, TdLibError> {
        self.get_basic_group_full_info(basic_group_id)
    }

    fn get_supergroup(&self, supergroup_id: i64) -> Result<Supergroup, TdLibError> {
        self.get_supergroup(supergroup_id)
    }

    fn get_supergroup_full_info(
        &self,
        supergroup_id: i64,
    ) -> Result<SupergroupFullInfo, TdLibError> {
        self.get_supergroup_full_info(supergroup_id)
    }

    fn get_secret_chat(&self, secret_chat_id: i32) -> Result<SecretChat, TdLibError> {
        self.get_secret_chat(secret_chat_id)
    }

    fn get_scope_notification_settings(
        &self,
        scope: NotificationSettingsScope,
    ) -> Result<ScopeNotificationSettings, TdLibError> {
        self.get_scope_notification_settings(scope)
    }

    fn get_countries(&self) -> Result<Vec<CountryInfo>, TdLibError> {
        self.get_countries()
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
    pub fn resolve_chat_info(&self, query: &ChatInfoQuery) -> Result<ChatInfo, ChatSubtitleError> {
        resolve_chat_info(&self.client, query, Local::now())
    }
}

/// Only the requests behind the header are critical: if one fails the popup
/// shows an error rather than an empty status or "0 members". Every other
/// request only fills rows, which show `—` when it fails.
pub(super) fn resolve_chat_info(
    resolver: &dyn ChatInfoResolver,
    query: &ChatInfoQuery,
    now: DateTime<Local>,
) -> Result<ChatInfo, ChatSubtitleError> {
    let chat = resolver
        .get_chat(query.chat_id)
        .map_err(|e| request_failed("getChat", query.chat_id, e))?;

    let (status_line, kind) = match query.chat_type {
        ChatType::Private => user_info::resolve(resolver, &chat, now)?,
        ChatType::Group => group_info::resolve_group(resolver, &chat, now)?,
        ChatType::Channel => group_info::resolve_channel(resolver, &chat, now)?,
    };

    Ok(ChatInfo {
        title: query.title.clone(),
        status_line,
        kind,
        chat: chat_block::resolve(resolver, &chat, &query.folders, now),
    })
}

fn request_failed(request: &'static str, id: i64, error: TdLibError) -> ChatSubtitleError {
    tracing::warn!(request, id, error = %error, "chat info request failed");
    ChatSubtitleError::Unavailable
}

fn optional<T>(request: &'static str, result: Result<T, TdLibError>) -> Option<T> {
    result
        .inspect_err(|error| {
            tracing::debug!(request, error = %error, "chat info: optional request failed");
        })
        .ok()
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

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn non_zero(value: i32) -> Option<i32> {
    (value != 0).then_some(value)
}

fn chat_title(resolver: &dyn ChatInfoResolver, chat_id: i64) -> Option<String> {
    if chat_id == 0 {
        return None;
    }
    optional("getChat", resolver.get_chat(chat_id)).map(|chat| chat.title)
}

fn verification_badges(status: Option<&VerificationStatus>) -> Vec<Badge> {
    let Some(status) = status else {
        return Vec::new();
    };
    [
        (status.is_verified, Badge::Verified),
        (status.is_scam, Badge::Scam),
        (status.is_fake, Badge::Fake),
    ]
    .into_iter()
    .filter_map(|(is_set, badge)| is_set.then_some(badge))
    .collect()
}

#[cfg(test)]
mod tests;
