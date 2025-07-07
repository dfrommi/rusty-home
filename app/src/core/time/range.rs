use std::fmt::Display;

use crate::t;

use super::{DateTime, Duration, Time};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyTimeRange {
    start: Time,
    end: Time,
}

#[derive(Debug, Clone)]
pub struct DateTimeRange {
    start: DateTime,
    end: DateTime,
}

impl Display for DailyTimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl Display for DateTimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl DailyTimeRange {
    pub fn new(start: Time, end: Time) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, time: Time) -> bool {
        if self.start <= self.end {
            //same-day scenario
            self.start <= time && time <= self.end
        } else {
            //cross-day scenario
            self.start <= time || time <= self.end
        }
    }

    pub fn duration(&self) -> Duration {
        let d = if self.end >= self.start {
            self.end.delegate.signed_duration_since(self.start.delegate)
        } else {
            // Add a full day if the end time is earlier than the start time (i.e., passing midnight)
            self.end.delegate.signed_duration_since(self.start.delegate) + chrono::Duration::days(1)
        };

        Duration::new(d)
    }

    pub fn next_end(&self) -> DateTime {
        let now = t!(now);
        let end = now.at(self.end).unwrap();

        if end >= now { end } else { end.on_next_day() }
    }

    pub fn active(&self) -> Option<DateTimeRange> {
        let now = t!(now);
        let dt_range = self.active_or_previous_at(now);

        if dt_range.contains(now) { Some(dt_range) } else { None }
    }

    pub fn active_or_previous(&self) -> DateTimeRange {
        self.active_or_previous_at(t!(now))
    }

    pub fn active_or_previous_at(&self, reference: DateTime) -> DateTimeRange {
        //TODO handle switch to winter time
        //workaround for switch to summer time by adjusting the range. Not ideal, but does the job
        let start = reference.at(self.start).or_else(|_| reference.at(t!(2:00))).unwrap();
        let mut end = start.at(self.end).or_else(|_| start.at(t!(3:00))).unwrap();

        if start > end {
            end = end.on_next_day();
        }

        if start <= reference {
            DateTimeRange::new(start, end)
        } else {
            DateTimeRange::new(start.on_prev_day(), end.on_prev_day())
        }
    }
}

impl DateTimeRange {
    pub fn new(start: DateTime, end: DateTime) -> Self {
        Self { start, end }
    }

    pub fn step_by(&self, step: Duration) -> DateTimeIterator {
        DateTimeIterator::new(self, &step)
    }

    pub fn since(start: DateTime) -> Self {
        Self::new(start, t!(now))
    }

    pub fn non_future(&self) -> Self {
        let now = t!(now);
        Self::new(now.min(self.start), now.min(self.end))
    }

    pub fn intersection_with(&self, other: &Self) -> Self {
        Self::new(self.start.max(other.start), self.end.min(other.end))
    }

    pub fn start(&self) -> &DateTime {
        &self.start
    }

    pub fn end(&self) -> &DateTime {
        &self.end
    }

    pub fn contains(&self, datetime: DateTime) -> bool {
        datetime >= self.start && datetime <= self.end
    }
}

pub struct DateTimeIterator {
    next: DateTime,
    end: DateTime,
    step: Duration,
}

impl DateTimeIterator {
    pub fn new(range: &DateTimeRange, step: &Duration) -> Self {
        Self {
            next: *range.start(),
            end: *range.end(),
            step: step.clone(),
        }
    }
}

impl Iterator for DateTimeIterator {
    type Item = DateTime;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next > self.end {
            return None;
        }

        let current = self.next;
        let next = current + self.step.clone();

        //iterator should contain the end exactly
        self.next = if current < self.end && next > self.end {
            self.end
        } else {
            next
        };

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t;

    #[test]
    fn test_step_by() {
        let range = t!(10:03 - 10:20).active_or_previous_at(t!(10:10).today());
        let mut iter = range.step_by(Duration::minutes(5));

        assert_eq!(iter.next(), Some(t!(10:03).today()));
        assert_eq!(iter.next(), Some(t!(10:08).today()));
        assert_eq!(iter.next(), Some(t!(10:13).today()));
        assert_eq!(iter.next(), Some(t!(10:18).today()));
        assert_eq!(iter.next(), Some(t!(10:20).today()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_contains_same_day() {
        let range = t!(10:00 - 14:00);

        assert!(!range.contains(t!(09:59)));
        assert!(range.contains(t!(10:00)));
        assert!(range.contains(t!(11:00)));
        assert!(range.contains(t!(14:00)));
        assert!(!range.contains(t!(14:01)));
    }

    #[test]
    fn test_contains_cross_day() {
        let range = t!(22:00 - 03:00);

        assert!(!range.contains(t!(21:59)));
        assert!(range.contains(t!(22:00)));
        assert!(range.contains(t!(23:00)));
        assert!(range.contains(t!(03:00)));
        assert!(!range.contains(t!(03:01)));
    }

    #[test]
    fn test_active_or_previous_in_future() {
        let range = t!(13:00 - 15:00).active_or_previous_at(t!(10:00).today());

        assert_eq!(range.start(), &t!(13:00).yesterday());
        assert_eq!(range.end(), &t!(15:00).yesterday());
    }

    #[test]
    fn test_active_or_previous_in_past() {
        let range = t!(10:00 - 12:00).active_or_previous_at(t!(15:00).today());

        assert_eq!(range.start(), &t!(10:00).today());
        assert_eq!(range.end(), &t!(12:00).today());
    }

    #[test]
    fn test_active_or_previous_spread_over_days() {
        let range = t!(22:00 - 03:00).active_or_previous_at(t!(10:00).today());

        assert_eq!(range.start(), &t!(22:00).yesterday());
        assert_eq!(range.end(), &t!(03:00).today());
    }

    #[test]
    fn test_active_or_previous_dst() {
        let range = t!(02:15 - 02:45).active_or_previous_at(DateTime::from_iso("2025-03-30T12:30:00+01:00").unwrap());

        assert_eq!(range.start(), &t!(2:00).today());
        assert_eq!(range.end(), &t!(3:00).today());
    }

    #[tokio::test]
    async fn test_active_or_previous_after_midnight() {
        let range = t!(22:00 - 03:00).active_or_previous_at(t!(01:00).today());

        assert_eq!(range.start(), &t!(22:00).yesterday());
        assert_eq!(range.end(), &t!(03:00).today());
    }

    #[test]
    fn test_duration_same_day() {
        let range = t!(10:00 - 12:00);

        assert_eq!(range.duration(), Duration::hours(2));
    }

    #[test]
    fn test_duration_cross_day() {
        let range = t!(22:00 - 03:00);

        assert_eq!(range.duration(), Duration::hours(5));
    }
}
