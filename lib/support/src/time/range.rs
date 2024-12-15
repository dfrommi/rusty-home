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

    pub fn starting_today(&self) -> DateTimeRange {
        let now = DateTime::now();
        //TODO could crash with DST -> fallback to duration between start and end
        let start = now.at(self.start).unwrap();
        let mut end = start.at(self.end).unwrap();

        if start > end {
            end = end.on_next_day();
        }

        DateTimeRange::new(start, end)
    }

    pub fn active_or_previous(&self) -> DateTimeRange {
        let now = t!(now);
        let dt_range = self.starting_today();

        if dt_range.start <= now {
            dt_range
        } else {
            DateTimeRange::new(dt_range.start.on_prev_day(), dt_range.end.on_prev_day())
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
    use crate::time::FIXED_NOW;

    use super::*;

    #[test]
    fn test_step_by() {
        let range = t!(10:03 - 10:20).starting_today();
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

    #[tokio::test]
    async fn test_active_or_previous_in_future() {
        FIXED_NOW
            .scope(t!(10:00).today(), async {
                let range = t!(13:00 - 15:00).active_or_previous();

                assert_eq!(range.start(), &t!(13:00).yesterday());
                assert_eq!(range.end(), &t!(15:00).yesterday());
            })
            .await;
    }

    #[tokio::test]
    async fn test_active_or_previous_in_past() {
        FIXED_NOW
            .scope(t!(15:00).today(), async {
                let range = t!(10:00 - 12:00).active_or_previous();

                assert_eq!(range.start(), &t!(10:00).today());
                assert_eq!(range.end(), &t!(12:00).today());
            })
            .await;
    }

    #[tokio::test]
    async fn test_active_or_previous_spread_over_days() {
        FIXED_NOW
            .scope(t!(10:00).today(), async {
                let range = t!(22:00 - 03:00).active_or_previous();

                assert_eq!(range.start(), &t!(22:00).yesterday());
                assert_eq!(range.end(), &t!(03:00).today());
            })
            .await;
    }
}
