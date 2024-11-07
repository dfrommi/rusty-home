use anyhow::Result;
use api::state::Presence;
use chrono::Utc;
use support::t;

use crate::{
    adapter::persistence::DataPoint,
    home_api,
    support::timeseries::{interpolate, TimeSeries},
};

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resident {
    DennisSleeping,
    SabineSleeping,
}

//TODO maybe combination via Baysian to detect resident state
impl DataPointAccess<bool> for Resident {
    async fn current_data_point(&self) -> Result<DataPoint<bool>> {
        match self {
            Resident::DennisSleeping => sleeping(Presence::BedDennis).await,
            Resident::SabineSleeping => sleeping(Presence::BedSabine).await,
        }
    }
}

async fn at_home(presence: Presence) -> Result<DataPoint<bool>> {
    home_api().get_latest(&presence).await
}

async fn sleeping(in_bed: Presence) -> Result<DataPoint<bool>> {
    let in_bed_full_range = t!(21:00 - 13:00);
    let in_bed_start_range = t!(21:00 - 3:00);

    let now = Utc::now();
    if !in_bed_full_range.contains(now) {
        return Ok(DataPoint {
            value: false,
            timestamp: now,
        });
    }

    //TODO TimeSeries with date in future?
    let range_start = in_bed_full_range.for_today().0;
    let ts = in_bed.series_since(range_start).await?.with_duration();

    let sleeping_started = ts.iter().find(|dp| {
        in_bed_start_range.contains(dp.timestamp) && dp.value.0 && dp.value.1 > t!(30 seconds)
    });

    let sleeping_stopped = sleeping_started.and_then(|started_dp| {
        ts.iter().find(|dp| {
            !dp.value.0 && dp.value.1 > t!(5 minutes) && started_dp.timestamp < dp.timestamp
        })
    });

    let result = match (sleeping_started, sleeping_stopped) {
        (_, Some(stopped_dp)) => (false, stopped_dp.timestamp),

        //started but not stopped
        (Some(started_dp), None) => (true, started_dp.timestamp),

        //should not happen
        (None, None) => (false, now),
    };

    Ok(DataPoint {
        value: result.0,
        timestamp: result.1,
    })
}

//TODO blanket impl
impl TimeSeriesAccess<bool> for Presence {
    async fn series_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<TimeSeries<bool>> {
        home_api()
            .get_covering(self, since)
            .await
            .map(|v| TimeSeries::new(v, since))?
    }
}
