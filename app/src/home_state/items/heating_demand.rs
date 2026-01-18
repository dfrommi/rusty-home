use crate::{
    automation::Radiator,
    core::{
        time::DateTime,
        timeseries::{DataFrame, interpolate::LinearInterpolator},
        unit::{DegreeCelsius, Percent, RateOfChange},
    },
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
    t,
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    Radiator(Radiator),
    BarelyWarmSurface(Radiator),
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<HeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: HeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        use crate::device_state::HeatingDemand as DeviceHeatingDemand;

        match id {
            HeatingDemand::Radiator(radiator) => match radiator {
                Radiator::LivingRoomBig => ctx.device_state(DeviceHeatingDemand::LivingRoomBig)?.value,
                Radiator::LivingRoomSmall => ctx.device_state(DeviceHeatingDemand::LivingRoomSmall)?.value,
                Radiator::Bedroom => ctx.device_state(DeviceHeatingDemand::Bedroom)?.value,
                Radiator::Kitchen => ctx.device_state(DeviceHeatingDemand::Kitchen)?.value,
                Radiator::RoomOfRequirements => ctx.device_state(DeviceHeatingDemand::RoomOfRequirements)?.value,
                Radiator::Bathroom => ctx.device_state(DeviceHeatingDemand::Bathroom)?.value,
            },
            HeatingDemand::BarelyWarmSurface(radiator) => {
                let radiator_temperatures = ctx.all_since(radiator.surface_temperature(), t!(3 hours ago))?;
                let room_temperatures = ctx.all_since(radiator.room_temperature(), t!(3 hours ago))?;
                let demands = ctx.all_since(radiator.heating_demand(), t!(3 hours ago))?;
                estimate_barely_warm_surface(&radiator, radiator_temperatures, room_temperatures, demands)
            }
        }
        .into()
    }
}

fn estimate_barely_warm_surface(
    radiator: &Radiator,
    radiator_temperatures: DataFrame<DegreeCelsius>,
    room_temperatures: DataFrame<DegreeCelsius>,
    demands: DataFrame<Percent>,
) -> Percent {
    let offset = t!(5 minutes);
    let demand_lower_bound = Percent(5.0);

    struct Delta {
        demand: Percent,
        increase: RateOfChange<DegreeCelsius>,
        is_increasing: bool,
        is_hot_and_holding: bool,
        time: DateTime,
    }

    let mut delta_candidates: Vec<Delta> = Vec::new();

    let rad_above_room = DataFrame::by_reducing2(
        (&radiator_temperatures, LinearInterpolator),
        (&room_temperatures, LinearInterpolator),
        |rad_temp, room_temp| rad_temp.value - room_temp.value,
    );

    for dt in rad_above_room.iter_dt() {
        let Some(demand) = demands.prev_or_at(dt) else {
            continue;
        };

        //only realisic values
        if demand.value <= demand_lower_bound {
            continue;
        }

        let Some(prev_delta) = rad_above_room.at(dt - offset.clone(), LinearInterpolator) else {
            continue;
        };
        let Some(next_delta) = rad_above_room.at(dt + offset.clone(), LinearInterpolator) else {
            continue;
        };

        let roc = RateOfChange::from_dps(&prev_delta, &next_delta);

        let is_increasing = roc > RateOfChange::new(DegreeCelsius(2.0), t!(10 minutes));
        let is_hot_and_holding = prev_delta.value > DegreeCelsius(8.0)
            && next_delta.value > DegreeCelsius(8.0)
            && roc > RateOfChange::new(DegreeCelsius(-0.2), t!(10 minutes));

        if is_increasing || is_hot_and_holding {
            let causing_demand = demands
                .prev_or_at(dt - t!(5 minutes))
                .take_if(|d| d.value > demand_lower_bound)
                .unwrap_or(demand);
            delta_candidates.push(Delta {
                demand: causing_demand.value,
                increase: roc,
                is_increasing,
                is_hot_and_holding,
                time: dt,
            });
        }
    }

    if let Some(min) = delta_candidates
        .iter()
        .min_by(|a, b| a.demand.partial_cmp(&b.demand).unwrap_or(std::cmp::Ordering::Equal))
    {
        tracing::trace!(
            "Estimated barely warm surface demand for {:?} as {:?} (increase: {}/h, is_increasing: {}, is_hot_and_holding: {}, time: {:?})",
            radiator,
            min.demand,
            min.increase.per_hour(),
            min.is_increasing,
            min.is_hot_and_holding,
            min.time,
        );
        return min.demand;
    }

    //TODO requires heat quite often. If not found, expand search range, maybe to similar time range
    //on previous day

    //TODO should take outside temperature into account, as overall heating availability changes with it
    match radiator {
        Radiator::LivingRoomBig => Percent(16.0),
        Radiator::LivingRoomSmall => Percent(18.0),
        Radiator::Bedroom => Percent(16.0),
        Radiator::Kitchen => Percent(18.0),
        Radiator::RoomOfRequirements => Percent(14.0),
        Radiator::Bathroom => Percent(20.0),
    }
}
