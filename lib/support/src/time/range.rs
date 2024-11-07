use std::fmt::Display;

use chrono::{DateTime, NaiveTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyTimeRange {
    start: NaiveTime,
    end: NaiveTime,
}

impl Display for DailyTimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl DailyTimeRange {
    pub fn new(start: (u32, u32), end: (u32, u32)) -> Self {
        Self {
            start: NaiveTime::from_hms_opt(start.0, start.1, 0)
                .unwrap_or_else(|| panic!("Error parsing time {}:{}", start.0, start.1)),
            end: NaiveTime::from_hms_opt(end.0, end.1, 0)
                .unwrap_or_else(|| panic!("Error parsing time {}:{}", end.0, end.1)),
        }
    }

    pub fn prev_start(&self) -> DateTime<Utc> {
        let now = Utc::now();
        let mut start = now.with_time(self.start).unwrap();
        if start > now {
            start -= chrono::Duration::days(1)
        }

        start
    }

    pub fn for_today(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let start = now.with_time(self.start).unwrap();
        let mut end = start.with_time(self.end).unwrap();

        if start > end {
            end += chrono::Duration::days(1);
        }

        (start, end)
    }

    pub fn contains(&self, datetime: DateTime<Utc>) -> bool {
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
