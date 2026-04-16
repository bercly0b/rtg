use crate::domain::chat::ChatType;
use crate::domain::chat_subtitle::ChatSubtitle;

use super::TdLibAuthBackend;
use crate::telegram::tdlib_mappers;

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
    ) -> crate::domain::chat_info_state::ChatInfo {
        use crate::domain::chat_info_state::ChatInfo;

        let chat = match self.client.get_chat(chat_id) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(chat_id, error = ?e, "failed to get chat for info popup");
                return ChatInfo {
                    title,
                    chat_type,
                    status_line: String::new(),
                    username: None,
                    description: None,
                };
            }
        };

        match chat_type {
            ChatType::Private => self.resolve_private_info(&chat.r#type, title),
            ChatType::Group => self.resolve_group_info(&chat.r#type, title),
            ChatType::Channel => self.resolve_channel_info(&chat.r#type, title),
        }
    }

    fn resolve_private_info(
        &self,
        td_type: &tdlib_rs::enums::ChatType,
        title: String,
    ) -> crate::domain::chat_info_state::ChatInfo {
        use crate::domain::chat_info_state::ChatInfo;
        use chrono::Local;

        let Some(user_id) = tdlib_mappers::get_private_chat_user_id(td_type) else {
            return ChatInfo {
                title,
                chat_type: ChatType::Private,
                status_line: String::new(),
                username: None,
                description: None,
            };
        };

        let user = match self.client.get_user(user_id) {
            Ok(u) => u,
            Err(_) => {
                return ChatInfo {
                    title,
                    chat_type: ChatType::Private,
                    status_line: String::new(),
                    username: None,
                    description: None,
                };
            }
        };

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

        let description = self
            .client
            .get_user_full_info(user_id)
            .ok()
            .and_then(|info| {
                let ft = info.bio?;
                let t = ft.text.trim().to_owned();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            });

        ChatInfo {
            title,
            chat_type: ChatType::Private,
            status_line,
            username,
            description,
        }
    }

    fn resolve_group_info(
        &self,
        td_type: &tdlib_rs::enums::ChatType,
        title: String,
    ) -> crate::domain::chat_info_state::ChatInfo {
        use crate::domain::chat_info_state::ChatInfo;

        match td_type {
            tdlib_rs::enums::ChatType::BasicGroup(bg) => {
                let (member_count, description) =
                    match self.client.get_basic_group_full_info(bg.basic_group_id) {
                        Ok(info) => {
                            let desc = if info.description.trim().is_empty() {
                                None
                            } else {
                                Some(info.description.clone())
                            };
                            (info.members.len() as i32, desc)
                        }
                        Err(_) => (0, None),
                    };

                ChatInfo {
                    title,
                    chat_type: ChatType::Group,
                    status_line: ChatSubtitle::Members(member_count).format(chrono::Local::now()),
                    username: None,
                    description,
                }
            }
            tdlib_rs::enums::ChatType::Supergroup(sg) => {
                let (member_count, description) =
                    match self.client.get_supergroup_full_info(sg.supergroup_id) {
                        Ok(info) => {
                            let desc = if info.description.trim().is_empty() {
                                None
                            } else {
                                Some(info.description.clone())
                            };
                            (info.member_count, desc)
                        }
                        Err(_) => (0, None),
                    };

                ChatInfo {
                    title,
                    chat_type: ChatType::Group,
                    status_line: ChatSubtitle::Members(member_count).format(chrono::Local::now()),
                    username: None,
                    description,
                }
            }
            _ => ChatInfo {
                title,
                chat_type: ChatType::Group,
                status_line: String::new(),
                username: None,
                description: None,
            },
        }
    }

    fn resolve_channel_info(
        &self,
        td_type: &tdlib_rs::enums::ChatType,
        title: String,
    ) -> crate::domain::chat_info_state::ChatInfo {
        use crate::domain::chat_info_state::ChatInfo;

        if let tdlib_rs::enums::ChatType::Supergroup(sg) = td_type {
            let (subscriber_count, description) =
                match self.client.get_supergroup_full_info(sg.supergroup_id) {
                    Ok(info) => {
                        let desc = if info.description.trim().is_empty() {
                            None
                        } else {
                            Some(info.description.clone())
                        };
                        (info.member_count, desc)
                    }
                    Err(_) => (0, None),
                };

            ChatInfo {
                title,
                chat_type: ChatType::Channel,
                status_line: ChatSubtitle::Subscribers(subscriber_count)
                    .format(chrono::Local::now()),
                username: None,
                description,
            }
        } else {
            ChatInfo {
                title,
                chat_type: ChatType::Channel,
                status_line: String::new(),
                username: None,
                description: None,
            }
        }
    }
}
