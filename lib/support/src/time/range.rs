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

    pub fn starting_today(&self) -> DateTimeRange {
        let now = DateTime::now();
        let start = now.at(self.start).unwrap();
        let mut end = start.at(self.end).unwrap();

        if start > end {
            end = end.on_next_day();
        }

        DateTimeRange::new(start, end)
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
}
