//! Dates and times in the user's time zone.
//!
//! The only module that names the date-time crate: everything else works
//! with [`LocalTime`] and [`LocalDate`].

use jiff::{civil, tz::TimeZone, Timestamp};

/// A moment, read on the clock of the system time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    timestamp: Timestamp,
    civil: civil::DateTime,
}

/// A calendar day in the system time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDate(civil::Date);

impl LocalTime {
    pub fn now() -> Self {
        Self::from_timestamp(Timestamp::now())
    }

    /// `None` when the moment is outside the supported range of years.
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        Timestamp::from_second(seconds)
            .ok()
            .map(Self::from_timestamp)
    }

    /// `None` when the moment is outside the supported range of years.
    pub fn from_unix_millis(millis: i64) -> Option<Self> {
        Timestamp::from_millisecond(millis)
            .ok()
            .map(Self::from_timestamp)
    }

    fn from_timestamp(timestamp: Timestamp) -> Self {
        Self {
            timestamp,
            civil: TimeZone::system().to_datetime(timestamp),
        }
    }

    #[cfg(test)]
    pub fn from_local(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Self {
        let civil = civil::DateTime::new(year, month, day, hour, minute, 0, 0)
            .expect("valid date and time");
        let timestamp = TimeZone::system()
            .to_timestamp(civil)
            .expect("local time in range");
        Self::from_timestamp(timestamp)
    }

    pub fn date(self) -> LocalDate {
        LocalDate(self.civil.date())
    }

    pub fn year(self) -> i16 {
        self.civil.year()
    }

    pub fn unix_seconds(self) -> i64 {
        self.timestamp.as_second()
    }

    pub fn unix_millis(self) -> i64 {
        self.timestamp.as_millisecond()
    }

    /// Formats with a `strftime` string, e.g. `%H:%M`.
    pub fn format(self, strftime: &str) -> String {
        self.civil.strftime(strftime).to_string()
    }
}

impl LocalDate {
    /// `None` for the first supported day.
    pub fn previous_day(self) -> Option<Self> {
        self.0.yesterday().ok().map(Self)
    }

    /// Formats with a `strftime` string, e.g. `%-d %b %Y`.
    pub fn format(self, strftime: &str) -> String {
        self.0.strftime(strftime).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_fields_round_trip_through_unix_time() {
        let time = LocalTime::from_local(2026, 11, 6, 9, 5);

        assert_eq!(
            LocalTime::from_unix_seconds(time.unix_seconds()),
            Some(time)
        );
        assert_eq!(LocalTime::from_unix_millis(time.unix_millis()), Some(time));
        assert_eq!(time.unix_millis(), time.unix_seconds() * 1000);
    }

    #[test]
    fn time_is_formatted_with_strftime() {
        let time = LocalTime::from_local(2026, 11, 6, 9, 5);

        assert_eq!(time.format("%Y-%m-%d %H:%M"), "2026-11-06 09:05");
        assert_eq!(time.format("%-d %b"), "6 Nov");
        assert_eq!(time.format("%Y%m%d_%H%M%S"), "20261106_090500");
        assert_eq!(time.year(), 2026);
    }

    #[test]
    fn date_is_formatted_with_strftime() {
        let date = LocalTime::from_local(2026, 2, 4, 23, 30).date();

        assert_eq!(date.format("%-d %b %Y"), "4 Feb 2026");
    }

    #[test]
    fn times_on_one_day_share_a_date() {
        let morning = LocalTime::from_local(2026, 3, 1, 0, 0);
        let evening = LocalTime::from_local(2026, 3, 1, 23, 59);
        let day_before = LocalTime::from_local(2026, 2, 28, 23, 59);

        assert_eq!(morning.date(), evening.date());
        assert_ne!(morning.date(), day_before.date());
        assert_eq!(morning.date().previous_day(), Some(day_before.date()));
    }

    #[test]
    fn moment_out_of_range_is_none() {
        assert_eq!(LocalTime::from_unix_seconds(i64::MIN), None);
        assert_eq!(LocalTime::from_unix_millis(i64::MIN), None);
        assert_eq!(LocalTime::from_unix_millis(i64::MAX), None);
    }
}
