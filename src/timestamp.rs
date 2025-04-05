use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, Timelike};
use chronoutil::RelativeDuration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub DateTime<Local>);

impl AsRef<DateTime<Local>> for Timestamp {
    fn as_ref(&self) -> &DateTime<Local> {
        &self.0
    }
}

impl Timestamp {
    #[must_use]
    pub fn now() -> Self {
        Self(Local::now())
    }

    pub fn parse_from_str(s: &str, format: &str) -> Result<Self> {
        let timestamp = NaiveDateTime::parse_from_str(s, format)
            .context("invalid timestamp format")?
            .and_local_timezone(Local)
            .single()
            .context("failed to convert to local timezone")?;
        Ok(Self(timestamp))
    }

    #[must_use]
    pub fn shift(&self, range: Range, amount: i32) -> Self {
        let timestamp = match range {
            Range::Minute => self.0 + Duration::minutes(amount.into()),
            Range::Hour => self.0 + Duration::hours(amount.into()),
            Range::Day => self.0 + Duration::days(amount.into()),
            Range::Month => self.0 + RelativeDuration::months(amount),
            Range::Year => self.0 + RelativeDuration::years(amount),
        };
        Self(timestamp)
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn floor(&self, range: Range) -> Self {
        let minute = self
            .0
            .with_second(0)
            .expect("second 0")
            .with_nanosecond(0)
            .expect("nanosecond 0");
        let hour = minute.with_minute(0).expect("minute 0");
        let day = hour.with_hour(0).expect("hour 0");
        let month = day.with_day(1).expect("day 1");
        let year = month.with_month(1).expect("month 1");
        match range {
            Range::Minute => Self(minute),
            Range::Hour => Self(hour),
            Range::Day => Self(day),
            Range::Month => Self(month),
            Range::Year => Self(year),
        }
    }

    #[must_use]
    pub fn humanized(&self) -> String {
        self.0.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d_%H-%M-%S"))
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Range {
    Minute,
    Hour,
    Day,
    Month,
    Year,
}
