//! The Channel template.

use super::*;
use crate::domain::chat_info_state::{Badge, ChannelInfo, MemberRole};

const JOINED: i32 = 1_673_308_800;

#[test]
fn channel_template_shows_every_block_in_order() {
    let channel = ChannelInfo {
        link: Some("rust_news".into()),
        discussion: Some("Rust Talks".into()),
        signatures: Some(true),
        has_protected_content: false,
        admins: None,
        badges: vec![Badge::Verified],
        description: Some("News about Rust".into()),
        role: Some(MemberRole::Member),
        joined: Some(JOINED),
    };
    let state = loaded("12345 subscribers", ChatInfoKind::Channel(channel));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "12345 subscribers".into(),
            String::new(),
            "Channel".into(),
            row("Link", "t.me/rust_news"),
            row("Discussion", "Rust Talks"),
            row("Signatures", "on"),
            row("Forwarding", "allowed"),
            row("Admins", "—"),
            row("Badges", "verified"),
            row("Description", "News about Rust"),
            String::new(),
            "You".into(),
            row("Role", "subscriber"),
            row("Joined", &format::date(JOINED)),
        ])
    );
}

#[test]
fn channel_rows_stay_with_placeholders() {
    let channel = ChannelInfo {
        link: None,
        discussion: None,
        signatures: None,
        has_protected_content: true,
        admins: None,
        badges: vec![],
        description: None,
        role: None,
        joined: None,
    };
    let state = loaded("10 subscribers", ChatInfoKind::Channel(channel));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "10 subscribers".into(),
            String::new(),
            "Channel".into(),
            row("Link", "—"),
            row("Discussion", "—"),
            row("Signatures", "—"),
            row("Forwarding", "restricted"),
            row("Admins", "—"),
            row("Badges", "—"),
            row("Description", "—"),
            String::new(),
            "You".into(),
            row("Role", "—"),
            row("Joined", "—"),
        ])
    );
}
