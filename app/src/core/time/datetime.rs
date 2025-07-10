use std::{
    fmt::Display,
    ops::{Add, Sub},
};

use tokio::task_local;

use super::{Duration, Time};

task_local! {
    pub static FIXED_NOW: DateTime;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DateTime {
    delegate: chrono::DateTime<chrono::Local>,
}

impl DateTime {
    fn new<T: chrono::TimeZone>(delegate: chrono::DateTime<T>) -> Self {
        Self {
            delegate: delegate.with_timezone(&chrono::Local),
        }
    }

    pub fn midpoint(start: &DateTime, end: &DateTime) -> DateTime {
        let start_timestamp = start.delegate.timestamp_millis();
        let end_timestamp = end.delegate.timestamp_millis();

        // Calculate the midpoint in seconds
        let mid_timestamp = (start_timestamp + end_timestamp) / 2;

        chrono::DateTime::from_timestamp_millis(mid_timestamp)
            .unwrap_or_else(|| {
                panic!("Error calculating midpoint for {start:?} and {end:?}. Should not fail with reasonable dates")
            })
            .into()
    }

    pub(super) fn delegate(&self) -> &chrono::DateTime<chrono::Local> {
        &self.delegate
    }

    pub fn now() -> Self {
        FIXED_NOW
            .try_with(|t| *t)
            .unwrap_or_else(|_| chrono::Local::now().into())
    }

    pub fn from_iso(iso8601: &str) -> anyhow::Result<Self> {
        Ok(chrono::DateTime::parse_from_rfc3339(iso8601)?.into())
    }

    pub fn to_iso_string(&self) -> String {
        self.delegate.to_rfc3339()
    }

    pub fn to_human_readable(&self) -> String {
        chrono_humanize::HumanTime::from(self.delegate).to_string()
    }

    pub fn time(&self) -> Time {
        Time::new(self.delegate.time())
    }

    pub fn at(&self, time: Time) -> anyhow::Result<Self> {
        //TODO handle DST
        let dt = self
            .delegate
            .with_time(time.delegate)
            .earliest()
            .ok_or_else(|| anyhow::anyhow!("Error parsing time {:?} for date-time {:?}", time, self))?;

        Ok(dt.into())
    }

    pub fn on_next_day(&self) -> Self {
        //failing only at the edges of what can be stored in a date-time
        self.delegate
            .checked_add_signed(chrono::Duration::days(1))
            .unwrap()
            .into()
    }

    pub fn on_prev_day(&self) -> Self {
        //failing only at the edges of what can be stored in a date-time
        self.delegate
            .checked_sub_signed(chrono::Duration::days(1))
            .unwrap()
            .into()
    }

    pub fn elapsed_since(&self, since: Self) -> Duration {
        Duration::new(self.delegate - since.delegate)
    }

    pub fn elapsed(&self) -> Duration {
        Self::now().elapsed_since(*self)
    }

    pub fn into_db(&self) -> chrono::DateTime<chrono::Local> {
        self.delegate
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.delegate)
    }
}

impl Add<Duration> for DateTime {
    type Output = DateTime;

    fn add(self, rhs: Duration) -> Self::Output {
        Self::new(self.delegate + rhs.delegate)
    }
}

impl Sub<Duration> for DateTime {
    type Output = DateTime;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self::new(self.delegate - rhs.delegate)
    }
}

impl From<DateTime> for f64 {
    fn from(val: DateTime) -> Self {
        (val.delegate.timestamp_millis() as f64) / 1000.0
    }
}

impl<T: chrono::TimeZone> From<chrono::DateTime<T>> for DateTime {
    fn from(val: chrono::DateTime<T>) -> Self {
        DateTime::new(val)
    }
}

#[cfg(test)]
mod constructor {
    use super::*;

    #[test]
    fn test_midpoint() {
        let start = DateTime::from_iso("2024-11-03T15:23:46Z").unwrap();
        let end = DateTime::from_iso("2024-11-03T17:23:46Z").unwrap();

        let midpoint = DateTime::midpoint(&start, &end);

        assert_eq!(midpoint, DateTime::from_iso("2024-11-03T16:23:46Z").unwrap());
    }

    #[test]
    fn test_midpoint_wrong_order() {
        let start = DateTime::from_iso("2024-11-03T17:23:46Z").unwrap();
        let end = DateTime::from_iso("2024-11-03T15:23:46Z").unwrap();

        let midpoint = DateTime::midpoint(&start, &end);

        assert_eq!(midpoint, DateTime::from_iso("2024-11-03T16:23:46Z").unwrap());
    }
}
