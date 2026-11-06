//! The Group template: basic groups, supergroups and forums.

use tdlib_rs::enums::ChatMemberStatus;
use tdlib_rs::types::{
    ChatAdministratorRights, ChatLocation, ChatMemberStatusAdministrator, ChatMemberStatusCreator,
    Location,
};

use super::*;
use crate::domain::chat_info_state::{
    Badge, ChatInfoKind, GroupInfo, GroupKind, Joining, MemberRole,
};

fn group_info(info: ChatInfo) -> GroupInfo {
    match info.kind {
        ChatInfoKind::Group(group) => group,
        other => panic!("expected the Group template, got {other:?}"),
    }
}

fn admin(custom_title: &str) -> ChatMemberStatus {
    ChatMemberStatus::Administrator(ChatMemberStatusAdministrator {
        custom_title: custom_title.to_owned(),
        can_be_edited: false,
        rights: ChatAdministratorRights {
            can_manage_chat: true,
            can_change_info: false,
            can_post_messages: false,
            can_edit_messages: false,
            can_delete_messages: false,
            can_invite_users: false,
            can_restrict_members: false,
            can_pin_messages: false,
            can_manage_topics: false,
            can_promote_members: false,
            can_manage_video_chats: false,
            can_post_stories: false,
            can_edit_stories: false,
            can_delete_stories: false,
            can_manage_direct_messages: false,
            is_anonymous: false,
        },
    })
}

fn owner() -> ChatMemberStatus {
    ChatMemberStatus::Creator(ChatMemberStatusCreator {
        custom_title: String::new(),
        is_anonymous: false,
        is_member: true,
    })
}

fn resolver_with_supergroup(group: Supergroup, full_info: SupergroupFullInfo) -> FakeResolver {
    let mut resolver = FakeResolver::default();
    resolver
        .chats
        .insert(6, supergroup_chat(6, group.id, false));
    resolver.supergroup_full_infos.insert(group.id, full_info);
    resolver.supergroups.insert(group.id, group);
    resolver
}

#[test]
fn supergroup_template_fills_every_row() {
    let group = Supergroup {
        usernames: usernames(&["rust_talks"], &[]),
        date: 1_714_521_600,
        status: admin("Moderator"),
        is_forum: true,
        verification_status: verified(),
        ..supergroup(60, false)
    };
    let full = SupergroupFullInfo {
        administrator_count: 12,
        linked_chat_id: 7,
        slow_mode_delay: 30,
        is_all_history_available: false,
        invite_link: invite_link("https://t.me/+abc"),
        location: Some(ChatLocation {
            location: Location {
                latitude: 52.5,
                longitude: 13.4,
                horizontal_accuracy: 0.0,
            },
            address: "Berlin".to_owned(),
        }),
        ..supergroup_full_info(1234, "All about Rust")
    };
    let mut resolver = resolver_with_supergroup(group, full);
    resolver.chats.get_mut(&6).unwrap().has_protected_content = true;
    resolver.chats.insert(7, make_test_chat(7, "Rust News"));

    let info = resolve(&resolver, 6, ChatType::Group).unwrap();

    assert_eq!(info.status_line, "1234 members");
    assert_eq!(
        group_info(info),
        GroupInfo {
            link: Some("rust_talks".into()),
            kind: GroupKind::Forum,
            admins: Some(12),
            linked_channel: Some("Rust News".into()),
            slow_mode_delay: Some(30),
            joining: Some(Joining::Open),
            history_visible: Some(false),
            has_protected_content: true,
            invite_link: Some("https://t.me/+abc".into()),
            badges: vec![Badge::Verified],
            description: Some("All about Rust".into()),
            role: Some(MemberRole::Admin {
                custom_title: "Moderator".into(),
            }),
            joined: Some(1_714_521_600),
            location: Some("Berlin".into()),
        }
    );
}

#[test]
fn basic_group_template_counts_admins_among_members() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(5, basic_group_chat(5, 50));
    resolver.basic_group_full_infos.insert(
        50,
        BasicGroupFullInfo {
            invite_link: invite_link("https://t.me/+team"),
            ..basic_group_full_info(vec![owner(), admin(""), member_status()], "Our team")
        },
    );
    resolver.basic_groups.insert(50, basic_group(50, owner()));

    let info = resolve(&resolver, 5, ChatType::Group).unwrap();

    assert_eq!(info.status_line, "3 members");
    assert_eq!(
        group_info(info),
        GroupInfo {
            link: None,
            kind: GroupKind::BasicGroup,
            admins: Some(2),
            linked_channel: None,
            slow_mode_delay: None,
            joining: Some(Joining::InviteOnly),
            history_visible: None,
            has_protected_content: false,
            invite_link: Some("https://t.me/+team".into()),
            badges: vec![],
            description: Some("Our team".into()),
            role: Some(MemberRole::Owner),
            joined: None,
            location: None,
        }
    );
}

#[test]
fn basic_group_failure_leaves_the_role_unknown() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(5, basic_group_chat(5, 50));
    resolver
        .basic_group_full_infos
        .insert(50, basic_group_full_info(vec![member_status()], ""));

    let info = group_info(resolve(&resolver, 5, ChatType::Group).unwrap());

    assert_eq!(info.role, None);
    assert_eq!(info.admins, None);
    assert_eq!(info.description, None);
}

#[test]
fn supergroup_failure_leaves_its_rows_unknown() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(6, supergroup_chat(6, 60, false));
    resolver.supergroup_full_infos.insert(
        60,
        SupergroupFullInfo {
            linked_chat_id: 7,
            ..supergroup_full_info(42, "")
        },
    );

    let info = resolve(&resolver, 6, ChatType::Group).unwrap();

    assert_eq!(info.status_line, "42 members");
    let info = group_info(info);
    assert_eq!(info.link, None);
    assert_eq!(info.kind, GroupKind::Supergroup);
    assert_eq!(
        info.linked_channel, None,
        "the linked chat request failed too"
    );
    assert_eq!(info.joining, None);
    assert_eq!(info.badges, vec![]);
    assert_eq!(info.role, None);
    assert_eq!(info.joined, None);
}

#[test]
fn joining_follows_join_requests_and_the_public_username() {
    let cases = [
        (true, None, Joining::ByRequest),
        (false, Some("public"), Joining::Open),
        (false, None, Joining::InviteOnly),
    ];
    for (join_by_request, username, expected) in cases {
        let group = Supergroup {
            join_by_request,
            usernames: username.and_then(|u| usernames(&[u], &[])),
            ..supergroup(60, false)
        };
        let resolver = resolver_with_supergroup(group, supergroup_full_info(1, ""));

        let info = group_info(resolve(&resolver, 6, ChatType::Group).unwrap());

        assert_eq!(info.joining, Some(expected));
    }
}

#[test]
fn broadcast_group_kind() {
    let group = Supergroup {
        is_broadcast_group: true,
        ..supergroup(60, false)
    };
    let resolver = resolver_with_supergroup(group, supergroup_full_info(1, ""));

    let info = group_info(resolve(&resolver, 6, ChatType::Group).unwrap());

    assert_eq!(info.kind, GroupKind::BroadcastGroup);
}

#[test]
fn joined_is_unknown_when_the_user_is_not_a_member() {
    let group = Supergroup {
        date: 1_600_000_000,
        status: ChatMemberStatus::Left,
        ..supergroup(60, false)
    };
    let resolver = resolver_with_supergroup(group, supergroup_full_info(1, ""));

    let info = group_info(resolve(&resolver, 6, ChatType::Group).unwrap());

    assert_eq!(info.role, Some(MemberRole::Left));
    assert_eq!(info.joined, None);
}
