use std::fmt::Display;

use super::{DateTime, Time};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyTimeRange {
    start: Time,
    end: Time,
}

impl Display for DailyTimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl DailyTimeRange {
    pub fn new(start: Time, end: Time) -> Self {
        Self { start, end }
    }

    pub fn prev_start(&self) -> DateTime {
        let now = DateTime::now();
        let mut start = now.at(self.start).unwrap();
        if start > now {
            start = start.on_prev_day();
        }

        start
    }

    pub fn for_today(&self) -> (DateTime, DateTime) {
        let now = DateTime::now();
        let start = now.at(self.start).unwrap();
        let mut end = start.at(self.end).unwrap();

        if start > end {
            end = end.on_next_day();
        }

        (start, end)
    }

    pub fn contains(&self, datetime: DateTime) -> bool {
        let time = datetime.time();

        // same-day scenario
        if self.start <= self.end {
            self.start <= time && time <= self.end
        } else {
            // cross-day scenario
            self.start <= time || time <= self.end
        }
    }
}
