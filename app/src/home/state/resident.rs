use crate::core::time::DateTimeRange;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::Presence};
use anyhow::{Context, Result, bail};
use r#macro::{EnumVariants, Id};

use crate::home::state::macros::result;

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Resident {
    DennisSleeping,
    SabineSleeping,
    AnyoneOnCouch,
}

//TODO maybe combination via Baysian to detect resident state
impl DataPointAccess<Resident> for Resident {
    async fn current_data_point(&self, api: &crate::core::HomeApi) -> Result<DataPoint<bool>> {
        match self {
            Resident::DennisSleeping => sleeping(Presence::BedDennis, api).await,
            Resident::SabineSleeping => sleeping(Presence::BedSabine, api).await,
            Resident::AnyoneOnCouch => anyone_on_couch(api).await,
        }
    }
}

async fn sleeping(in_bed: Presence, api: &crate::core::HomeApi) -> Result<DataPoint<bool>> {
    let now = t!(now);
    let in_bed_full_range = t!(21:00 - 13:00).active_or_previous_at(now);

    if !in_bed_full_range.contains(now) {
        result!(
            false,
            *in_bed_full_range.end(),
            in_bed,
            bedtime_range.start = %in_bed_full_range.start(),
            bedtime_range.end = %in_bed_full_range.end(),
            "Not sleeping, because out of bedtime range"
        );
    }

    //TODO TimeSeries with date in future?
    let range_start = in_bed_full_range.start();
    let ts = in_bed
        .clone()
        .series_since(*range_start, api)
        .await?
        .with_duration_until_next_dp();

    let in_bed_start_range =
        DateTimeRange::new(*in_bed_full_range.start(), in_bed_full_range.end().at(t!(3:00)).unwrap());

    let in_bed_stop_range = DateTimeRange::new(in_bed_full_range.end().at(t!(5:00)).unwrap(), *in_bed_full_range.end());

    //Some has always true value
    let sleeping_started = ts
        .iter()
        .find(|dp| in_bed_start_range.contains(dp.timestamp) && dp.value.0 && dp.value.1 > t!(30 seconds))
        .map(|dp| dp.map_value(|v| v.1.clone()));

    //Some has always true value
    let sleeping_stopped = sleeping_started
        .as_ref()
        .and_then(|started_dp| {
            ts.iter().find(|dp| {
                in_bed_stop_range.contains(dp.timestamp)
                    && !dp.value.0
                    && dp.value.1 > t!(5 minutes)
                    && started_dp.timestamp < dp.timestamp
            })
        })
        .map(|dp| dp.map_value(|v| v.1.clone()));

    match (sleeping_started, sleeping_stopped) {
        (Some(started), Some(stopped)) => {
            result!(
                false,
                stopped.timestamp,
                in_bed,
                @started,
                @stopped,
                bedtime_range.start = %in_bed_full_range.start(),
                bedtime_range.end = %in_bed_full_range.end(),
                "Not sleeping, because out of bed for more than 5 minutes"
            );
        }

        //started but not stopped
        (Some(started_dp), None) => {
            result!(true, started_dp.timestamp, in_bed,
                @started_dp,
                bedtime_range.start = %in_bed_full_range.start(),
                bedtime_range.end = %in_bed_full_range.end(),
                "Sleeping, because in bed for more than 30 seconds"
            );
        }

        (None, None) => {
            result!(false, now, in_bed,
                bedtime_range.start = %in_bed_full_range.start(),
                bedtime_range.end = %in_bed_full_range.end(),
                "Not sleeping, because in time range, but no in bed for more than 30 seconds"
            );
        }

        (None, Some(stopped_dp)) => {
            bail!("Internal error: {} sleeping stopped, but not started: {:?}", in_bed, stopped_dp);
        }
    };
}

//TODO cover flaky on/off behaviour on movement
async fn anyone_on_couch(api: &crate::core::HomeApi) -> Result<DataPoint<bool>> {
    let (left, center, right) = tokio::try_join!(
        Presence::CouchLeft.current_data_point(api),
        Presence::CouchCenter.current_data_point(api),
        Presence::CouchRight.current_data_point(api)
    )?;

    let dps = [&left, &center, &right];

    //not fully correct. Iterate over timeseries backwards, then stop when first time all false

    let occupied_dps = dps.iter().filter(|dp| dp.value).collect::<Vec<_>>();

    if occupied_dps.is_empty() {
        return Ok(DataPoint::new(
            false,
            dps.iter()
                .map(|dp| dp.timestamp)
                .max()
                .context("Internal error: no minimum of non-empty vec")?,
        ));
    }

    Ok(DataPoint::new(
        true,
        occupied_dps
            .iter()
            .map(|dp| dp.timestamp)
            .min()
            .context("Internal error: no minimum of non-empty vec")?,
    ))
}
