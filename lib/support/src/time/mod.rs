mod range;

pub use range::DailyTimeRange;

use chrono::{DateTime, Duration, NaiveTime, Utc};

pub fn in_time_range(
    to_test: DateTime<Utc>,
    from: (u32, u32),
    to: (u32, u32),
) -> anyhow::Result<bool> {
    let start_dt = at(to_test, from)?;
    let mut end_dt = at(to_test, to)?;

    if end_dt < start_dt {
        end_dt += Duration::days(1);
    }

    Ok(to_test >= start_dt && to_test <= end_dt)
}

fn at(date_time: DateTime<Utc>, time: (u32, u32)) -> anyhow::Result<DateTime<Utc>> {
    let Some(naive_time) = NaiveTime::from_hms_opt(time.0, time.1, 0) else {
        return Err(anyhow::anyhow!("Not a valid time {:?}", time));
    };

    date_time.with_time(naive_time).earliest().ok_or_else(|| {
        anyhow::anyhow!(
            "Error parsing time {:?} for date-time {:?}",
            time,
            date_time
        )
    })
}

pub fn elapsed_since(time: DateTime<Utc>) -> Duration {
    Utc::now() - time
}
