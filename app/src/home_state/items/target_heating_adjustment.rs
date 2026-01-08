use r#macro::{EnumVariants, Id};

use crate::{
    automation::{HeatingZone, Thermostat},
    core::{timeseries::DataFrame, unit::DegreeCelsius},
    home_state::{
        HeatingMode, TargetHeatingMode,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingAdjustment {
    Radiator(Thermostat),
    Setpoint(Thermostat),
    HeatingDemand(Thermostat),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdjustmentDirection {
    MustIncrease,
    ShouldIncrease,
    Hold,
    ShouldDecrease,
    MustDecrease,
    MustOff,
}

pub struct TargetHeatingAdjustmentStateProvider;

impl DerivedStateProvider<TargetHeatingAdjustment, AdjustmentDirection> for TargetHeatingAdjustmentStateProvider {
    fn calculate_current(
        &self,
        id: TargetHeatingAdjustment,
        ctx: &StateCalculationContext,
    ) -> Option<AdjustmentDirection> {
        let thermostat = match id {
            TargetHeatingAdjustment::Radiator(thermostat) => thermostat,
            TargetHeatingAdjustment::Setpoint(thermostat) => thermostat,
            TargetHeatingAdjustment::HeatingDemand(thermostat) => thermostat,
        };
        let heating_zone = HeatingZone::for_thermostat(&thermostat);
        let mode = ctx.get(TargetHeatingMode::from_thermostat(thermostat))?.value;

        match id {
            TargetHeatingAdjustment::Radiator(thermostat) => {
                let radiator_temperatures = ctx.all_since(thermostat.surface_temperature(), t!(1 hours ago))?;
                let current_radiator_temperature = radiator_temperatures.last()?.value;
                let current_room_temperature = ctx.get(heating_zone.inside_temperature())?.value;

                let radiator_strategy = radiator_strategy(current_room_temperature, mode);
                Some(
                    radiator_strategy
                        .adjustment_direction(current_radiator_temperature, change_per_hour(&radiator_temperatures)?),
                )
            }
            TargetHeatingAdjustment::Setpoint(thermostat) => {
                let room_temperatures = ctx.all_since(heating_zone.inside_temperature(), t!(1 hours ago))?;
                let current_room_temperature = room_temperatures.last()?.value;
                let setpoint = ctx.get(thermostat.set_point())?;

                let setpoint_strategy = setpoint_strategy(setpoint.value, mode);

                Some(
                    setpoint_strategy
                        .adjustment_direction(current_room_temperature, change_per_hour(&room_temperatures)?),
                )
            }
            TargetHeatingAdjustment::HeatingDemand(thermostat) => {
                let radiator_adjustment = ctx.get(TargetHeatingAdjustment::Radiator(thermostat))?.value;
                let setpoint_adjustment = ctx.get(TargetHeatingAdjustment::Setpoint(thermostat))?.value;

                //TODO decide better what the fallback should be
                radiator_adjustment
                    .merge(&setpoint_adjustment)
                    .or(Some(setpoint_adjustment))
            }
        }
    }
}

fn radiator_strategy(current_room_temperature: DegreeCelsius, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    macro_rules! new {
        ($min:literal - $max:literal, min_heatup = $min_heatup:literal / h, max_overshoot = $max_overshoot:expr) => {
            HeatingAdjustmentStrategy::new(
                (
                    current_room_temperature + DegreeCelsius($min),
                    current_room_temperature + DegreeCelsius($max),
                ),
                DegreeCelsius($min_heatup),
                DegreeCelsius($max_overshoot),
            )
        };
    }

    match mode {
        HeatingMode::Manual(_, _) => new!(8.0 - 14.0, min_heatup = 6.0 / h, max_overshoot = 6.0),
        HeatingMode::Comfort => new!(7.0 - 11.0, min_heatup = 5.0 / h, max_overshoot = 5.0),
        HeatingMode::EnergySaving => new!(4.0 - 8.0, min_heatup = 2.0 / h, max_overshoot = 3.0),
        HeatingMode::Sleep => new!(5.0 - 8.0, min_heatup = 3.0 / h, max_overshoot = 3.0),
        HeatingMode::Ventilation => new!(0.0 - 3.0, min_heatup = 1.0 / h, max_overshoot = 1.0),
        HeatingMode::PostVentilation => new!(3.0 - 6.0, min_heatup = 1.0 / h, max_overshoot = 2.0),
        HeatingMode::Away => new!(2.0 - 6.0, min_heatup = 0.5 / h, max_overshoot = 2.0),
    }
}

fn setpoint_strategy(setpoint: DegreeCelsius, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    macro_rules! new {
        ($min:literal - $max:literal, min_heatup = $min_heatup:literal / h, max_overshoot = $max_overshoot:expr) => {
            HeatingAdjustmentStrategy::new(
                (setpoint + DegreeCelsius($min), setpoint + DegreeCelsius($max)),
                DegreeCelsius($min_heatup),
                DegreeCelsius($max_overshoot),
            )
        };
    }

    match mode {
        HeatingMode::Manual(_, _) => new!(-0.2 - 0.2, min_heatup = 2.0 / h, max_overshoot = 0.4),
        HeatingMode::Comfort => new!(-0.4 - 0.0, min_heatup = 1.5 / h, max_overshoot = 0.2),
        HeatingMode::EnergySaving => new!(-0.6 - 0.0, min_heatup = 1.0 / h, max_overshoot = 0.1),
        HeatingMode::Sleep => new!(-0.8 - 0.0, min_heatup = 0.75 / h, max_overshoot = 0.0),
        HeatingMode::Ventilation => new!(-5.0 - 0.0, min_heatup = 0.2 / h, max_overshoot = 0.0),
        HeatingMode::PostVentilation => new!(-1.5 - 0.0, min_heatup = 0.4 / h, max_overshoot = 0.0),
        HeatingMode::Away => new!(-1.0 - 0.0, min_heatup = 0.4 / h, max_overshoot = 0.0),
    }
}

fn change_per_hour(temperatures: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
    let (start, end) = temperatures.last2()?;
    let delta_temp = end.value - start.value;
    let delta_time = end.timestamp.elapsed_since(start.timestamp).as_hours_f64();
    Some(delta_temp / delta_time)
}

impl AdjustmentDirection {
    fn merge(&self, other: &AdjustmentDirection) -> Option<AdjustmentDirection> {
        use AdjustmentDirection::*;

        match (self, other) {
            (MustOff, MustOff)
            | (MustOff, ShouldDecrease)
            | (MustOff, ShouldIncrease)
            | (MustOff, Hold)
            | (ShouldDecrease, MustOff)
            | (ShouldIncrease, MustOff)
            | (Hold, MustOff) => Some(MustOff),
            (MustIncrease, MustIncrease)
            | (MustIncrease, ShouldIncrease)
            | (MustIncrease, ShouldDecrease)
            | (MustIncrease, Hold)
            | (ShouldIncrease, MustIncrease)
            | (ShouldDecrease, MustIncrease)
            | (Hold, MustIncrease) => Some(MustIncrease),
            (MustDecrease, MustDecrease)
            | (MustDecrease, ShouldDecrease)
            | (MustDecrease, ShouldIncrease)
            | (MustDecrease, Hold)
            | (ShouldDecrease, MustDecrease)
            | (ShouldIncrease, MustDecrease)
            | (Hold, MustDecrease) => Some(MustDecrease),
            (ShouldIncrease, ShouldIncrease) | (ShouldIncrease, Hold) | (Hold, ShouldIncrease) => Some(ShouldIncrease),
            (ShouldDecrease, ShouldDecrease) | (ShouldDecrease, Hold) | (Hold, ShouldDecrease) => Some(ShouldDecrease),
            (Hold, Hold) => Some(Hold),
            _ => None, //conflicting directions
        }
    }
}

struct HeatingAdjustmentStrategy {
    min: DegreeCelsius,
    max: DegreeCelsius,
    min_heatup_change_per_hour: DegreeCelsius,
    band: DegreeCelsius,
    max_overshoot: DegreeCelsius,
}

impl HeatingAdjustmentStrategy {
    fn new(
        range: (DegreeCelsius, DegreeCelsius),
        min_heatup_change_per_hour: DegreeCelsius,
        max_overshoot: DegreeCelsius,
    ) -> Self {
        Self {
            min: range.0,
            max: range.1,
            band: (range.1 - range.0) * 0.25,
            min_heatup_change_per_hour,
            max_overshoot,
        }
    }

    fn adjustment_direction(
        &self,
        current: DegreeCelsius,
        current_change_per_hour: DegreeCelsius,
    ) -> AdjustmentDirection {
        let increasing = current_change_per_hour > DegreeCelsius(0.1);
        let decreasing = current_change_per_hour < DegreeCelsius(0.1);

        //Too low -> increase if not heating up fast enough already
        if current < self.min && current_change_per_hour < self.min_heatup_change_per_hour {
            return AdjustmentDirection::MustIncrease;
        }

        //In lower band -> increase to avoid undershoot
        if current >= self.min && current <= self.min + self.band && decreasing {
            return AdjustmentDirection::ShouldIncrease;
        }

        //no rule for center area -> hold

        //In upper band -> decrease to avoid overshoot
        if current >= self.max - self.band && current <= self.max && increasing {
            return AdjustmentDirection::ShouldDecrease;
        }

        //Too high -> decrease
        if current > self.max && current <= self.max + self.max_overshoot {
            return AdjustmentDirection::MustDecrease;
        }

        //Too much overshoot -> turn off for cooldown
        if current > self.max + self.max_overshoot {
            return AdjustmentDirection::MustOff;
        }

        AdjustmentDirection::Hold
    }
}
