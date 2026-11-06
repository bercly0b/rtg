//! The User template: private and secret chats with a person.

use base64::Engine;
use tdlib_rs::enums::{BlockList, ChatActionBar, UserStatus};
use tdlib_rs::types::{
    BusinessLocation, BusinessOpeningHours, ChatActionBarReportAddBlock, Location,
    UserStatusRecently,
};

use super::*;
use crate::domain::chat_info_state::{
    AccountInfo, Badge, Birthdate, BlockState, BusinessHours, BusinessInfo, ChatInfoKind,
    ContactStatus, MonthYear, SecretChatInfo, SecretChatState, UserInfo, Username,
};

fn user_info(info: ChatInfo) -> UserInfo {
    match info.kind {
        ChatInfoKind::User(user) => *user,
        other => panic!("expected the User template, got {other:?}"),
    }
}

fn resolver_with_user(user: User) -> FakeResolver {
    let mut resolver = FakeResolver::default();
    resolver
        .chats
        .insert(user.id, private_chat(user.id, user.id));
    resolver.users.insert(user.id, user);
    resolver
}

fn account_info_bar(info: tdlib_rs::types::AccountInfo) -> Option<ChatActionBar> {
    Some(ChatActionBar::ReportAddBlock(ChatActionBarReportAddBlock {
        can_unarchive: false,
        account_info: Some(info),
    }))
}

fn account_info_fields(country_code: &str) -> tdlib_rs::types::AccountInfo {
    tdlib_rs::types::AccountInfo {
        registration_month: 3,
        registration_year: 2021,
        phone_number_country_code: country_code.to_owned(),
        last_name_change_date: 1_790_000_000,
        last_photo_change_date: 1_780_000_000,
    }
}

fn business(
    location: Option<&str>,
    has_hours: bool,
    next_open_in: i32,
    next_close_in: i32,
) -> tdlib_rs::types::BusinessInfo {
    tdlib_rs::types::BusinessInfo {
        location: location.map(|address| BusinessLocation {
            location: Some(Location {
                latitude: 0.0,
                longitude: 0.0,
                horizontal_accuracy: 0.0,
            }),
            address: address.to_owned(),
        }),
        opening_hours: has_hours.then(|| BusinessOpeningHours {
            time_zone_id: "Europe/Berlin".to_owned(),
            opening_hours: vec![],
        }),
        local_opening_hours: None,
        next_open_in,
        next_close_in,
        greeting_message_settings: None,
        away_message_settings: None,
        start_page: None,
    }
}

#[test]
fn user_template_fills_every_row() {
    let mut alice = user(1);
    alice.usernames = usernames(&["alice", "alice_nft"], &["alice_nft"]);
    alice.phone_number = "79001234567".to_owned();
    alice.verification_status = verified();
    alice.is_premium = true;
    alice.is_contact = true;
    alice.is_mutual_contact = true;
    alice.is_close_friend = true;
    alice.status = UserStatus::Online(tdlib_rs::types::UserStatusOnline { expires: 0 });
    let mut resolver = resolver_with_user(alice);
    let mut full = user_full_info();
    full.birthdate = Some(tdlib_rs::types::Birthdate {
        day: 12,
        month: 3,
        year: 1990,
    });
    full.personal_chat_id = 77;
    full.bio = formatted("  Rust dev, Berlin  ");
    full.group_in_common_count = 3;
    full.outgoing_paid_message_star_count = 100;
    full.block_list = Some(BlockList::Stories);
    full.note = formatted("Met at RustConf");
    resolver.user_full_infos.insert(1, full);
    resolver
        .chats
        .insert(77, make_test_chat(77, "Alice's notes"));

    let info = resolve(&resolver, 1, ChatType::Private).unwrap();

    assert_eq!(info.title, "Title");
    assert_eq!(info.status_line, "online");
    assert_eq!(
        user_info(info),
        UserInfo {
            user_id: 1,
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
            blocked: Some(BlockState::StoriesOnly),
            note: Some("Met at RustConf".into()),
            account_info: None,
            business: None,
            secret_chat: None,
        }
    );
}

#[test]
fn unset_values_resolve_to_none_and_an_empty_phone_to_hidden() {
    let mut resolver = resolver_with_user(user(1));
    resolver.user_full_infos.insert(1, user_full_info());

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(
        info,
        UserInfo {
            user_id: 1,
            usernames: vec![],
            phone: None,
            birthdate: None,
            personal_channel: None,
            badges: vec![],
            bio: None,
            contact: ContactStatus::default(),
            groups_in_common: Some(0),
            paid_message_stars: Some(0),
            blocked: Some(BlockState::NotBlocked),
            note: None,
            account_info: None,
            business: None,
            secret_chat: None,
        }
    );
    assert!(!resolver.requested("getCountries"));
}

#[test]
fn birthdate_without_year_keeps_day_and_month() {
    let mut resolver = resolver_with_user(user(1));
    let mut full = user_full_info();
    full.birthdate = Some(tdlib_rs::types::Birthdate {
        day: 1,
        month: 12,
        year: 0,
    });
    resolver.user_full_infos.insert(1, full);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(
        info.birthdate,
        Some(Birthdate {
            day: 1,
            month: 12,
            year: None,
        })
    );
}

#[test]
fn user_full_info_failure_leaves_its_rows_unknown() {
    let mut alice = user(1);
    alice.phone_number = "79001234567".to_owned();
    let resolver = resolver_with_user(alice);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(info.phone.as_deref(), Some("79001234567"));
    assert_eq!(info.birthdate, None);
    assert_eq!(info.personal_channel, None);
    assert_eq!(info.bio, None);
    assert_eq!(info.groups_in_common, None);
    assert_eq!(info.paid_message_stars, None);
    assert_eq!(info.blocked, None);
    assert_eq!(info.note, None);
    assert_eq!(info.business, None);
}

#[test]
fn personal_channel_failure_leaves_its_row_unknown() {
    let mut resolver = resolver_with_user(user(1));
    let mut full = user_full_info();
    full.personal_chat_id = 77;
    resolver.user_full_infos.insert(1, full);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(info.personal_channel, None);
}

#[test]
fn status_line_marks_a_last_seen_hidden_by_my_settings() {
    let mut mine = user(1);
    mine.status = UserStatus::Recently(UserStatusRecently {
        by_my_privacy_settings: true,
    });
    let mut theirs = user(2);
    theirs.status = UserStatus::Recently(UserStatusRecently {
        by_my_privacy_settings: false,
    });
    let mut resolver = resolver_with_user(mine);
    resolver.chats.insert(2, private_chat(2, 2));
    resolver.users.insert(2, theirs);

    let mine = resolve(&resolver, 1, ChatType::Private).unwrap();
    let theirs = resolve(&resolver, 2, ChatType::Private).unwrap();

    assert_eq!(mine.status_line, "last seen recently (you hide yours)");
    assert_eq!(theirs.status_line, "last seen recently");
}

#[test]
fn deleted_account_status_line() {
    let mut deleted = user(1);
    deleted.r#type = tdlib_rs::enums::UserType::Deleted;
    let resolver = resolver_with_user(deleted);

    let info = resolve(&resolver, 1, ChatType::Private).unwrap();

    assert_eq!(info.status_line, "deleted account");
    assert!(matches!(info.kind, ChatInfoKind::User(_)));
}

#[test]
fn account_info_comes_from_the_action_bar_with_the_country_name() {
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.get_mut(&1).unwrap().action_bar = account_info_bar(account_info_fields("RU"));
    resolver.countries = Some(vec![country("DE", "Germany"), country("RU", "Russia")]);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(
        info.account_info,
        Some(AccountInfo {
            registered: Some(MonthYear {
                month: Some(3),
                year: 2021,
            }),
            phone_country: Some("Russia".into()),
            name_changed: Some(1_790_000_000),
            photo_changed: Some(1_780_000_000),
        })
    );
}

#[test]
fn account_info_country_falls_back_to_its_code() {
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.get_mut(&1).unwrap().action_bar = account_info_bar(account_info_fields("RU"));

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(
        info.account_info.unwrap().phone_country.as_deref(),
        Some("RU")
    );
}

#[test]
fn account_info_zero_and_empty_fields_are_unknown() {
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.get_mut(&1).unwrap().action_bar =
        account_info_bar(tdlib_rs::types::AccountInfo {
            registration_month: 0,
            registration_year: 2021,
            phone_number_country_code: String::new(),
            last_name_change_date: 0,
            last_photo_change_date: 0,
        });

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(
        info.account_info,
        Some(AccountInfo {
            registered: Some(MonthYear {
                month: None,
                year: 2021,
            }),
            phone_country: None,
            name_changed: None,
            photo_changed: None,
        })
    );
    assert!(!resolver.requested("getCountries"));
}

#[test]
fn no_account_info_once_tdlib_drops_it() {
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.get_mut(&1).unwrap().action_bar = Some(ChatActionBar::AddContact);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(info.account_info, None);
}

#[test]
fn business_block_needs_a_location_or_opening_hours() {
    let mut resolver = resolver_with_user(user(1));
    let mut full = user_full_info();
    full.business_info = Some(business(None, false, 0, 0));
    resolver.user_full_infos.insert(1, full);

    let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

    assert_eq!(info.business, None);
}

#[test]
fn business_hours_tell_whether_it_is_open_now() {
    let cases = [
        ((3600, 600), Some(BusinessHours::OpenClosesIn(600))),
        ((0, 600), Some(BusinessHours::OpenClosesIn(600))),
        ((600, 3600), Some(BusinessHours::ClosedOpensIn(600))),
        ((600, 0), Some(BusinessHours::ClosedOpensIn(600))),
        ((0, 0), None),
    ];
    for ((next_open_in, next_close_in), expected) in cases {
        let mut resolver = resolver_with_user(user(1));
        let mut full = user_full_info();
        full.business_info = Some(business(
            Some("Berlin, Main st. 1"),
            true,
            next_open_in,
            next_close_in,
        ));
        resolver.user_full_infos.insert(1, full);

        let info = user_info(resolve(&resolver, 1, ChatType::Private).unwrap());

        assert_eq!(
            info.business,
            Some(BusinessInfo {
                address: Some("Berlin, Main st. 1".into()),
                hours: expected,
            }),
            "next_open_in {next_open_in}, next_close_in {next_close_in}"
        );
    }
}

#[test]
fn secret_chat_shows_state_and_the_comparable_part_of_the_key() {
    let key: Vec<u8> = (0..36).collect();
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.insert(10, secret_chat(10, 5, 1));
    resolver.secret_chats.insert(
        5,
        SecretChat {
            id: 5,
            user_id: 1,
            state: tdlib_rs::enums::SecretChatState::Ready,
            is_outbound: true,
            key_hash: base64::engine::general_purpose::STANDARD.encode(&key),
            layer: 144,
        },
    );

    let info = user_info(resolve(&resolver, 10, ChatType::Private).unwrap());

    assert_eq!(
        info.secret_chat,
        Some(SecretChatInfo {
            state: Some(SecretChatState::Ready),
            key_hash: key[..32].to_vec(),
        })
    );
}

#[test]
fn secret_chat_failure_keeps_the_block_with_unknown_rows() {
    let mut resolver = resolver_with_user(user(1));
    resolver.chats.insert(10, secret_chat(10, 5, 1));

    let info = user_info(resolve(&resolver, 10, ChatType::Private).unwrap());

    assert_eq!(
        info.secret_chat,
        Some(SecretChatInfo {
            state: None,
            key_hash: vec![],
        })
    );
}
