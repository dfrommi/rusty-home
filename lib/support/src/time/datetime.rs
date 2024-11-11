use std::{
    fmt::Display,
    ops::{Add, Sub},
};

use tokio::task_local;

use super::{Duration, Time};

task_local! {
    pub static FIXED_NOW: DateTime;
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
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

    pub fn time(&self) -> Time {
        Time::new(self.delegate.time())
    }

    pub fn at(&self, time: Time) -> anyhow::Result<Self> {
        let dt = self
            .delegate
            .with_time(time.delegate)
            .earliest()
            .ok_or_else(|| {
                anyhow::anyhow!("Error parsing time {:?} for date-time {:?}", time, self)
            })?;

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
