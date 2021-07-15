use std::fmt::Display;

#[derive(PartialEq, PartialOrd, Clone)]
pub struct Timestamp {
    inner: chrono::NaiveDateTime,
}

impl Timestamp {
    pub fn now() -> Self {
        Self {
            inner: chrono::offset::Local::now().naive_local(),
        }
    }

    pub fn from(str: &str) -> Option<Self> {
        let inner = chrono::NaiveDateTime::parse_from_str(str, "%Y-%m-%d_%H.%M").ok()?;
        Some(Self { inner })
    }

    pub fn get_next(&self) -> Self {
        let next_date_time = self.inner + chrono::Duration::minutes(1);
        Self {
            inner: next_date_time,
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
        let ts = Timestamp::from("2021-07-15_18.34").unwrap();

        assert_eq!(ts.inner.year(), 2021);
        assert_eq!(ts.inner.month(), 7);
        assert_eq!(ts.inner.day(), 15);
        assert_eq!(ts.inner.hour(), 18);
        assert_eq!(ts.inner.minute(), 34);
    }

    #[test]
    fn timestamp_from_invalid_string_returns_none() {
        assert!(Timestamp::from("boo").is_none());
        assert!(Timestamp::from(" \t  2021-07-15_18.34  \t\n").is_none());
        assert!(Timestamp::from("2021-07-15 18:34").is_none());
    }
}
