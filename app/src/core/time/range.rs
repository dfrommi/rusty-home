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
        let end = now.at(self.end);

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
        let start = reference.at(self.start);
        let mut end = start.at(self.end);

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
    fn test_active_or_previous_dst_spring_forward_range_in_gap() {
        // Spring forward: 2:00 AM jumps to 3:00 AM, so 2:15-2:45 doesn't exist
        let reference = DateTime::from_iso("2025-03-30T12:30:00+02:00").unwrap();
        let range = t!(02:15 - 02:45).active_or_previous_at(reference);

        // Both 2:15 and 2:45 get adjusted to 3:00
        assert_eq!(range.start(), range.end());
        assert_eq!(range.start().time(), t!(3:00));
    }

    #[test]
    fn test_active_or_previous_dst_spring_forward_range_ends_in_gap() {
        // Range starts before gap, ends in gap: 1:30-2:30
        let reference = DateTime::from_iso("2025-03-30T12:30:00+02:00").unwrap();
        let range = t!(01:30 - 02:30).active_or_previous_at(reference);

        // Start: 1:30 (exists), End: 2:30 -> 3:00 (adjusted)
        assert_eq!(range.start(), &reference.at(t!(01:30)));
        assert_eq!(range.end(), &reference.at(t!(03:00)));
    }

    #[test]
    fn test_active_or_previous_dst_spring_forward_cross_midnight_ends_in_gap() {
        // Cross-midnight range ending in DST gap: 23:30-2:30
        let reference = DateTime::from_iso("2025-03-30T12:30:00+02:00").unwrap();
        let range = t!(23:30 - 02:30).active_or_previous_at(reference);

        // DST handling should produce a valid range where start < end
        // End time should be adjusted due to spring forward (2:30 -> 3:00)
        assert!(range.start() < range.end());
        assert_eq!(range.end().time(), t!(3:00)); // 2:30 gets adjusted to 3:00
    }

    #[test]
    fn test_active_or_previous_dst_fall_back_range_in_ambiguous() {
        // Fall back: 3:00 AM falls back to 2:00 AM, so 2:15-2:45 exists twice
        let reference = DateTime::from_iso("2024-10-27T12:30:00+01:00").unwrap();
        let range = t!(02:15 - 02:45).active_or_previous_at(reference);

        // Should choose earlier occurrence (before the clock falls back)
        assert_eq!(range.start(), &reference.at(t!(02:15)));
        assert_eq!(range.end(), &reference.at(t!(02:45)));
    }

    #[test]
    fn test_active_or_previous_dst_fall_back_range_ends_in_ambiguous() {
        // Range starts before ambiguous time, ends in it: 1:30-2:30
        let reference = DateTime::from_iso("2024-10-27T12:30:00+01:00").unwrap();
        let range = t!(01:30 - 02:30).active_or_previous_at(reference);

        // Start: 1:30 (unambiguous), End: 2:30 (ambiguous, should pick earlier)
        assert_eq!(range.start(), &reference.at(t!(01:30)));
        assert_eq!(range.end(), &reference.at(t!(02:30)));
    }

    #[test]
    fn test_active_or_previous_dst_fall_back_cross_midnight_ends_in_ambiguous() {
        // Cross-midnight range ending in ambiguous time: 23:30-2:30
        let reference = DateTime::from_iso("2024-10-27T12:30:00+01:00").unwrap();
        let range = t!(23:30 - 02:30).active_or_previous_at(reference);

        // DST handling should produce a valid range where start < end
        // End time should be 2:30 (ambiguous time, picks earlier occurrence)
        assert!(range.start() < range.end());
        assert_eq!(range.end().time(), t!(2:30));
    }

    #[test]
    fn test_active_or_previous_normal_day_with_230() {
        // Normal day (no DST) with 2:30 end time for comparison
        let reference = DateTime::from_iso("2024-06-15T12:30:00+02:00").unwrap();
        let range = t!(01:30 - 02:30).active_or_previous_at(reference);

        // Should work normally without any DST adjustments
        assert_eq!(range.start(), &reference.at(t!(01:30)));
        assert_eq!(range.end(), &reference.at(t!(02:30)));
    }

    #[test]
    fn test_active_or_previous_dst_spring_forward_during_range() {
        // Reference time is DURING the cross-midnight range (1 AM)
        let reference = DateTime::from_iso("2025-03-30T01:00:00+01:00").unwrap();
        let range = t!(23:30 - 02:30).active_or_previous_at(reference);

        // Should get a valid range that covers the reference time
        // End time should be adjusted due to spring forward (2:30 -> 3:00)
        assert!(range.start() < range.end());
        assert!(*range.start() <= reference);
        assert!(reference <= *range.end());
        assert_eq!(range.end().time(), t!(3:00)); // 2:30 gets adjusted to 3:00
    }

    #[test]
    fn test_active_or_previous_dst_fall_back_during_range() {
        // Reference time is DURING the cross-midnight range (1 AM)
        let reference = DateTime::from_iso("2024-10-27T01:00:00+02:00").unwrap();
        let range = t!(23:30 - 02:30).active_or_previous_at(reference);

        // Should get a valid range that covers the reference time
        assert!(range.start() < range.end());
        assert!(*range.start() <= reference);
        assert!(reference <= *range.end());
        assert_eq!(range.end().time(), t!(2:30)); // 2:30 gets adjusted to 2:30
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
