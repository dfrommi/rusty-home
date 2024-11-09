use std::ops::{Add, AddAssign, Sub, SubAssign};

use super::{Duration, Time};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct DateTime {
    pub(super) delegate: chrono::DateTime<chrono::Utc>,
}

impl DateTime {
    pub(super) fn new(delegate: chrono::DateTime<chrono::Utc>) -> Self {
        Self { delegate }
    }

    pub fn now() -> Self {
        Self::new(chrono::Utc::now())
    }

    pub fn from_iso(iso8601: &str) -> anyhow::Result<Self> {
        Ok(Self::new(
            chrono::DateTime::parse_from_rfc3339(iso8601)?.with_timezone(&chrono::Utc),
        ))
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

        Ok(DateTime::new(dt))
    }

    pub fn elapsed_since(&self, since: Self) -> Duration {
        Duration::new(self.delegate - since.delegate)
    }

    pub fn into_db(&self) -> chrono::DateTime<chrono::Utc> {
        self.delegate
    }

    pub fn from_db(delegate: chrono::DateTime<chrono::Utc>) -> Self {
        Self::new(delegate)
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

impl Sub<DateTime> for DateTime {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration::new(self.delegate - rhs.delegate)
    }
}

impl AddAssign<Duration> for DateTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.delegate += rhs.delegate;
    }
}

impl SubAssign<Duration> for DateTime {
    fn sub_assign(&mut self, rhs: Duration) {
        self.delegate -= rhs.delegate;
    }
}

impl From<DateTime> for f64 {
    fn from(val: DateTime) -> Self {
        (val.delegate.timestamp_millis() as f64) / 1000.0
    }
}
