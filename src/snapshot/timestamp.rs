use std::fmt::Display;

#[derive(PartialEq, PartialOrd, Clone)]
pub struct Timestamp {
    inner: chrono::DateTime<chrono::Local>,
}

impl Timestamp {
    pub fn now() -> Self {
        Self {
            inner: chrono::offset::Local::now(),
        }
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
