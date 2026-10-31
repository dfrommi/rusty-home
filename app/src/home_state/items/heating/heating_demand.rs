use crate::{
    core::domain::Radiator,
    core::{
        range::Range,
        time::DateTime,
        timeseries::{DataFrame, DataPoint, interpolate::LinearInterpolator},
        unit::{DegreeCelsius, Percent, RateOfChange},
    },
    home_state::{
        HeatingDemandLimit, SetPoint, Temperature,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
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
        match id {
            HeatingDemand::Radiator(radiator) => {
                let setpoint_range = ctx.get(SetPoint::Current(radiator))?;
                let is_heating = guess_is_heating_from_hyserisis(
                    setpoint_range.clone(),
                    ctx.all_since(Temperature::RadiatorExternalTempSensor(radiator), setpoint_range.timestamp)?,
                );
                let current_demand = ctx.get(HeatingDemandLimit::Current(radiator))?.value;
                if is_heating { current_demand } else { Percent(0.0) }
            }

            HeatingDemand::BarelyWarmSurface(radiator) => {
                let radiator_temperatures = ctx.all_since(radiator.surface_temperature(), t!(3 hours ago))?;
                let room_temperatures = ctx.all_since(radiator.room_temperature(), t!(3 hours ago))?;
                let demands = ctx.all_since(radiator.current_heating_demand(), t!(3 hours ago))?;
                estimate_barely_warm_surface(&radiator, radiator_temperatures, room_temperatures, demands)
            }
        }
        .into()
    }
}

fn guess_is_heating_from_hyserisis(
    setpoints: DataPoint<Range<DegreeCelsius>>,
    room_temperatures: DataFrame<DegreeCelsius>,
) -> bool {
    let Some(current_room_temp) = room_temperatures.last().map(|temp| temp.value) else {
        return false;
    };

    let min = *setpoints.value.from();
    let max = *setpoints.value.to();

    if current_room_temp <= min {
        true
    } else if current_room_temp >= max {
        false
    } else {
        //in-range. Check if range left in any direction
        let latest = room_temperatures
            .latest_where(|temp| temp.value >= max || temp.value <= min)
            .take_if(|temp| temp.timestamp >= setpoints.timestamp)
            .map(|temp| temp.value);

        match latest {
            Some(temp) if (temp >= max) => false,
            Some(temp) if (temp <= min) => true,
            //Seems to heat if range is around current temp on setpoint change
            _ => true,
        }
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
        Radiator::LivingRoomBig | Radiator::LivingRoomSmall | Radiator::RoomOfRequirements => Percent(20.0),
        Radiator::Bedroom | Radiator::Kitchen => Percent(8.0),
        Radiator::Bathroom => Percent(20.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hysteresis_switches_at_the_setpoint_boundaries() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        let setpoints = DataPoint::new(Range::new(DegreeCelsius(19.0), DegreeCelsius(20.0)), now);

        for (temperature, expected) in [(18.0, true), (19.0, true), (20.0, false), (21.0, false)] {
            let temperatures = DataFrame::new([DataPoint::new(DegreeCelsius(temperature), now)]);
            assert_eq!(guess_is_heating_from_hyserisis(setpoints.clone(), temperatures), expected);
        }
        assert!(!guess_is_heating_from_hyserisis(setpoints, DataFrame::empty()));
    }

    #[test]
    fn hysteresis_preserves_the_last_switch_inside_the_band() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        let setpoints = DataPoint::new(Range::new(DegreeCelsius(19.0), DegreeCelsius(20.0)), now - t!(1 hours));

        for (previous_temperature, expected) in [(18.0, true), (21.0, false)] {
            let temperatures = DataFrame::new([
                DataPoint::new(DegreeCelsius(previous_temperature), now - t!(30 minutes)),
                DataPoint::new(DegreeCelsius(19.5), now),
            ]);
            assert_eq!(guess_is_heating_from_hyserisis(setpoints.clone(), temperatures), expected);
        }
    }

    #[test]
    fn setpoint_change_ignores_older_switches_and_arms_heating_inside_the_band() {
        let now = DateTime::from_static_iso("2026-01-01T12:00:00Z");
        let setpoints = DataPoint::new(Range::new(DegreeCelsius(19.0), DegreeCelsius(20.0)), now);
        let temperatures = DataFrame::new([
            DataPoint::new(DegreeCelsius(21.0), now - t!(30 minutes)),
            DataPoint::new(DegreeCelsius(19.5), now),
        ]);

        assert!(guess_is_heating_from_hyserisis(setpoints, temperatures));
    }
}
