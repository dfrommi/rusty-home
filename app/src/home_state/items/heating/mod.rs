pub(super) mod heating_demand;
pub(super) mod heating_demand_limit;
pub(super) mod set_point;
pub(super) mod target_heating_adjustment;
pub(super) mod target_heating_demand;
pub(super) mod target_heating_mode;

pub use heating_demand::HeatingDemand;
pub use heating_demand_limit::HeatingDemandLimit;
pub use set_point::SetPoint;
pub use target_heating_adjustment::{AdjustmentDirection, TargetHeatingAdjustment};
pub use target_heating_demand::TargetHeatingDemand;
pub use target_heating_mode::*;

use crate::{
    automation::Radiator,
    core::{
        range::Range,
        unit::{DegreeCelsius, Percent, RateOfChange},
    },
    t,
};

struct HeatingAdjustmentStrategy {
    min: DegreeCelsius,
    max: DegreeCelsius,
    min_heatup: Option<RateOfChange<DegreeCelsius>>,
    band: DegreeCelsius,
    max_overshoot: DegreeCelsius,
}

impl HeatingAdjustmentStrategy {
    fn new(
        range: (DegreeCelsius, DegreeCelsius),
        min_heatup: Option<RateOfChange<DegreeCelsius>>,
        max_overshoot: DegreeCelsius,
    ) -> Self {
        Self {
            min: range.0,
            max: range.1,
            band: (range.1 - range.0) * 1.0 / 3.0,
            min_heatup,
            max_overshoot,
        }
    }
}

struct ControlLimits {
    barely_warm: Percent,
    step: Percent,
    cold_start_should_factor: f64,
    cold_start_must_factor: f64,
    min_output: Percent,
    max_output: Percent,
}

impl ControlLimits {
    fn new(radiator: &Radiator, mode: &HeatingMode, barely_warm_output: Percent) -> Self {
        let (min_output, step) = match radiator {
            Radiator::LivingRoomBig | Radiator::LivingRoomSmall | Radiator::RoomOfRequirements => {
                (Percent(12.0), Percent(5.0))
            }
            Radiator::Bedroom | Radiator::Kitchen => (Percent(6.0), Percent(3.0)),
            Radiator::Bathroom => (Percent(10.0), Percent(7.0)),
        };

        Self {
            barely_warm: barely_warm_output,
            step,
            cold_start_should_factor: match mode {
                HeatingMode::Comfort | HeatingMode::Manual(_, _) => 1.0,
                _ => 0.0,
            },
            cold_start_must_factor: match mode {
                HeatingMode::Comfort | HeatingMode::Manual(_, _) => 2.0,
                _ => 0.0,
            },
            min_output,
            max_output: match mode {
                HeatingMode::Ventilation => Percent(0.0),
                HeatingMode::EnergySaving => Percent(40.0),
                HeatingMode::Comfort => Percent(50.0),
                HeatingMode::Manual(_, _) => Percent(60.0),
                HeatingMode::Sleep => Percent(40.0),
                HeatingMode::Away => Percent(30.0),
            }
            .max(min_output),
        }
    }

    fn clamp(&self, output: Percent) -> Percent {
        let output = output.round();
        if output < self.min_output {
            self.min_output
        } else if output > self.max_output {
            self.max_output
        } else {
            output
        }
        .clamp()
    }
}

fn radiator_strategy(current_room_temperature: DegreeCelsius, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    let max_temp = match mode {
        HeatingMode::Manual(_, _) => 14.0,
        HeatingMode::Comfort => 11.0,
        HeatingMode::EnergySaving => 8.0,
        HeatingMode::Sleep => 8.0,
        HeatingMode::Ventilation => 3.0,
        HeatingMode::Away => 6.0,
    };

    HeatingAdjustmentStrategy::new(
        (
            DegreeCelsius(0.0), //no forced heating caused by radiator temp
            current_room_temperature + DegreeCelsius(max_temp),
        ),
        None,
        //don't force it off due to radiator unless very hot
        DegreeCelsius(10.0),
    )
}

fn setpoint_strategy(setpoint: Range<DegreeCelsius>, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    let min_heatup_per_hour = match mode {
        HeatingMode::Manual(_, _) => 2.0,
        HeatingMode::Comfort => 1.5,
        HeatingMode::EnergySaving => 1.0,
        HeatingMode::Sleep => 0.75,
        HeatingMode::Ventilation => 0.2,
        HeatingMode::Away => 0.4,
    };

    HeatingAdjustmentStrategy::new(
        setpoint.into(),
        RateOfChange::new(DegreeCelsius(min_heatup_per_hour), t!(1 hours)).into(),
        DegreeCelsius(1.0),
    )
}

fn setpoint_for_mode(radiator: Radiator, mode: &HeatingMode) -> Range<DegreeCelsius> {
    let t = match (radiator, mode) {
        (_, HeatingMode::Manual(t, _)) => t.0,
        (_, HeatingMode::Ventilation) => 0.0,
        (Radiator::LivingRoomBig, HeatingMode::EnergySaving) => 19.0,
        (Radiator::LivingRoomBig, HeatingMode::Sleep) => 18.5,
        (Radiator::LivingRoomBig, HeatingMode::Comfort) => 19.5,
        (Radiator::LivingRoomBig, HeatingMode::Away) => 17.0,
        (Radiator::LivingRoomSmall, HeatingMode::EnergySaving) => 19.0,
        (Radiator::LivingRoomSmall, HeatingMode::Sleep) => 18.5,
        (Radiator::LivingRoomSmall, HeatingMode::Comfort) => 19.5,
        (Radiator::LivingRoomSmall, HeatingMode::Away) => 17.0,
        (Radiator::RoomOfRequirements, HeatingMode::EnergySaving) => 18.0,
        (Radiator::RoomOfRequirements, HeatingMode::Sleep) => 17.0,
        (Radiator::RoomOfRequirements, HeatingMode::Comfort) => 19.0,
        (Radiator::RoomOfRequirements, HeatingMode::Away) => 16.0,
        (Radiator::Bedroom, HeatingMode::EnergySaving) => 17.5,
        (Radiator::Bedroom, HeatingMode::Sleep) => 18.5,
        (Radiator::Bedroom, HeatingMode::Comfort) => 19.0,
        (Radiator::Bedroom, HeatingMode::Away) => 16.5,
        (Radiator::Kitchen, HeatingMode::EnergySaving) => 17.0,
        (Radiator::Kitchen, HeatingMode::Sleep) => 16.5,
        (Radiator::Kitchen, HeatingMode::Comfort) => 18.0,
        (Radiator::Kitchen, HeatingMode::Away) => 16.0,
        (Radiator::Bathroom, _) => 15.0,
    };

    //range: 0.2 - 1.0 with 0.2 increments
    let offset = match mode {
        HeatingMode::Comfort | HeatingMode::Manual(_, _) => 0.4,
        HeatingMode::EnergySaving | HeatingMode::Ventilation | HeatingMode::Sleep | HeatingMode::Away => 1.0,
    };

    Range::new(DegreeCelsius(t), DegreeCelsius(t - offset))
}
