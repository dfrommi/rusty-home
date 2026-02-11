use crate::core::time::DateTimeRange;
use crate::home_state::IsRunning;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::home_state::items::ventilation::Ventilation;
use crate::t;
use crate::{core::timeseries::DataPoint, home_state::Presence};
use anyhow::Result;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Resident {
    AnyoneSleeping,
    AnyoneOnCouch,
}

pub struct ResidentStateProvider;

impl DerivedStateProvider<Resident, bool> for ResidentStateProvider {
    fn calculate_current(&self, id: Resident, ctx: &StateCalculationContext) -> Option<bool> {
        match id {
            Resident::AnyoneSleeping => {
                let ventilation = ctx.get(Ventilation::AcrossAllRooms)?;
                let tv_on = ctx.get(IsRunning::LivingRoomTv)?;

                sleeping(tv_on, ventilation)
            }

            Resident::AnyoneOnCouch => ctx.get(Presence::LivingRoomCouch).map(|dp| dp.value),
        }
    }
}

fn sleeping(tv_on: DataPoint<bool>, ventilation: DataPoint<bool>) -> Option<bool> {
    //let in_bed_full_range = t!(22:30 - 13:00).active_or_previous_at(now);

    let in_bed_full_range = t!(22:30 - 13:00).active_or_previous_at(t!(now));
    let in_bed_start_range = DateTimeRange::new(*in_bed_full_range.start(), in_bed_full_range.end().at(t!(3:00)));
    let in_bed_stop_range = DateTimeRange::new(in_bed_full_range.end().at(t!(6:00)), *in_bed_full_range.end());

    if !in_bed_full_range.is_active() {
        tracing::trace!("Not sleeping, because out of bedtime range");
        return Some(false);
    }

    //TODO improve by incorporating more hints
    //like dimmer switch

    if in_bed_start_range.is_active() {
        tracing::trace!("In bed start range active");

        if tv_on.value {
            tracing::trace!("Not sleeping, because TV is still on");
            return Some(false);
        }

        //tv off
        if tv_on.timestamp < *in_bed_start_range.start() {
            tracing::trace!("Not sleeping, because TV turned off before bed time range");
            return Some(false);
        }

        if tv_on.timestamp.elapsed() <= t!(10 minutes) {
            tracing::trace!("Not sleeping, because TV turned off less than 10 minutes ago");
            return Some(false);
        }
    }

    tracing::trace!("Sleeping started");

    if in_bed_stop_range.is_active() {
        tracing::trace!("In bed stop range active");

        if in_bed_stop_range.contains(&ventilation.timestamp) {
            tracing::trace!("Not sleeping, because ventilation not yet done");
            return Some(false);
        }
    }

    tracing::trace!("Sleeping stopped");

    //started but not stopped yet, so sleeping
    Some(true)
}
