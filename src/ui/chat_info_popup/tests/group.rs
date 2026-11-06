//! The Group template.

use super::*;
use crate::domain::chat_info_state::{GroupInfo, GroupKind, Joining, MemberRole};

const JOINED: i32 = 1_714_560_000;

fn basic_group() -> GroupInfo {
    GroupInfo {
        link: None,
        kind: GroupKind::BasicGroup,
        admins: None,
        linked_channel: None,
        slow_mode_delay: None,
        joining: Some(Joining::InviteOnly),
        history_visible: None,
        has_protected_content: false,
        invite_link: None,
        badges: vec![],
        description: None,
        role: Some(MemberRole::Owner),
        joined: None,
        location: None,
    }
}

#[test]
fn group_template_shows_every_block_in_order() {
    let group = GroupInfo {
        link: Some("rust_talks".into()),
        kind: GroupKind::Forum,
        admins: Some(12),
        linked_channel: Some("Rust News".into()),
        slow_mode_delay: Some(30),
        joining: Some(Joining::ByRequest),
        history_visible: Some(false),
        has_protected_content: true,
        description: Some("Line one\nLine two".into()),
        role: Some(MemberRole::Admin {
            custom_title: "Moderator".into(),
        }),
        joined: Some(JOINED),
        location: Some("Berlin".into()),
        ..basic_group()
    };
    let state = loaded("1234 members", ChatInfoKind::Group(group));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "1234 members".into(),
            String::new(),
            "Group".into(),
            row("Link", "t.me/rust_talks"),
            row("Kind", "supergroup, forum"),
            row("Admins", "12"),
            row("Channel", "Rust News"),
            row("Slow mode", "30 s"),
            row("Joining", "by request"),
            row("History", "hidden from new members"),
            row("Forwarding", "restricted"),
            row("Invite link", "—"),
            row("Badges", "—"),
            row("Description", "Line one"),
            continuation("Line two"),
            String::new(),
            "You".into(),
            row("Role", "admin \"Moderator\""),
            row("Joined", &format::date(JOINED)),
            String::new(),
            "Location".into(),
            row("Address", "Berlin"),
        ])
    );
}

#[test]
fn basic_group_rows_stay_with_placeholders_without_a_location_block() {
    let state = loaded("3 members", ChatInfoKind::Group(basic_group()));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "3 members".into(),
            String::new(),
            "Group".into(),
            row("Link", "—"),
            row("Kind", "basic group"),
            row("Admins", "—"),
            row("Channel", "—"),
            row("Slow mode", "—"),
            row("Joining", "invite only"),
            row("History", "—"),
            row("Forwarding", "allowed"),
            row("Invite link", "—"),
            row("Badges", "—"),
            row("Description", "—"),
            String::new(),
            "You".into(),
            row("Role", "owner"),
            row("Joined", "—"),
        ])
    );
}

#[test]
fn long_description_wraps_in_the_value_column() {
    let group = GroupInfo {
        description: Some("word ".repeat(12).trim_end().to_owned()),
        ..basic_group()
    };
    let state = loaded("3 members", ChatInfoKind::Group(group));

    let lines = rendered_lines(&state);

    let description = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Description"))
        .unwrap();
    assert_eq!(
        lines[description..description + 2],
        [
            row("Description", "word ".repeat(8).trim_end()),
            continuation("word ".repeat(4).trim_end()),
        ]
    );
}
