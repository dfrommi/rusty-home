mod range;

pub use range::DailyTimeRange;

use chrono::{DateTime, Duration, NaiveTime, Utc};

pub fn in_time_range(
    to_test: DateTime<Utc>,
    from: (u32, u32),
    to: (u32, u32),
) -> anyhow::Result<bool> {
    let start_dt = to_test
        .with_time(NaiveTime::from_hms(from.0, from.1, 0))
        .single()
        .ok_or_else(|| anyhow::anyhow!("Error parsing start-time {:?}", from))?;

    let mut end_dt = to_test
        .with_time(NaiveTime::from_hms(to.0, to.1, 0))
        .single()
        .ok_or_else(|| anyhow::anyhow!("Error parsing end-time {:?}", to))?;

    if end_dt < start_dt {
        end_dt += Duration::days(1);
    }

    Ok(to_test >= start_dt && to_test <= end_dt)
}

pub fn elapsed_since(time: DateTime<Utc>) -> Duration {
    Utc::now() - time
}
