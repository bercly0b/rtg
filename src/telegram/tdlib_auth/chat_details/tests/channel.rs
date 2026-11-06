//! The Channel template.

use super::*;
use crate::domain::chat_info_state::{Badge, ChannelInfo, ChatInfoKind, MemberRole};

fn channel_info(info: ChatInfo) -> ChannelInfo {
    match info.kind {
        ChatInfoKind::Channel(channel) => channel,
        other => panic!("expected the Channel template, got {other:?}"),
    }
}

#[test]
fn channel_template_fills_every_row() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, supergroup_chat(7, 70, true));
    resolver.chats.insert(6, make_test_chat(6, "Rust Talks"));
    resolver.supergroups.insert(
        70,
        Supergroup {
            usernames: usernames(&["rust_news"], &[]),
            date: 1_673_308_800,
            sign_messages: true,
            verification_status: verified(),
            ..supergroup(70, true)
        },
    );
    resolver.supergroup_full_infos.insert(
        70,
        SupergroupFullInfo {
            linked_chat_id: 6,
            administrator_count: 3,
            ..supergroup_full_info(12_345, "News about Rust")
        },
    );

    let info = resolve(&resolver, 7, ChatType::Channel).unwrap();

    assert_eq!(info.status_line, "12345 subscribers");
    assert_eq!(
        channel_info(info),
        ChannelInfo {
            link: Some("rust_news".into()),
            discussion: Some("Rust Talks".into()),
            signatures: Some(true),
            has_protected_content: false,
            admins: Some(3),
            badges: vec![Badge::Verified],
            description: Some("News about Rust".into()),
            role: Some(MemberRole::Member),
            joined: Some(1_673_308_800),
        }
    );
}

#[test]
fn channel_supergroup_failure_leaves_its_rows_unknown() {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(7, supergroup_chat(7, 70, true));
    resolver
        .supergroup_full_infos
        .insert(70, supergroup_full_info(10, ""));

    let info = resolve(&resolver, 7, ChatType::Channel).unwrap();

    assert_eq!(info.status_line, "10 subscribers");
    assert_eq!(
        channel_info(info),
        ChannelInfo {
            link: None,
            discussion: None,
            signatures: None,
            has_protected_content: false,
            admins: None,
            badges: vec![],
            description: None,
            role: None,
            joined: None,
        }
    );
}
