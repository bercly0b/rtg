//! Chat subtitle — status line shown below the chat title.
//!
//! Mirrors what Telegram desktop/mobile shows under the chat name:
//! - Private chats: online status or last seen time
//! - Bots: "bot"
//! - Groups: "N members"
//! - Channels: "N subscribers"

use super::local_time::LocalTime;

/// Subtitle displayed in the chat header next to the title.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ChatSubtitle {
    /// No subtitle available yet.
    #[default]
    None,
    /// User is currently online.
    Online,
    /// User was last seen at a specific Unix timestamp.
    LastSeen(i32),
    /// User was online recently (hidden exact time).
    Recently,
    /// User was online within the last week.
    WithinWeek,
    /// User was online within the last month.
    WithinMonth,
    /// User's status has never been set.
    LongTimeAgo,
    /// Chat partner is a bot.
    Bot,
    /// Group with N members.
    Members(i32),
    /// Channel with N subscribers.
    Subscribers(i32),
}

impl ChatSubtitle {
    /// Formats the subtitle into a human-readable `String`.
    ///
    /// Produces exact output including dynamic timestamps
    /// like "last seen today at 14:03".
    pub fn format(&self, now: LocalTime) -> String {
        match self {
            Self::None => String::new(),
            Self::Online => "online".to_owned(),
            Self::Recently => "last seen recently".to_owned(),
            Self::WithinWeek => "last seen within a week".to_owned(),
            Self::WithinMonth => "last seen within a month".to_owned(),
            Self::LongTimeAgo => "last seen a long time ago".to_owned(),
            Self::Bot => "bot".to_owned(),
            Self::Members(n) => format_members(*n),
            Self::Subscribers(n) => format_subscribers(*n),
            Self::LastSeen(ts) => format_last_seen(*ts, now),
        }
    }
}

fn format_members(n: i32) -> String {
    if n == 1 {
        "1 member".to_owned()
    } else {
        format!("{} members", n)
    }
}

fn format_subscribers(n: i32) -> String {
    if n == 1 {
        "1 subscriber".to_owned()
    } else {
        format!("{} subscribers", n)
    }
}

fn format_last_seen(unix_ts: i32, now: LocalTime) -> String {
    let Some(seen) = LocalTime::from_unix_seconds(unix_ts as i64) else {
        return "last seen a long time ago".to_owned();
    };
    let today = now.date();
    let seen_date = seen.date();
    let time_str = seen.format("%H:%M");

    if seen_date == today {
        format!("last seen today at {}", time_str)
    } else if seen_date == today.previous_day().unwrap_or(today) {
        format!("last seen yesterday at {}", time_str)
    } else {
        format!("last seen {}", seen.format("%d.%m.%Y"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> LocalTime {
        LocalTime::from_local(2026, 3, 22, 15, 30)
    }

    #[test]
    fn none_formats_empty() {
        assert_eq!(ChatSubtitle::None.format(fixed_now()), "");
    }

    #[test]
    fn online_formats_correctly() {
        assert_eq!(ChatSubtitle::Online.format(fixed_now()), "online");
    }

    #[test]
    fn bot_formats_correctly() {
        assert_eq!(ChatSubtitle::Bot.format(fixed_now()), "bot");
    }

    #[test]
    fn recently_formats_correctly() {
        assert_eq!(
            ChatSubtitle::Recently.format(fixed_now()),
            "last seen recently"
        );
    }

    #[test]
    fn within_week_formats_correctly() {
        assert_eq!(
            ChatSubtitle::WithinWeek.format(fixed_now()),
            "last seen within a week"
        );
    }

    #[test]
    fn within_month_formats_correctly() {
        assert_eq!(
            ChatSubtitle::WithinMonth.format(fixed_now()),
            "last seen within a month"
        );
    }

    #[test]
    fn long_time_ago_formats_correctly() {
        assert_eq!(
            ChatSubtitle::LongTimeAgo.format(fixed_now()),
            "last seen a long time ago"
        );
    }

    #[test]
    fn members_singular() {
        assert_eq!(ChatSubtitle::Members(1).format(fixed_now()), "1 member");
    }

    #[test]
    fn members_plural() {
        assert_eq!(ChatSubtitle::Members(42).format(fixed_now()), "42 members");
    }

    #[test]
    fn subscribers_singular() {
        assert_eq!(
            ChatSubtitle::Subscribers(1).format(fixed_now()),
            "1 subscriber"
        );
    }

    #[test]
    fn subscribers_plural() {
        assert_eq!(
            ChatSubtitle::Subscribers(1000).format(fixed_now()),
            "1000 subscribers"
        );
    }

    fn local_unix(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> i32 {
        LocalTime::from_local(year, month, day, hour, minute).unix_seconds() as i32
    }

    #[test]
    fn last_seen_today() {
        let seen = local_unix(2026, 3, 22, 9, 5);

        assert_eq!(
            ChatSubtitle::LastSeen(seen).format(fixed_now()),
            "last seen today at 09:05"
        );
    }

    #[test]
    fn last_seen_yesterday() {
        let seen = local_unix(2026, 3, 21, 23, 59);

        assert_eq!(
            ChatSubtitle::LastSeen(seen).format(fixed_now()),
            "last seen yesterday at 23:59"
        );
    }

    #[test]
    fn last_seen_old_date() {
        let seen = local_unix(2026, 3, 5, 12, 0);

        assert_eq!(
            ChatSubtitle::LastSeen(seen).format(fixed_now()),
            "last seen 05.03.2026"
        );
    }
}
