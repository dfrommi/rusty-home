use r#macro::{EnumVariants, Id};

use crate::{
    automation::Radiator,
    core::unit::{DegreeCelsius, Percent, RateOfChange},
    home_state::{
        HeatingDemand, HeatingMode, SetPoint, TargetHeatingMode, Temperature, TemperatureChange,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
    t,
};

use super::{radiator_strategy, setpoint_strategy};

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
        let Some(modes) = ctx.all_since(TargetHeatingMode::from_radiator(radiator), t!(3 hours ago)) else {
            tracing::warn!(
                "No heating mode data available for radiator {:?}, cannot calculate TargetHeatingAdjustment",
                radiator
            );
            return None;
        };
        let mode = modes.last()?.clone();
        let last_ventilation_finished_within_5_min = modes
            .fulfilled_since(|dp| dp.value != HeatingMode::Ventilation)
            .map(|ts| ts.elapsed() < t!(5 minutes))
            .unwrap_or(false);

        match id {
            //Force ventilation always into full off
            _ if mode.value == HeatingMode::Ventilation => Some(AdjustmentDirection::MustOff),
            //give post-ventilation some time to settle
            _ if last_ventilation_finished_within_5_min => Some(AdjustmentDirection::MustOff),

            TargetHeatingAdjustment::Radiator(radiator) => {
                let radiator_temperature = ctx.get(radiator.surface_temperature())?.value;
                let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;
                let room_temperature = ctx.get(heating_zone.room_temperature())?.value;
                let is_heating = ctx.get(HeatingDemand::Radiator(radiator))?.value > Percent(0.0);

                let radiator_strategy = radiator_strategy(room_temperature, mode.value);
                Some(radiator_strategy.adjustment_direction(radiator_temperature, radiator_roc, is_heating))
            }
            TargetHeatingAdjustment::RadiatorIn15Minutes(radiator) => {
                let radiator_temperature = ctx.get(Temperature::RadiatorIn15Minutes(radiator))?.value;
                //Assume current ROC still active in 15 minutes
                let radiator_roc = ctx.get(TemperatureChange::Radiator(radiator))?.value;
                let room_temperature = ctx.get(Temperature::RoomIn15Minutes(heating_zone.room()))?.value;
                let is_heating = ctx.get(HeatingDemand::Radiator(radiator))?.value > Percent(0.0);

                let radiator_strategy = radiator_strategy(room_temperature, mode.value);
                Some(radiator_strategy.adjustment_direction(radiator_temperature, radiator_roc, is_heating))
            }
            TargetHeatingAdjustment::Setpoint(radiator) => {
                let room_temperature = ctx.get(heating_zone.room_temperature())?.value;
                let room_roc = ctx.get(TemperatureChange::Room(heating_zone.room()))?.value;
                let setpoint = ctx.get(SetPoint::Target(radiator))?.value;
                let is_heating = ctx.get(HeatingDemand::Radiator(radiator))?.value > Percent(0.0);

                let setpoint_strategy = setpoint_strategy(setpoint, mode.value);

                Some(setpoint_strategy.adjustment_direction(room_temperature, room_roc, is_heating))
            }
            TargetHeatingAdjustment::SetpointIn15Minutes(radiator) => {
                let room_temperature = ctx.get(Temperature::RoomIn15Minutes(heating_zone.room()))?.value;
                //Assume current ROC still active in 15 minutes
                let room_roc = ctx.get(TemperatureChange::Room(heating_zone.room()))?.value;
                let setpoint = ctx.get(SetPoint::Target(radiator))?.value;
                let is_heating = ctx.get(HeatingDemand::Radiator(radiator))?.value > Percent(0.0);

                let setpoint_strategy = setpoint_strategy(setpoint, mode.value);

                Some(setpoint_strategy.adjustment_direction(room_temperature, room_roc, is_heating))
            }
            TargetHeatingAdjustment::HeatingDemand(radiator) => {
                use AdjustmentDirection::*;

                let radiator_adjustment = ctx.get(TargetHeatingAdjustment::Radiator(radiator))?.value;

                let setpoint_now = ctx.get(TargetHeatingAdjustment::Setpoint(radiator))?.value;
                let setpoint_in_15 = ctx.get(TargetHeatingAdjustment::SetpointIn15Minutes(radiator))?.value;

                //Push for heat by setpoint
                let setpoint_adjustment = setpoint_now.merge(&setpoint_in_15.no_must()).unwrap_or(Hold);

                //Combine both adjustments, radiator has priority in stopping
                match (&radiator_adjustment, &setpoint_adjustment) {
                    (_, MustIncrease) => MustIncrease,
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

impl super::HeatingAdjustmentStrategy {
    fn adjustment_direction(
        &self,
        current_temp: DegreeCelsius,
        current_change: RateOfChange<DegreeCelsius>,
        is_heating: bool,
    ) -> AdjustmentDirection {
        //TODO is_heating to drive some decisions. Temperature much reach limits in order to work

        //Too low -> increase if not heating up fast enough already until upper bound is reached
        if let Some(ref min_heatup) = self.min_heatup
            && &current_change < min_heatup
            && current_temp < self.max
            && is_heating
        {
            return AdjustmentDirection::MustIncrease;
        }

        //=> not relevant anymore as heating is off until below min
        //In lower band -> increase to avoid undershoot
        // if current_temp >= self.min && current_temp <= self.min + self.band && !is_heating {
        //     return AdjustmentDirection::ShouldIncrease;
        // }

        //no rule for center area -> hold

        //In upper band -> decrease to avoid overshoot
        if current_temp >= self.max - self.band && current_temp <= self.max && is_heating {
            return AdjustmentDirection::ShouldDecrease;
        }

        //Too high -> decrease
        if current_temp > self.max && current_temp <= self.max + self.max_overshoot {
            return AdjustmentDirection::MustDecrease;
        }

        //Too much overshoot -> turn off for cooldown
        if current_temp > self.max + self.max_overshoot {
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
