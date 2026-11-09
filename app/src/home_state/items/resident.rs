use crate::core::time::DateTimeRange;
use crate::home_state::IsRunning;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::home_state::items::ventilation::Ventilation;
use crate::t;
use crate::trigger::{DimmerSwitchButtonPress, RemoteTrigger, RemoteTriggerTarget, UserTrigger, UserTriggerExecution};
use crate::{core::timeseries::DataPoint, home_state::Presence};
use actix_web::cookie::time;
use anyhow::Result;
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Resident {
    AnyoneSleeping,
    AnyoneOnCouch,
    PreparingForSleep,
}

pub struct ResidentStateProvider;

impl DerivedStateProvider<Resident, bool> for ResidentStateProvider {
    fn calculate_current(&self, id: Resident, ctx: &StateCalculationContext) -> Option<bool> {
        match id {
            Resident::AnyoneSleeping => {
                let ventilation = ctx.get(Ventilation::AcrossAllRooms)?;
                let tv_on = ctx.get(IsRunning::LivingRoomTv)?;
                let bed_light_control = ctx.latest_trigger(RemoteTriggerTarget::BedframeRemote.into());

                sleeping(tv_on, ventilation, bed_light_control)
            }

            Resident::AnyoneOnCouch => ctx.get(Presence::LivingRoomCouch).map(|dp| dp.value),
            Resident::PreparingForSleep => {
                let Some(time_range) = t!(22:30 - 3:00).active() else {
                    tracing::trace!("Not preparing for sleep, because out of going-to-bed range");
                    return Some(false);
                };

                let sleeping = ctx.get(Resident::AnyoneSleeping)?;
                let tv_states = ctx.all_since(IsRunning::LivingRoomTv, *time_range.start())?;

                pre_sleep(time_range, sleeping, tv_states)
            }
        }
    }
}

fn pre_sleep(
    time_range: DateTimeRange,
    sleeping: DataPoint<bool>,
    tv_states: crate::core::timeseries::DataFrame<bool>,
) -> Option<bool> {
    if sleeping.value {
        tracing::trace!("Not preparing for sleep, because already sleeping");
        return Some(false);
    }

    let Some((prev, current)) = tv_states.last2() else {
        tracing::trace!("Not preparing for sleep, because only one TV state found in time range");
        return Some(false);
    };

    if current.value {
        tracing::trace!("Not preparing for sleep, because TV is on");
        return Some(false);
    }

    //Current is false -> prev was true. Only check time range constraints
    if current.timestamp.elapsed_since(prev.timestamp) < t!(45 minutes) {
        tracing::trace!("Not preparing for sleep, because TV was on for less than 45 minutes");
        return Some(false);
    }

    if !time_range.contains(&current.timestamp) {
        tracing::trace!("Not preparing for sleep, because last TV state change was outside of bedtime range");
        return Some(false);
    }

    tracing::trace!(
        "Preparing for sleep, because TV off for {} after on for {} while not yet sleeping",
        current.timestamp.elapsed(),
        prev.timestamp.elapsed()
    );

    Some(true)
}

fn sleeping(
    tv_on: DataPoint<bool>,
    ventilation: DataPoint<bool>,
    bed_light_control: Option<UserTriggerExecution>,
) -> Option<bool> {
    //let in_bed_full_range = t!(22:30 - 13:00).active_or_previous_at(now);

    let in_bed_full_range = t!(22:30 - 13:00).active_or_previous();
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

        let bed_light_turned_off_ts = match bed_light_control {
            Some(UserTriggerExecution {
                trigger:
                    UserTrigger::Remote(RemoteTrigger::BedframeRemote(
                        DimmerSwitchButtonPress::SingleOff | DimmerSwitchButtonPress::HoldOff,
                    )),
                timestamp,
                ..
            }) => Some(timestamp),
            _ => None,
        };

        if let Some(bed_light_turned_off_ts) = bed_light_turned_off_ts
            && tv_on.timestamp < bed_light_turned_off_ts
        {
            tracing::trace!("Sleeping, because bed light turned off from bed after TV turned off");
        } else if tv_on.timestamp.elapsed() <= t!(30 minutes) {
            //Bedlight should usually trigger it. 30 minutes is just a safety net
            tracing::trace!("Not sleeping, because TV turned off less than 30 minutes ago");
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
