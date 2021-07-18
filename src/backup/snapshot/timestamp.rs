use std::{fmt::Display, ops::Sub, time::SystemTime};

use log::warn;

#[derive(PartialEq, PartialOrd, Eq, Ord, Clone, Debug)]
pub struct Timestamp {
    inner: chrono::NaiveDateTime,
}

impl Timestamp {
    pub fn now() -> Self {
        Self {
            inner: chrono::offset::Local::now().naive_local(),
        }
    }

    pub fn parse_from(str: &str) -> Option<Self> {
        let inner = chrono::NaiveDateTime::parse_from_str(str, "%Y-%m-%d_%H.%M");
        match inner {
            Ok(inner) => Some(Self { inner }),
            Err(e) => {
                warn!("Filed to parse \"{}\" as Timestamp\n{}", str, e);
                None
            }
        }
    }

    pub fn get_next(&self) -> Self {
        let next_date_time = self.inner + chrono::Duration::minutes(1);
        Self {
            inner: next_date_time,
        }
    }
}

impl Sub<chrono::Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: chrono::Duration) -> Self::Output {
        Self {
            inner: self.inner - rhs,
        }
    }
}

impl From<SystemTime> for Timestamp {
    fn from(system_time: SystemTime) -> Self {
        let local: chrono::DateTime<chrono::Local> = system_time.into();
        Self {
            inner: local.naive_local(),
        }
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use chrono::{Datelike, Timelike};
        let date = self.inner.date();
        let time = self.inner.time();
        write!(
            f,
            "{}-{:02}-{:02}_{:02}.{:02}",
            date.year(),
            date.month(),
            date.day(),
            time.hour(),
            time.minute()
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    #[test]
    fn get_timestamp_from_string() {
        let ts = Timestamp::parse_from("2021-07-15_18.34").unwrap();

        assert_eq!(ts.inner.year(), 2021);
        assert_eq!(ts.inner.month(), 7);
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
}
