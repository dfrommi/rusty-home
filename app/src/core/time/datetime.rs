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

    pub fn now() -> Self {
        FIXED_NOW
            .try_with(|t| *t)
            .unwrap_or_else(|_| chrono::Local::now().into())
    }

    pub fn real_now() -> Self {
        chrono::Local::now().into()
    }

    pub fn millis(&self) -> i64 {
        self.delegate.timestamp_millis()
    }

    pub fn min_value() -> Self {
        chrono::DateTime::<chrono::Local>::MIN_UTC.into()
    }

    pub fn max_value() -> Self {
        chrono::DateTime::<chrono::Local>::MAX_UTC.into()
    }

    pub async fn eval_timeshifted<F, T>(&self, f: F) -> T
    where
        F: Future<Output = T>,
    {
        FIXED_NOW.scope(*self, f).await
    }

    pub fn is_shifted() -> bool {
        FIXED_NOW.try_with(|_| ()).is_ok()
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

    pub fn at(&self, time: Time) -> Self {
        match self.delegate.with_time(time.delegate) {
            chrono::LocalResult::Single(dt) => dt.into(),
            chrono::LocalResult::Ambiguous(early, _late) => early.into(), // Winter DST: choose earlier time
            chrono::LocalResult::None => {
                // Spring forward: time doesn't exist, try next minute recursively
                self.at(time + Duration::minutes(1))
            }
        }
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

    pub fn is_passed(&self) -> bool {
        *self < Self::now()
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

    #[test]
    fn test_on_next_prev_day_roundtrip() {
        let dt = DateTime::from_iso("2024-06-15T14:30:00Z").unwrap();
        let roundtrip = dt.on_next_day().on_prev_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_on_prev_next_day_roundtrip() {
        let dt = DateTime::from_iso("2024-06-15T14:30:00Z").unwrap();
        let roundtrip = dt.on_prev_day().on_next_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_on_next_prev_day_spring_forward() {
        // Day before spring forward transition
        let dt = DateTime::from_iso("2025-03-29T02:30:00+01:00").unwrap();
        let roundtrip = dt.on_next_day().on_prev_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_on_next_prev_day_fall_back() {
        // Day before fall back transition
        let dt = DateTime::from_iso("2024-10-26T02:30:00+02:00").unwrap();
        let roundtrip = dt.on_next_day().on_prev_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_on_next_prev_day_from_dst_transition() {
        // From actual DST transition day (spring forward)
        let dt = DateTime::from_iso("2025-03-30T01:30:00+01:00").unwrap();
        let roundtrip = dt.on_next_day().on_prev_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_on_next_prev_day_from_fall_back_transition() {
        // From actual DST transition day (fall back)
        let dt = DateTime::from_iso("2024-10-27T01:30:00+01:00").unwrap();
        let roundtrip = dt.on_next_day().on_prev_day();
        assert_eq!(dt, roundtrip);
    }

    #[test]
    fn test_at_normal_time() {
        let dt = DateTime::from_iso("2024-06-15T12:00:00+02:00").unwrap();
        let result = dt.at(Time::at(14, 30).unwrap());
        assert_eq!(result, DateTime::from_iso("2024-06-15T14:30:00+02:00").unwrap());
    }

    #[test]
    fn test_at_spring_forward_nonexistent_time() {
        // March 30, 2025 is a spring forward day in Central Europe (2:00-3:00 doesn't exist)
        let dt = DateTime::from_iso("2025-03-30T12:00:00+02:00").unwrap();
        let result = dt.at(Time::at(2, 30).unwrap()); // This time doesn't exist

        // Should jump to 3:00 (first valid time after DST gap)
        assert_eq!(result, DateTime::from_iso("2025-03-30T03:00:00+02:00").unwrap());
    }

    #[test]
    fn test_at_fall_back_ambiguous_time() {
        // October 27, 2024 is a fall back day in Central Europe (2:30 exists twice)
        let dt = DateTime::from_iso("2024-10-27T12:00:00+01:00").unwrap();
        let result = dt.at(Time::at(2, 30).unwrap());

        // Should choose the earlier occurrence (before the clock falls back)
        assert_eq!(result, DateTime::from_iso("2024-10-27T02:30:00+01:00").unwrap());
    }

    #[test]
    fn test_at_midnight() {
        let dt = DateTime::from_iso("2024-06-15T12:00:00+02:00").unwrap();
        let result = dt.at(Time::at(0, 0).unwrap());
        assert_eq!(result, DateTime::from_iso("2024-06-15T00:00:00+02:00").unwrap());
    }

    #[test]
    fn test_at_end_of_day() {
        let dt = DateTime::from_iso("2024-06-15T12:00:00+02:00").unwrap();
        let result = dt.at(Time::at(23, 59).unwrap());
        assert_eq!(result, DateTime::from_iso("2024-06-15T23:59:00+02:00").unwrap());
    }

    #[test]
    fn test_at_preserves_date() {
        let dt = DateTime::from_iso("2024-12-25T12:00:00+01:00").unwrap();
        let result = dt.at(Time::at(8, 15).unwrap());
        assert_eq!(result, DateTime::from_iso("2024-12-25T08:15:00+01:00").unwrap());
    }
}
