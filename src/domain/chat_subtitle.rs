//! Chat subtitle — status line shown below the chat title.
//!
//! Mirrors what Telegram desktop/mobile shows under the chat name:
//! - Private chats: online status or last seen time
//! - Bots: "bot"
//! - Groups: "N members"
//! - Channels: "N subscribers"

use chrono::{DateTime, Local};

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
    pub fn format(&self, now: DateTime<Local>) -> String {
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

fn format_last_seen(unix_ts: i32, now: DateTime<Local>) -> String {
    let Some(seen_utc) = DateTime::from_timestamp(unix_ts as i64, 0) else {
        return "last seen a long time ago".to_owned();
    };
    let seen = seen_utc.with_timezone(&Local);
    let today = now.date_naive();
    let seen_date = seen.date_naive();
    let time_str = seen.format("%H:%M").to_string();

    if seen_date == today {
        format!("last seen today at {}", time_str)
    } else if seen_date == today.pred_opt().unwrap_or(today) {
        format!("last seen yesterday at {}", time_str)
    } else {
        format!("last seen {}", seen.format("%d.%m.%Y"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_now() -> DateTime<Local> {
        // 2026-03-22 15:30:00 local
        Local.with_ymd_and_hms(2026, 3, 22, 15, 30, 0).unwrap()
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

    #[test]
    fn last_seen_today() {
        let now = fixed_now();
        // 2 hours ago in UTC
        let ts = (now.timestamp() - 7200) as i32;
        let result = ChatSubtitle::LastSeen(ts).format(now);
        assert!(result.starts_with("last seen today at"), "got: {result}");
    }

    #[test]
    fn last_seen_yesterday() {
        let now = fixed_now();
        // 24 + 2 hours ago
        let ts = (now.timestamp() - 93600) as i32;
        let result = ChatSubtitle::LastSeen(ts).format(now);
        assert!(
            result.starts_with("last seen yesterday at"),
            "got: {result}"
        );
    }

    #[test]
    fn last_seen_old_date() {
        let now = fixed_now();
        // A week ago
        let ts = (now.timestamp() - 7 * 86400) as i32;
        let result = ChatSubtitle::LastSeen(ts).format(now);
        assert!(result.starts_with("last seen "), "got: {result}");
        assert!(!result.contains("today"), "got: {result}");
        assert!(!result.contains("yesterday"), "got: {result}");
    }
}
