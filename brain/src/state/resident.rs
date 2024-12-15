use anyhow::Result;
use api::state::{ChannelTypeInfo, Presence};
use support::{t, DataPoint};

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resident {
    DennisSleeping,
    SabineSleeping,
}

impl ChannelTypeInfo for Resident {
    type ValueType = bool;
}

//TODO maybe combination via Baysian to detect resident state
impl<T> DataPointAccess<Resident> for T
where
    T: DataPointAccess<Presence> + TimeSeriesAccess<Presence>,
{
    async fn current_data_point(&self, item: Resident) -> Result<DataPoint<bool>> {
        match item {
            Resident::DennisSleeping => sleeping(Presence::BedDennis, self).await,
            Resident::SabineSleeping => sleeping(Presence::BedSabine, self).await,
        }
    }
}

async fn sleeping(
    in_bed: Presence,
    api: &impl TimeSeriesAccess<Presence>,
) -> Result<DataPoint<bool>> {
    let in_bed_full_range = t!(21:00 - 13:00).starting_today();
    let in_bed_start_range = t!(21:00 - 3:00).starting_today();

    let now = t!(now);
    if !in_bed_full_range.contains(now) {
        return Ok(DataPoint {
            value: false,
            timestamp: now,
        });
    }

    //TODO TimeSeries with date in future?
    let range_start = in_bed_full_range.start();
    let ts = api
        .series_since(in_bed, *range_start)
        .await?
        .with_duration();

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
