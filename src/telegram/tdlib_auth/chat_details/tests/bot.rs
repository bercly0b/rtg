//! The Bot template.

use tdlib_rs::enums::BlockList;
use tdlib_rs::types::BotCommand;

use super::*;
use crate::domain::chat_info_state::{
    Badge, BlockState, BotGroupAccess, BotInfo as BotTemplate, ChatInfoKind, Username,
};

fn bot_info(info: ChatInfo) -> BotTemplate {
    match info.kind {
        ChatInfoKind::Bot(bot) => bot,
        other => panic!("expected the Bot template, got {other:?}"),
    }
}

fn resolver_with_bot(bot: User) -> FakeResolver {
    let mut resolver = FakeResolver::default();
    resolver.chats.insert(bot.id, private_chat(bot.id, bot.id));
    resolver.users.insert(bot.id, bot);
    resolver
}

fn command(name: &str) -> BotCommand {
    BotCommand {
        command: name.to_owned(),
        description: format!("{name} description"),
    }
}

#[test]
fn bot_template_fills_every_row() {
    let mut weather = bot_user(
        3,
        UserTypeBot {
            can_join_groups: true,
            is_inline: true,
            inline_query_placeholder: "Search city…".to_owned(),
            active_user_count: 120_345,
            ..bot_type()
        },
    );
    weather.usernames = usernames(&["weather_bot"], &[]);
    weather.verification_status = verified();
    let mut resolver = resolver_with_bot(weather);
    let mut full = user_full_info();
    full.block_list = Some(BlockList::Main);
    full.bot_info = Some(BotInfo {
        short_description: "Short description".to_owned(),
        description: "Long description".to_owned(),
        commands: vec![command("start"), command("help"), command("settings")],
        privacy_policy_url: "https://example.com/privacy".to_owned(),
        ..bot_details()
    });
    resolver.user_full_infos.insert(3, full);

    let info = resolve(&resolver, 3, ChatType::Private).unwrap();

    assert_eq!(info.status_line, "bot · 120345 monthly users");
    assert_eq!(
        bot_info(info),
        BotTemplate {
            user_id: 3,
            usernames: vec![Username {
                name: "weather_bot".into(),
                is_collectible: false,
            }],
            commands: vec!["start".into(), "help".into(), "settings".into()],
            groups: BotGroupAccess::CanJoin,
            inline_placeholder: Some("Search city…".into()),
            privacy_policy_url: Some("https://example.com/privacy".into()),
            badges: vec![Badge::Verified],
            blocked: Some(BlockState::Blocked),
            about: Some("Short description".into()),
            description: Some("Long description".into()),
        }
    );
}

#[test]
fn bot_status_line_omits_unknown_monthly_users() {
    let resolver = resolver_with_bot(bot_user(3, bot_type()));

    let info = resolve(&resolver, 3, ChatType::Private).unwrap();

    assert_eq!(info.status_line, "bot");
}

#[test]
fn bot_group_access_follows_join_and_privacy_mode() {
    let cases = [
        ((false, true), BotGroupAccess::CannotJoin),
        ((true, false), BotGroupAccess::CanJoin),
        ((true, true), BotGroupAccess::ReadsAllMessages),
    ];
    for ((can_join_groups, can_read_all_group_messages), expected) in cases {
        let resolver = resolver_with_bot(bot_user(
            3,
            UserTypeBot {
                can_join_groups,
                can_read_all_group_messages,
                ..bot_type()
            },
        ));

        let info = bot_info(resolve(&resolver, 3, ChatType::Private).unwrap());

        assert_eq!(info.groups, expected);
    }
}

#[test]
fn bot_full_info_failure_leaves_its_rows_unknown() {
    let resolver = resolver_with_bot(bot_user(3, bot_type()));

    let info = bot_info(resolve(&resolver, 3, ChatType::Private).unwrap());

    assert_eq!(info.commands, Vec::<String>::new());
    assert_eq!(info.inline_placeholder, None);
    assert_eq!(info.privacy_policy_url, None);
    assert_eq!(info.blocked, None);
    assert_eq!(info.about, None);
    assert_eq!(info.description, None);
}
