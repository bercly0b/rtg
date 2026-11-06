//! The User template.

use super::*;
use crate::domain::chat_info_state::{
    AccountInfo, Badge, Birthdate, BlockState, BusinessHours, BusinessInfo, ContactStatus,
    MonthYear, SecretChatInfo, SecretChatState, UserInfo, Username,
};
use crate::domain::message_info_state::format_unix_timestamp;

const NAME_CHANGED: i32 = 1_790_000_000;
const PHOTO_CHANGED: i32 = 1_780_000_000;

fn empty_user() -> UserInfo {
    UserInfo {
        user_id: 123_456_789,
        usernames: vec![],
        phone: None,
        birthdate: None,
        personal_channel: None,
        badges: vec![],
        bio: None,
        contact: ContactStatus::default(),
        groups_in_common: None,
        paid_message_stars: None,
        blocked: None,
        note: None,
        account_info: None,
        business: None,
        secret_chat: None,
    }
}

#[test]
fn user_template_shows_every_block_in_order() {
    let user = UserInfo {
        usernames: vec![
            Username {
                name: "alice".into(),
                is_collectible: false,
            },
            Username {
                name: "alice_nft".into(),
                is_collectible: true,
            },
        ],
        phone: Some("79001234567".into()),
        birthdate: Some(Birthdate {
            day: 12,
            month: 3,
            year: Some(1990),
        }),
        personal_channel: Some("Alice's notes".into()),
        badges: vec![Badge::Verified, Badge::Premium],
        bio: Some("Rust dev, Berlin".into()),
        contact: ContactStatus {
            is_contact: true,
            is_mutual: true,
            is_close_friend: true,
        },
        groups_in_common: Some(3),
        paid_message_stars: Some(100),
        blocked: Some(BlockState::NotBlocked),
        note: Some("Met at RustConf".into()),
        account_info: Some(AccountInfo {
            registered: Some(MonthYear {
                month: Some(3),
                year: 2021,
            }),
            phone_country: Some("Russia".into()),
            name_changed: Some(NAME_CHANGED),
            photo_changed: Some(PHOTO_CHANGED),
        }),
        business: Some(BusinessInfo {
            address: Some("Berlin, Main st. 1".into()),
            hours: Some(BusinessHours::OpenClosesIn(3 * 3600 + 120)),
        }),
        secret_chat: Some(SecretChatInfo {
            state: Some(SecretChatState::Ready),
            key_hash: vec![0x8f, 0x3a, 0x91, 0xc2, 0x4b, 0xe0],
        }),
        ..empty_user()
    };
    let state = loaded(
        "last seen recently (you hide yours)",
        ChatInfoKind::User(Box::new(user)),
    );

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "last seen recently (you hide yours)".into(),
            String::new(),
            "Profile".into(),
            row("User ID", "123456789"),
            row("Username", "@alice, @alice_nft (collectible)"),
            row("Phone", "+79001234567"),
            row("Birthday", "12 March 1990"),
            row("Channel", "Alice's notes"),
            row("Badges", "verified, premium"),
            row("Bio", "Rust dev, Berlin"),
            String::new(),
            "You & them".into(),
            row("Contact", "mutual, close friend"),
            row("In common", "3 groups"),
            row("Messages", "paid, 100 stars each"),
            row("Blocked", "no"),
            row("Note", "Met at RustConf"),
            String::new(),
            "Account info".into(),
            row("Registered", "March 2021"),
            row("Phone from", "Russia"),
            row("Name changed", &format_unix_timestamp(NAME_CHANGED)),
            row("Photo changed", &format_unix_timestamp(PHOTO_CHANGED)),
            String::new(),
            "Business".into(),
            row("Address", "Berlin, Main st. 1"),
            row("Hours", "open, closes in 3 h"),
            String::new(),
            "Secret chat".into(),
            row("State", "ready"),
            row("Key", "8f3a 91c2 4be0"),
        ])
    );
}

#[test]
fn user_rows_stay_with_placeholders_and_conditional_blocks_hide() {
    let state = loaded(
        "last seen a long time ago",
        ChatInfoKind::User(Box::new(empty_user())),
    );

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "last seen a long time ago".into(),
            String::new(),
            "Profile".into(),
            row("User ID", "123456789"),
            row("Username", "—"),
            row("Phone", "hidden"),
            row("Birthday", "—"),
            row("Channel", "—"),
            row("Badges", "—"),
            row("Bio", "—"),
            String::new(),
            "You & them".into(),
            row("Contact", "not a contact"),
            row("In common", "—"),
            row("Messages", "—"),
            row("Blocked", "—"),
            row("Note", "—"),
        ])
    );
}

#[test]
fn account_info_and_secret_chat_rows_show_placeholders_for_unknown_values() {
    let user = UserInfo {
        groups_in_common: Some(1),
        paid_message_stars: Some(0),
        blocked: Some(BlockState::StoriesOnly),
        account_info: Some(AccountInfo {
            registered: Some(MonthYear {
                month: None,
                year: 2021,
            }),
            phone_country: None,
            name_changed: None,
            photo_changed: None,
        }),
        secret_chat: Some(SecretChatInfo {
            state: None,
            key_hash: vec![],
        }),
        ..empty_user()
    };
    let state = loaded("online", ChatInfoKind::User(Box::new(user)));

    let lines = rendered_lines(&state);

    for expected in [
        row("In common", "1 group"),
        row("Messages", "free"),
        row("Blocked", "stories only"),
        row("Registered", "2021"),
        row("Phone from", "—"),
        row("Name changed", "—"),
        row("Photo changed", "—"),
        row("State", "—"),
        row("Key", "—"),
    ] {
        assert!(
            lines.contains(&expected),
            "missing {expected:?} in {lines:#?}"
        );
    }
}

#[test]
fn scam_and_fake_badges_are_red() {
    let user = UserInfo {
        badges: vec![Badge::Verified, Badge::Scam, Badge::Fake],
        ..empty_user()
    };
    let state = loaded("online", ChatInfoKind::User(Box::new(user)));

    let lines = build_info_lines(&state, VALUE_WIDTH);
    let badges = lines
        .iter()
        .find(|line| line_text(line).trim_start().starts_with("Badges"))
        .unwrap();
    let styled: Vec<(&str, bool)> = badges.spans[1..]
        .iter()
        .map(|span| {
            (
                span.content.as_ref(),
                span.style == styles::chat_info_popup_warning_style(),
            )
        })
        .collect();

    assert_eq!(
        styled,
        vec![
            ("verified", false),
            (", ", false),
            ("scam", true),
            (", ", false),
            ("fake", true),
        ]
    );
}

#[test]
fn placeholders_are_dimmed() {
    let state = loaded("online", ChatInfoKind::User(Box::new(empty_user())));

    let lines = build_info_lines(&state, VALUE_WIDTH);
    let phone = lines
        .iter()
        .find(|line| line_text(line).trim_start().starts_with("Phone"))
        .unwrap();

    assert_eq!(phone.spans[1].content, "hidden");
    assert_eq!(
        phone.spans[1].style,
        styles::chat_info_popup_placeholder_style()
    );
}
