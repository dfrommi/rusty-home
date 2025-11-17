use crate::core::time::DateTime;
use crate::core::time::DateTimeRange;

pub mod meta;
pub mod smart_home;

#[derive(Clone, Debug, serde::Deserialize)]
struct TimeRangeQuery {
    from: DateTime,
    to: DateTime,
}

impl TimeRangeQuery {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }
}
