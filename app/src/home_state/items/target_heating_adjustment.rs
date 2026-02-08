use r#macro::{EnumVariants, Id};

use crate::{
    automation::Radiator,
    core::{
        range::Range,
        unit::{DegreeCelsius, RateOfChange},
    },
    home_state::{
        HeatingMode, TargetHeatingMode, Temperature, TemperatureChange,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingAdjustment {
    Radiator(Radiator),
    RadiatorIn15Minutes(Radiator),
    Setpoint(Radiator),
    SetpointIn15Minutes(Radiator),
    HeatingDemand(Radiator),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum AdjustmentDirection {
    MustIncrease = 2,
    ShouldIncrease = 1,
    Hold = 0,
    ShouldDecrease = -1,
    MustDecrease = -2,
    MustOff = -3,
}

pub struct TargetHeatingAdjustmentStateProvider;

impl DerivedStateProvider<TargetHeatingAdjustment, AdjustmentDirection> for TargetHeatingAdjustmentStateProvider {
    fn calculate_current(
        &self,
        id: TargetHeatingAdjustment,
        ctx: &StateCalculationContext,
    ) -> Option<AdjustmentDirection> {
        let radiator = match id {
            TargetHeatingAdjustment::Radiator(radiator) => radiator,
            TargetHeatingAdjustment::RadiatorIn15Minutes(radiator) => radiator,
            TargetHeatingAdjustment::Setpoint(radiator) => radiator,
            TargetHeatingAdjustment::SetpointIn15Minutes(radiator) => radiator,
            TargetHeatingAdjustment::HeatingDemand(radiator) => radiator,
        };
        let heating_zone = radiator.heating_zone();
        let mode = ctx.get(TargetHeatingMode::from_radiator(radiator))?;

        match id {
            //Force ventilation always into full off
            _ if mode.value == HeatingMode::Ventilation => Some(AdjustmentDirection::MustOff),
            //give post-ventilation some time to settle
            _ if mode.value == HeatingMode::PostVentilation && mode.timestamp.elapsed() < t!(5 minutes) => {
                Some(AdjustmentDirection::MustOff)
            }

            TargetHeatingAdjustment::Radiator(radiator) => {
                let radiator_temperature = ctx.get(radiator.surface_temperature())?.value;
                let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;
                let room_temperature = ctx.get(heating_zone.inside_temperature())?.value;

                let radiator_strategy = radiator_strategy(room_temperature, mode.value);
                Some(radiator_strategy.adjustment_direction(radiator_temperature, radiator_roc))
            }
            TargetHeatingAdjustment::RadiatorIn15Minutes(radiator) => {
                let radiator_temperature = ctx.get(Temperature::RadiatorIn15Minutes(radiator))?.value;
                //Assume current ROC still active in 15 minutes
                let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;
                let room_temperature = ctx.get(Temperature::RoomIn15Minutes(heating_zone.room()))?.value;

                let radiator_strategy = radiator_strategy(room_temperature, mode.value);
                Some(radiator_strategy.adjustment_direction(radiator_temperature, radiator_roc))
            }
            TargetHeatingAdjustment::Setpoint(radiator) => {
                let room_temperature = ctx.get(heating_zone.inside_temperature())?.value;
                let room_roc = ctx.get(TemperatureChange::Room(heating_zone.room()))?.value;
                let setpoint = ctx.get(radiator.target_set_point())?.value;

                let setpoint_strategy = setpoint_strategy(setpoint, mode.value);

                Some(setpoint_strategy.adjustment_direction(room_temperature, room_roc))
            }
            TargetHeatingAdjustment::SetpointIn15Minutes(radiator) => {
                let room_temperature = ctx.get(Temperature::RoomIn15Minutes(heating_zone.room()))?.value;
                //Assume current ROC still active in 15 minutes
                let room_roc = ctx.get(TemperatureChange::Room(heating_zone.room()))?.value;
                let setpoint = ctx.get(radiator.target_set_point())?.value;

                let setpoint_strategy = setpoint_strategy(setpoint, mode.value);

                Some(setpoint_strategy.adjustment_direction(room_temperature, room_roc))
            }
            TargetHeatingAdjustment::HeatingDemand(radiator) => {
                use AdjustmentDirection::*;

                let radiator_now = ctx.get(TargetHeatingAdjustment::Radiator(radiator))?.value;
                let radiator_in_15 = ctx.get(TargetHeatingAdjustment::RadiatorIn15Minutes(radiator))?.value;

                //Stop radiator from overheating
                let radiator_adjustment = match (radiator_now, radiator_in_15) {
                    (MustOff, _) => MustOff,
                    (MustDecrease, _) => MustDecrease,
                    (_, MustOff) | (_, MustDecrease) => MustDecrease,
                    (_, ShouldDecrease) => ShouldDecrease,
                    _ => Hold,
                };

                let setpoint_now = ctx.get(TargetHeatingAdjustment::Setpoint(radiator))?.value;
                let setpoint_in_15 = ctx.get(TargetHeatingAdjustment::SetpointIn15Minutes(radiator))?.value;

                //Push for heat by setpoint
                let setpoint_adjustment = setpoint_now.merge(&setpoint_in_15.no_must()).unwrap_or(Hold);

                //Combine both adjustments, radiator has priority in stopping
                match (&radiator_adjustment, &setpoint_adjustment) {
                    (MustOff, _) | (_, MustOff) => MustOff,
                    (MustDecrease, _) => MustDecrease,
                    _ => setpoint_adjustment
                        .merge(&radiator_adjustment)
                        .unwrap_or(setpoint_adjustment),
                }
                .into()
            }
        }
    }
}

fn radiator_strategy(current_room_temperature: DegreeCelsius, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    let max_temp = match mode {
        HeatingMode::Manual(_, _) => 14.0,
        HeatingMode::Comfort => 11.0,
        HeatingMode::EnergySaving => 8.0,
        HeatingMode::Sleep => 8.0,
        HeatingMode::Ventilation => 3.0,
        HeatingMode::PostVentilation => 6.0,
        HeatingMode::Away => 6.0,
    };

    HeatingAdjustmentStrategy::new(
        (
            DegreeCelsius(0.0), //no forced heating caused by radiator temp
            current_room_temperature + DegreeCelsius(max_temp),
        ),
        None,
        DegreeCelsius(3.0),
    )
}

fn setpoint_strategy(setpoint: Range<DegreeCelsius>, mode: HeatingMode) -> HeatingAdjustmentStrategy {
    macro_rules! new {
        ($min:literal - $max:literal, min_heatup = $min_heatup:literal / h, max_overshoot = $max_overshoot:expr) => {
            HeatingAdjustmentStrategy::new(
                setpoint.into(),
                RateOfChange::new(DegreeCelsius($min_heatup), t!(1 hours)).into(),
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

    fn no_must(self) -> AdjustmentDirection {
        use AdjustmentDirection::*;

        match self {
            MustOff => ShouldDecrease,
            MustDecrease => ShouldDecrease,
            MustIncrease => ShouldIncrease,
            other => other,
        }
    }
}

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

    fn adjustment_direction(
        &self,
        current: DegreeCelsius,
        current_change: RateOfChange<DegreeCelsius>,
    ) -> AdjustmentDirection {
        let increasing = current_change > DegreeCelsius(0.1) / t!(1 hours);
        let decreasing = current_change < DegreeCelsius(0.1) / t!(1 hours);

        //Too low -> increase if not heating up fast enough already
        if let Some(ref min_heatup) = self.min_heatup
            && current < self.min
            && &current_change < min_heatup
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use AdjustmentDirection::*;

    #[test]
    fn text_adjustment_ordering() {
        assert!(MustIncrease > ShouldIncrease);
        assert!(ShouldIncrease > Hold);
        assert!(Hold > ShouldDecrease);
        assert!(ShouldDecrease > MustDecrease);
        assert!(MustDecrease > MustOff);

        assert!(MustIncrease > Hold);
        assert!(Hold > ShouldDecrease);
    }
}
