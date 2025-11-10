use crate::core::time::DateTime;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;



pub mod energy_iq;
pub mod energy_monitor;
pub mod heating_details;
pub mod meta;
pub mod smart_home;

use super::support::empty_string_as_none;

const EURO_PER_KWH: f64 = 0.349;

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

#[derive(Clone, Debug, serde::Deserialize)]
struct TimeRangeWithIntervalQuery {
    from: DateTime,
    to: DateTime,
    interval_ms: i64,
}

impl TimeRangeWithIntervalQuery {
    fn range(&self) -> DateTimeRange {
        DateTimeRange::new(self.from, self.to).non_future()
    }

    fn iter(&self) -> impl Iterator<Item = DateTime> + '_ {
        self.range().step_by(Duration::millis(self.interval_ms))
    }
}
