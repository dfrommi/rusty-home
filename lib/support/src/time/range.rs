use std::fmt::Display;

use crate::t;

use super::{DateTime, Time};

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

    pub fn since(start: DateTime) -> Self {
        Self::new(start, t!(now))
    }

    pub fn start(&self) -> DateTime {
        self.start
    }

    pub fn end(&self) -> DateTime {
        self.end
    }

    pub fn contains(&self, datetime: DateTime) -> bool {
        datetime >= self.start && datetime <= self.end
    }
}
