//! How values are written.

use super::*;
use crate::domain::chat_info_state::{BusinessHours, MemberRole};
use crate::domain::message_info_state::format_unix_timestamp;

#[test]
fn periods_use_the_largest_whole_unit() {
    let cases = [
        (0, "off"),
        (10, "10 s"),
        (60, "1 min"),
        (90, "90 s"),
        (300, "5 min"),
        (3600, "1 h"),
        (86_400, "1 day"),
        (2 * 86_400, "2 days"),
        (30 * 86_400, "30 days"),
        (31 * 86_400, "1 month"),
        (365 * 86_400, "1 year"),
    ];
    for (seconds, expected) in cases {
        assert_eq!(format::off_or_period(seconds), expected, "{seconds} s");
    }
}

#[test]
fn business_hours_round_the_time_left_down() {
    assert_eq!(
        format::business_hours(BusinessHours::OpenClosesIn(3 * 3600 + 59 * 60)),
        "open, closes in 3 h"
    );
    assert_eq!(
        format::business_hours(BusinessHours::ClosedOpensIn(2 * 86_400 + 5)),
        "closed, opens in 2 days"
    );
    assert_eq!(
        format::business_hours(BusinessHours::ClosedOpensIn(45)),
        "closed, opens in 45 s"
    );
}

#[test]
fn roles_name_the_users_status() {
    let restricted_until = 1_800_000_000;
    let cases = [
        (MemberRole::Owner, "owner".to_owned()),
        (
            MemberRole::Admin {
                custom_title: String::new(),
            },
            "admin".to_owned(),
        ),
        (MemberRole::Member, "member".to_owned()),
        (MemberRole::Restricted { until: 0 }, "restricted".to_owned()),
        (
            MemberRole::Restricted {
                until: restricted_until,
            },
            format!(
                "restricted until {}",
                format_unix_timestamp(restricted_until)
            ),
        ),
        (MemberRole::Left, "left".to_owned()),
        (MemberRole::Banned, "banned".to_owned()),
    ];
    for (role, expected) in cases {
        assert_eq!(format::role(&role, "member"), expected);
    }
}

#[test]
fn key_hash_is_hex_in_pairs_of_bytes() {
    assert_eq!(
        format::key_hash(&[0x8f, 0x3a, 0x01, 0xc2, 0x4b]),
        "8f3a 01c2 4b"
    );
}

#[test]
fn contact_status_names_the_closest_relation() {
    use crate::domain::chat_info_state::ContactStatus;

    let contact = ContactStatus {
        is_contact: true,
        ..ContactStatus::default()
    };
    let stranger_friend = ContactStatus {
        is_close_friend: true,
        ..ContactStatus::default()
    };

    assert_eq!(format::contact(contact), "contact");
    assert_eq!(
        format::contact(stranger_friend),
        "not a contact, close friend"
    );
}

#[test]
fn paid_messages_count_stars() {
    assert_eq!(format::paid_messages(1), "paid, 1 star each");
}
