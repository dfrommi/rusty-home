use std::fmt::Display;

use super::{DateTime, Duration, Time};

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
    pub fn new(start: (u32, u32), end: (u32, u32)) -> Self {
        Self {
            start: Time::at(start.0, start.1)
                .unwrap_or_else(|_| panic!("Error parsing time {}:{}", start.0, start.1)),
            end: Time::at(end.0, end.1)
                .unwrap_or_else(|_| panic!("Error parsing time {}:{}", end.0, end.1)),
        }
    }

    pub fn prev_start(&self) -> DateTime {
        let now = DateTime::now();
        let mut start = now.at(self.start).unwrap();
        if start > now {
            start -= Duration::days(1)
        }

        start
    }

    pub fn for_today(&self) -> (DateTime, DateTime) {
        let now = DateTime::now();
        let start = now.at(self.start).unwrap();
        let mut end = start.at(self.end).unwrap();

        if start > end {
            end += Duration::days(1);
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
