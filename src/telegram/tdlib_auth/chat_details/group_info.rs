//! The Group and Channel info templates.

use chrono::{DateTime, Local};
use tdlib_rs::enums::{ChatMemberStatus, ChatType as TdChatType};
use tdlib_rs::types::{Chat, Supergroup};

use crate::domain::chat::ChatType;
use crate::domain::chat_info_state::{
    ChannelInfo, ChatInfoKind, GroupInfo, GroupKind, Joining, MemberRole,
};
use crate::domain::chat_subtitle::ChatSubtitle;
use crate::usecases::chat_subtitle::ChatSubtitleError;

use super::{
    chat_title, non_empty, non_zero, optional, request_failed, unexpected_chat_type,
    verification_badges, ChatInfoResolver,
};

pub(super) fn resolve_group(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    now: DateTime<Local>,
) -> Result<(String, ChatInfoKind), ChatSubtitleError> {
    match &chat.r#type {
        TdChatType::BasicGroup(bg) => resolve_basic_group(resolver, chat, bg.basic_group_id, now),
        TdChatType::Supergroup(sg) => resolve_supergroup(resolver, chat, sg.supergroup_id, now),
        other => Err(unexpected_chat_type(ChatType::Group, other)),
    }
}

pub(super) fn resolve_channel(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    now: DateTime<Local>,
) -> Result<(String, ChatInfoKind), ChatSubtitleError> {
    let TdChatType::Supergroup(sg) = &chat.r#type else {
        return Err(unexpected_chat_type(ChatType::Channel, &chat.r#type));
    };
    let full_info = resolver
        .get_supergroup_full_info(sg.supergroup_id)
        .map_err(|e| request_failed("getSupergroupFullInfo", sg.supergroup_id, e))?;
    let channel = optional("getSupergroup", resolver.get_supergroup(sg.supergroup_id));
    let channel = channel.as_ref();

    let info = ChannelInfo {
        link: channel.and_then(public_username),
        discussion: chat_title(resolver, full_info.linked_chat_id),
        signatures: channel.map(|c| c.sign_messages),
        has_protected_content: chat.has_protected_content,
        admins: non_zero(full_info.administrator_count),
        badges: channel
            .map(|c| verification_badges(c.verification_status.as_ref()))
            .unwrap_or_default(),
        description: non_empty(&full_info.description),
        role: channel.map(|c| member_role(&c.status)),
        joined: channel.and_then(joined_date),
    };
    let status_line = ChatSubtitle::Subscribers(full_info.member_count).format(now);
    Ok((status_line, ChatInfoKind::Channel(info)))
}

fn resolve_basic_group(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    basic_group_id: i64,
    now: DateTime<Local>,
) -> Result<(String, ChatInfoKind), ChatSubtitleError> {
    let full_info = resolver
        .get_basic_group_full_info(basic_group_id)
        .map_err(|e| request_failed("getBasicGroupFullInfo", basic_group_id, e))?;
    let group = optional("getBasicGroup", resolver.get_basic_group(basic_group_id));

    let admin_count = full_info
        .members
        .iter()
        .filter(|member| {
            matches!(
                member.status,
                ChatMemberStatus::Creator(_) | ChatMemberStatus::Administrator(_)
            )
        })
        .count();
    let member_count = i32::try_from(full_info.members.len()).unwrap_or(i32::MAX);

    let info = GroupInfo {
        link: None,
        kind: Some(GroupKind::BasicGroup),
        admins: non_zero(i32::try_from(admin_count).unwrap_or(i32::MAX)),
        linked_channel: None,
        slow_mode_delay: None,
        joining: Some(Joining::InviteOnly),
        history_visible: None,
        has_protected_content: chat.has_protected_content,
        invite_link: full_info.invite_link.map(|link| link.invite_link),
        badges: Vec::new(),
        description: non_empty(&full_info.description),
        role: group.map(|g| member_role(&g.status)),
        joined: None,
        location: None,
    };
    let status_line = ChatSubtitle::Members(member_count).format(now);
    Ok((status_line, ChatInfoKind::Group(info)))
}

fn resolve_supergroup(
    resolver: &dyn ChatInfoResolver,
    chat: &Chat,
    supergroup_id: i64,
    now: DateTime<Local>,
) -> Result<(String, ChatInfoKind), ChatSubtitleError> {
    let full_info = resolver
        .get_supergroup_full_info(supergroup_id)
        .map_err(|e| request_failed("getSupergroupFullInfo", supergroup_id, e))?;
    let group = optional("getSupergroup", resolver.get_supergroup(supergroup_id));
    let group = group.as_ref();

    let info = GroupInfo {
        link: group.and_then(public_username),
        kind: group.map(supergroup_kind),
        admins: non_zero(full_info.administrator_count),
        linked_channel: chat_title(resolver, full_info.linked_chat_id),
        slow_mode_delay: Some(full_info.slow_mode_delay),
        joining: group.map(joining),
        history_visible: Some(full_info.is_all_history_available),
        has_protected_content: chat.has_protected_content,
        invite_link: full_info
            .invite_link
            .as_ref()
            .map(|link| link.invite_link.clone()),
        badges: group
            .map(|g| verification_badges(g.verification_status.as_ref()))
            .unwrap_or_default(),
        description: non_empty(&full_info.description),
        role: group.map(|g| member_role(&g.status)),
        joined: group.and_then(joined_date),
        location: full_info
            .location
            .as_ref()
            .and_then(|location| non_empty(&location.address)),
    };
    let status_line = ChatSubtitle::Members(full_info.member_count).format(now);
    Ok((status_line, ChatInfoKind::Group(info)))
}

fn public_username(supergroup: &Supergroup) -> Option<String> {
    supergroup
        .usernames
        .as_ref()
        .and_then(|usernames| usernames.active_usernames.first().cloned())
}

fn supergroup_kind(supergroup: &Supergroup) -> GroupKind {
    if supergroup.is_forum {
        GroupKind::Forum
    } else if supergroup.is_broadcast_group {
        GroupKind::BroadcastGroup
    } else {
        GroupKind::Supergroup
    }
}

fn joining(supergroup: &Supergroup) -> Joining {
    if supergroup.join_by_request {
        Joining::ByRequest
    } else if public_username(supergroup).is_some() {
        Joining::Open
    } else {
        Joining::InviteOnly
    }
}

fn member_role(status: &ChatMemberStatus) -> MemberRole {
    match status {
        ChatMemberStatus::Creator(_) => MemberRole::Owner,
        ChatMemberStatus::Administrator(admin) => MemberRole::Admin {
            custom_title: admin.custom_title.clone(),
        },
        ChatMemberStatus::Member(_) => MemberRole::Member,
        ChatMemberStatus::Restricted(restricted) => MemberRole::Restricted {
            until: restricted.restricted_until_date,
        },
        ChatMemberStatus::Left => MemberRole::Left,
        ChatMemberStatus::Banned(_) => MemberRole::Banned,
    }
}

/// `supergroup.date` is the creation date when the user is not a member.
fn joined_date(supergroup: &Supergroup) -> Option<i32> {
    let is_member = match &supergroup.status {
        ChatMemberStatus::Creator(creator) => creator.is_member,
        ChatMemberStatus::Administrator(_) | ChatMemberStatus::Member(_) => true,
        ChatMemberStatus::Restricted(restricted) => restricted.is_member,
        ChatMemberStatus::Left | ChatMemberStatus::Banned(_) => false,
    };
    if is_member {
        non_zero(supergroup.date)
    } else {
        None
    }
}
