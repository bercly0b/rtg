//! The Bot template.

use super::*;
use crate::domain::chat_info_state::{Badge, BlockState, BotGroupAccess, BotInfo, Username};

fn empty_bot() -> BotInfo {
    BotInfo {
        user_id: 987_654_321,
        usernames: vec![],
        commands: vec![],
        groups: BotGroupAccess::CannotJoin,
        inline_placeholder: None,
        privacy_policy_url: None,
        badges: vec![],
        blocked: None,
        about: None,
        description: None,
    }
}

#[test]
fn bot_template_shows_every_row_in_order() {
    let bot = BotInfo {
        usernames: vec![Username {
            name: "weather_bot".into(),
            is_collectible: false,
        }],
        commands: vec!["start".into(), "help".into(), "settings".into()],
        groups: BotGroupAccess::ReadsAllMessages,
        inline_placeholder: Some("Search city…".into()),
        privacy_policy_url: Some("https://example.com/privacy".into()),
        badges: vec![Badge::Verified],
        blocked: Some(BlockState::NotBlocked),
        about: Some("Short description".into()),
        description: Some("Long description".into()),
        ..empty_bot()
    };
    let state = loaded("bot · 120345 monthly users", ChatInfoKind::Bot(bot));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "bot · 120345 monthly users".into(),
            String::new(),
            "Bot".into(),
            row("User ID", "987654321"),
            row("Username", "@weather_bot"),
            row("Commands", "/start /help /settings"),
            row("Groups", "can join, reads all messages"),
            row("Inline", "yes, \"Search city…\""),
            row("Privacy", "https://example.com/privacy"),
            row("Badges", "verified"),
            row("Blocked", "no"),
            row("About", "Short description"),
            row("Description", "Long description"),
        ])
    );
}

#[test]
fn bot_rows_stay_with_placeholders() {
    let state = loaded("bot", ChatInfoKind::Bot(empty_bot()));

    assert_eq!(
        rendered_lines(&state),
        with_chat_block(vec![
            "Title".into(),
            "bot".into(),
            String::new(),
            "Bot".into(),
            row("User ID", "987654321"),
            row("Username", "—"),
            row("Commands", "—"),
            row("Groups", "cannot join"),
            row("Inline", "no"),
            row("Privacy", "—"),
            row("Badges", "—"),
            row("Blocked", "—"),
            row("About", "—"),
            row("Description", "—"),
        ])
    );
}

#[test]
fn many_commands_wrap_in_the_value_column() {
    let bot = BotInfo {
        commands: [
            "start", "help", "settings", "forecast", "today", "tomorrow", "week", "units",
        ]
        .map(String::from)
        .to_vec(),
        inline_placeholder: Some(String::new()),
        ..empty_bot()
    };
    let state = loaded("bot", ChatInfoKind::Bot(bot));

    let lines = rendered_lines(&state);

    let commands = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Commands"))
        .unwrap();
    assert_eq!(
        lines[commands..commands + 2],
        [
            row("Commands", "/start /help /settings /forecast /today"),
            continuation("/tomorrow /week /units"),
        ]
    );
    assert!(lines.contains(&row("Inline", "yes")));
}
