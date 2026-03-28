use tdlib_rs::enums::UserStatus;

use crate::domain::chat_subtitle::ChatSubtitle;
use crate::telegram::tdlib_mappers::{
    format_user_name, is_user_online, map_user_status_to_subtitle,
};

use super::make_test_user;

#[test]
fn format_user_name_handles_first_name_only() {
    let user = make_test_user("John", "");
    assert_eq!(format_user_name(&user), "John");
}

#[test]
fn format_user_name_combines_first_and_last() {
    let user = make_test_user("John", "Doe");
    assert_eq!(format_user_name(&user), "John Doe");
}

#[test]
fn is_user_online_detects_online_status() {
    assert!(is_user_online(&UserStatus::Online(Default::default())));
    assert!(!is_user_online(&UserStatus::Offline(Default::default())));
    assert!(!is_user_online(&UserStatus::Recently(Default::default())));
    assert!(!is_user_online(&UserStatus::Empty));
}

#[test]
fn map_user_status_to_subtitle_online() {
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::Online(Default::default())),
        ChatSubtitle::Online
    );
}

#[test]
fn map_user_status_to_subtitle_offline() {
    let offline = tdlib_rs::types::UserStatusOffline {
        was_online: 1234567,
    };
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::Offline(offline)),
        ChatSubtitle::LastSeen(1234567)
    );
}

#[test]
fn map_user_status_to_subtitle_recently() {
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::Recently(Default::default())),
        ChatSubtitle::Recently
    );
}

#[test]
fn map_user_status_to_subtitle_last_week() {
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::LastWeek(Default::default())),
        ChatSubtitle::WithinWeek
    );
}

#[test]
fn map_user_status_to_subtitle_last_month() {
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::LastMonth(Default::default())),
        ChatSubtitle::WithinMonth
    );
}

#[test]
fn map_user_status_to_subtitle_empty() {
    assert_eq!(
        map_user_status_to_subtitle(&UserStatus::Empty),
        ChatSubtitle::LongTimeAgo
    );
}
