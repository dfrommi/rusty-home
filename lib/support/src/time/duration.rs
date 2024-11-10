use super::DateTime;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Duration {
    pub(super) delegate: chrono::Duration,
}

impl Duration {
    pub(super) fn new(delegate: chrono::Duration) -> Self {
        Self { delegate }
    }

    pub fn until(date_time: &DateTime) -> Self {
        Self::new(*date_time.delegate() - DateTime::now().delegate())
    }

    pub fn days(days: i64) -> Self {
        Self::new(chrono::Duration::days(days))
    }

    pub fn hours(hours: i64) -> Self {
        Self::new(chrono::Duration::hours(hours))
    }

    pub fn minutes(minutes: i64) -> Self {
        Self::new(chrono::Duration::minutes(minutes))
    }

    pub fn seconds(seconds: i64) -> Self {
        Self::new(chrono::Duration::seconds(seconds))
    }

    pub fn as_secs(&self) -> i64 {
        self.delegate.num_seconds()
    }

    pub fn as_secs_f64(&self) -> f64 {
        self.delegate.num_milliseconds() as f64 / 1000.0
    }

    pub fn as_minutes(&self) -> i64 {
        self.delegate.num_minutes()
    }

    pub fn as_hours(&self) -> i64 {
        self.delegate.num_hours()
    }
}
