use std::{fmt::Display, ops::{Add, Sub}};

use anyhow::Context;
use chrono::Timelike;

use super::{DateTime, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub(super) delegate: chrono::NaiveTime,
}

impl Time {
    pub(super) fn new(delegate: chrono::NaiveTime) -> Self {
        Self { delegate }
    }

    pub fn today(&self) -> DateTime {
        DateTime::now().at(*self)
    }

    pub fn yesterday(&self) -> DateTime {
        DateTime::now().at(*self).on_prev_day()
    }

    pub fn at(hour: u32, minute: u32) -> anyhow::Result<Self> {
        Ok(Self {
            delegate: chrono::NaiveTime::from_hms_opt(hour, minute, 0)
                .context(format!("Error parsing time {hour}:{minute}"))?,
        })
    }

    pub fn hour(&self) -> u32 {
        self.delegate.hour()
    }

    pub fn minute(&self) -> u32 {
        self.delegate.minute()
    }

    pub fn add_minutes(&self, minutes: u32) -> Self {
        let total_minutes = (self.delegate.hour() * 60 + self.delegate.minute() + minutes) % (24 * 60);
        let new_hour = total_minutes / 60;
        let new_minute = total_minutes % 60;
        Self::at(new_hour, new_minute).unwrap()
    }

    fn add_seconds(&self, seconds: i64) -> Self {
        let current_seconds = self.delegate.num_seconds_from_midnight() as i64;
        let total_seconds = current_seconds + seconds;
        
        // Use rem_euclid to handle negative values properly - it always returns positive remainder
        // e.g., -5.rem_euclid(24) = 19, whereas -5 % 24 = -5
        let wrapped_seconds = total_seconds.rem_euclid(24 * 60 * 60) as u32;
        
        let hours = wrapped_seconds / 3600;
        let minutes = (wrapped_seconds % 3600) / 60;
        let secs = wrapped_seconds % 60;
        
        Self::new(chrono::NaiveTime::from_hms_opt(hours, minutes, secs).unwrap())
    }
}

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        self.add_seconds(rhs.delegate.num_seconds())
    }
}

impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.add_seconds(-rhs.delegate.num_seconds())
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.delegate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_duration_normal() {
        let time = Time::at(10, 30).unwrap();
        let result = time + Duration::hours(2);
        assert_eq!(result, Time::at(12, 30).unwrap());
    }

    #[test]
    fn test_add_duration_overflow_hours() {
        let time = Time::at(22, 30).unwrap();
        let result = time + Duration::hours(3);
        assert_eq!(result, Time::at(1, 30).unwrap()); // Wraps to next day
    }

    #[test]
    fn test_add_duration_minutes() {
        let time = Time::at(10, 45).unwrap();
        let result = time + Duration::minutes(30);
        assert_eq!(result, Time::at(11, 15).unwrap());
    }

    #[test]
    fn test_add_duration_overflow_minutes() {
        let time = Time::at(23, 45).unwrap();
        let result = time + Duration::minutes(30);
        assert_eq!(result, Time::at(0, 15).unwrap()); // Wraps to next day
    }

    #[test]
    fn test_sub_duration_normal() {
        let time = Time::at(15, 30).unwrap();
        let result = time - Duration::hours(2);
        assert_eq!(result, Time::at(13, 30).unwrap());
    }

    #[test]
    fn test_sub_duration_underflow() {
        let time = Time::at(1, 30).unwrap();
        let result = time - Duration::hours(3);
        assert_eq!(result, Time::at(22, 30).unwrap()); // Wraps to previous day
    }

    #[test]
    fn test_sub_duration_minutes() {
        let time = Time::at(10, 15).unwrap();
        let result = time - Duration::minutes(30);
        assert_eq!(result, Time::at(9, 45).unwrap());
    }

    #[test]
    fn test_sub_duration_underflow_minutes() {
        let time = Time::at(0, 15).unwrap();
        let result = time - Duration::minutes(30);
        assert_eq!(result, Time::at(23, 45).unwrap()); // Wraps to previous day
    }

    #[test]
    fn test_add_full_day() {
        let time = Time::at(12, 0).unwrap();
        let result = time + Duration::hours(24);
        assert_eq!(result, Time::at(12, 0).unwrap()); // Same time after full day
    }

    #[test]
    fn test_sub_full_day() {
        let time = Time::at(12, 0).unwrap();
        let result = time - Duration::hours(24);
        assert_eq!(result, Time::at(12, 0).unwrap()); // Same time after full day
    }

    #[test]
    fn test_add_multiple_days() {
        let time = Time::at(15, 30).unwrap();
        let result = time + Duration::hours(50); // 2 days + 2 hours
        assert_eq!(result, Time::at(17, 30).unwrap());
    }

    #[test]
    fn test_midnight_boundary() {
        let time = Time::at(23, 59).unwrap();
        let result = time + Duration::minutes(1);
        assert_eq!(result, Time::at(0, 0).unwrap());
    }
}
