use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;
use crate::{core::timeseries::DataPoint, home_state::Presence};
use anyhow::{Result, bail};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Resident {
    AnyoneSleeping,
    AnyoneOnCouch,
}

pub struct ResidentStateProvider;

impl DerivedStateProvider<Resident, bool> for ResidentStateProvider {
    fn calculate_current(&self, id: Resident, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        match id {
            Resident::AnyoneSleeping => {
                let in_bed_full_range = t!(22:30 - 13:00).active_or_previous_at(t!(now));
                let in_bed_df = ctx.all_since(Presence::BedroomBed, *in_bed_full_range.start())?;

                match sleeping(in_bed_full_range, in_bed_df) {
                    Ok(dp) => Some(dp),
                    Err(e) => {
                        tracing::error!("Error calculating AnyoneSleeping: {:?}", e);
                        None
                    }
                }
            }

            Resident::AnyoneOnCouch => ctx.get(Presence::LivingRoomCouch),
        }
    }
}

fn sleeping(in_bed_full_range: DateTimeRange, in_bed_since_range_start: DataFrame<bool>) -> Result<DataPoint<bool>> {
    //let in_bed_full_range = t!(22:30 - 13:00).active_or_previous_at(now);

    if !in_bed_full_range.is_active() {
        tracing::trace!("Not sleeping, because out of bedtime range");
        return Ok(DataPoint::new(false, *in_bed_full_range.end()));
    }

    //TODO TimeSeries with date in future?
    let ts = in_bed_since_range_start.with_duration_until_next_dp();

    let in_bed_start_range = DateTimeRange::new(*in_bed_full_range.start(), in_bed_full_range.end().at(t!(3:00)));

    let in_bed_stop_range = DateTimeRange::new(in_bed_full_range.end().at(t!(6:00)), *in_bed_full_range.end());

    //Some has always true value
    let sleeping_started = ts
        .iter()
        .find(|dp| in_bed_start_range.contains(&dp.timestamp) && dp.value.0 && dp.value.1 > t!(30 seconds))
        .map(|dp| dp.map_value(|v| v.1.clone()));

    //Some has always true value
    let sleeping_stopped = sleeping_started
        .as_ref()
        .and_then(|started_dp| {
            ts.iter().find(|dp| {
                in_bed_stop_range.contains(&dp.timestamp)
                    && !dp.value.0
                    && dp.value.1 > t!(5 minutes)
                    && started_dp.timestamp < dp.timestamp
            })
        })
        .map(|dp| dp.map_value(|v| v.1.clone()));

    match (sleeping_started, sleeping_stopped) {
        (Some(_started), Some(stopped)) => {
            tracing::trace!("Not sleeping, because out of bed for more than 5 minutes");
            Ok(DataPoint::new(false, stopped.timestamp))
        }

        //started but not stopped
        (Some(started_dp), None) => {
            tracing::trace!("Sleeping, because in bed for more than 30 seconds");
            Ok(DataPoint::new(true, started_dp.timestamp))
        }

        (None, None) => {
            tracing::trace!("Not sleeping, because in time range, but no in bed for more than 30 seconds");
            Ok(DataPoint::new(false, t!(now)))
        }

        (None, Some(stopped_dp)) => {
            bail!("Internal error: sleeping stopped, but not started: {:?}", stopped_dp);
        }
    }
}
