use std::{fmt::Display, ops::Sub, time::SystemTime};

use log::warn;
use time::format_description::FormatItem;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone, Debug)]
pub struct Timestamp {
    inner: time::PrimitiveDateTime,
}

impl Timestamp {
    pub fn now() -> Self {
        Self::from(time::OffsetDateTime::now_local().unwrap_or(time::OffsetDateTime::now_utc()))
    }

    pub fn parse_from(str: &str) -> Option<Self> {
        let inner = time::PrimitiveDateTime::parse(str, &Self::get_format());
        match inner {
            Ok(inner) => Some(Self { inner }),
            Err(e) => {
                warn!("Failed to parse \"{}\" as Timestamp: {}", str, e);
                None
            }
        }
    }

    pub fn is_valid(str: &str) -> bool {
        time::PrimitiveDateTime::parse(str, &Self::get_format()).is_ok()
    }

    pub fn get_next(&self) -> Self {
        let next_date_time = self.inner + time::Duration::minutes(1);
        Self {
            inner: next_date_time,
        }
    }

    fn get_format<'a>() -> Vec<FormatItem<'a>> {
        // Format: yyyy-mm-dd_hh.mm
        time::format_description::parse_borrowed::<1>("[year]-[month]-[day]_[hour].[minute]")
            .unwrap()
    }
}

impl Sub<time::Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: time::Duration) -> Self::Output {
        Self {
            inner: self.inner - rhs,
        }
    }
}

impl From<SystemTime> for Timestamp {
    fn from(system_time: SystemTime) -> Self {
        let local: time::OffsetDateTime = system_time.into();
        let local_utc = local
            .to_offset(time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC));
        Self::from(local_utc)
    }
}
impl From<time::OffsetDateTime> for Timestamp {
    fn from(offset_date_time: time::OffsetDateTime) -> Self {
        let local = time::PrimitiveDateTime::new(offset_date_time.date(), offset_date_time.time());
        Self { inner: local }
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.format(&Self::get_format()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_timestamp_from_string() {
        let ts = Timestamp::parse_from("2021-07-15_18.34").unwrap();

        assert_eq!(ts.inner.year(), 2021);
        assert_eq!(ts.inner.month() as u8, 7);
        assert_eq!(ts.inner.day(), 15);
        assert_eq!(ts.inner.hour(), 18);
        assert_eq!(ts.inner.minute(), 34);
    }

    #[test]
    fn timestamp_from_invalid_string_returns_none() {
        assert!(Timestamp::parse_from("boo").is_none());
        assert!(Timestamp::parse_from(" \t  2021-07-15_18.34  \t\n").is_none());
        assert!(Timestamp::parse_from("2021-07-15 18:34").is_none());
    }

    #[test]
    fn is_valid_works() {
        assert!(Timestamp::is_valid("2021-07-15_18.34"));
        assert!(!Timestamp::is_valid(" 2021-07-15_18.34")); // leading space
        assert!(!Timestamp::is_valid("2021-07-15_18.34\n")); // newline
        assert!(!Timestamp::is_valid("2021-07-15 18.34")); // space instead of underscore
        assert!(!Timestamp::is_valid("2021-07-15_25.34")); // invalid hour
        assert!(!Timestamp::is_valid("2021-07-15_18.61")); // invalid minute
        assert!(!Timestamp::is_valid("2021-13-15_18.34")); // invalid month
        assert!(!Timestamp::is_valid("2021-07-32_18.34")); // invalid day
    }

    #[test]
    fn comapre_timestamps() {
        let ts_1 = Timestamp::parse_from("2021-07-15_18.34").unwrap();
        let ts_2 = Timestamp::parse_from("2021-07-15_18.35").unwrap();
        let ts_3 = Timestamp::parse_from("2021-07-15_18.34").unwrap();
        let ts_now = Timestamp::now();

        assert!(ts_1 < ts_2);
        assert!(ts_1 <= ts_2);
        assert!(ts_1 <= ts_3);
        assert!(ts_2 > ts_1);
        assert!(ts_2 >= ts_1);
        assert!(ts_1 >= ts_3);
        assert!(ts_1 == ts_3);
        assert!(ts_1 != ts_2);

        assert!(ts_1 < ts_now);
        assert!(ts_2 < ts_now);
        assert!(ts_3 < ts_now);
    }

    #[test]
    fn timestamp_from_system_time() {
        let system_time_now = std::time::SystemTime::now();
        let ts_system_now = Timestamp::from(system_time_now);
        let ts_now = Timestamp::now();

        assert_eq!(ts_system_now.inner.year(), ts_now.inner.year());
        assert_eq!(ts_system_now.inner.month(), ts_now.inner.month());
        assert_eq!(ts_system_now.inner.day(), ts_now.inner.day());
        assert_eq!(ts_system_now.inner.hour(), ts_now.inner.hour());
        assert_eq!(ts_system_now.inner.minute(), ts_now.inner.minute());
    }

    #[test]
    fn next_timestamp() {
        let ts = Timestamp::parse_from("2021-07-15_18.34").unwrap();

        let ts_next = ts.get_next();
        assert_eq!(ts_next, Timestamp::parse_from("2021-07-15_18.35").unwrap());

        let ts_next = ts_next.get_next();
        assert_eq!(ts_next, Timestamp::parse_from("2021-07-15_18.36").unwrap());
    }
}
