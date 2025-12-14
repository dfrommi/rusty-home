use crate::core::timeseries::DataPoint;
use crate::core::unit::DegreeCelsius;
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;
use r#macro::{EnumVariants, Id};

use super::opened::OpenedArea;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AutomaticTemperatureIncrease {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

pub struct AutomaticTemperatureIncreaseStateProvider;

impl DerivedStateProvider<AutomaticTemperatureIncrease, bool> for AutomaticTemperatureIncreaseStateProvider {
    fn calculate_current(
        &self,
        id: AutomaticTemperatureIncrease,
        ctx: &StateCalculationContext,
    ) -> Option<DataPoint<bool>> {
        //TODO define heating schedule lookup and test outside > schedule + 1.0
        let outside_temp = ctx.get(Temperature::Outside)?;

        if outside_temp.value > DegreeCelsius(22.0) {
            tracing::trace!("No automatic increase, temperature outside is too high");
            return Some(DataPoint::new(false, outside_temp.timestamp));
        }

        let (window, temp_sensor) = match id {
            AutomaticTemperatureIncrease::LivingRoom => (OpenedArea::LivingRoomWindowOrDoor, Temperature::LivingRoom),
            AutomaticTemperatureIncrease::Bedroom => (OpenedArea::BedroomWindow, Temperature::Bedroom),
            AutomaticTemperatureIncrease::Kitchen => (OpenedArea::KitchenWindow, Temperature::Kitchen),
            AutomaticTemperatureIncrease::RoomOfRequirements => {
                (OpenedArea::RoomOfRequirementsWindow, Temperature::RoomOfRequirements)
            }
        };

        let window_opened = ctx.get(window)?;
        if window_opened.value {
            tracing::trace!("No automatic temperature increase, because window is open");
            return Some(DataPoint::new(false, window_opened.timestamp));
        }

        let opened_elapsed = window_opened.timestamp.elapsed();

        if opened_elapsed > t!(30 minutes) {
            tracing::trace!(
                "No automatic temperature increase anymore, because window is closed for more than 30 minutes"
            );
            return Some(DataPoint::new(false, window_opened.timestamp));
        }

        if opened_elapsed < t!(5 minutes) {
            tracing::trace!("Automatic temperature increase assumed, because window is open for less than 5 minutes");
            return Some(DataPoint::new(true, window_opened.timestamp));
        }

        let temperature = ctx.all_since(temp_sensor, window_opened.timestamp)?;

        //wait for a measurement. until then assume opened window still has effect
        if temperature.len() < 2 {
            tracing::trace!(
                "Automatic temperature increase assumed, because not enough temperature measurements exist after window was opened"
            );
            return Some(DataPoint::new(true, window_opened.timestamp));
        }

        let current_temperature = temperature.prev_or_at(t!(now));
        let start_temperature = temperature.prev_or_at(t!(5 minutes ago));
        let any_timestamp = current_temperature
            .as_ref()
            .or(start_temperature.as_ref())
            .map(|v| v.timestamp)
            .unwrap_or(window_opened.timestamp);

        match (current_temperature, start_temperature) {
            (Some(current_temperature), Some(start_temperature)) => {
                let diff = current_temperature.value - start_temperature.value;
                //temperature still increasing significantly
                let significant_increase = diff >= DegreeCelsius(0.1);
                let message = if significant_increase {
                    "Automatic temperature increase active, because temperature increased by more than 0.1 degree in last 5 minutes"
                } else {
                    "Automatic temperature increase not active, because temperature increased by less than 0.1 degree in last 5 minutes"
                };
                tracing::trace!("{}", message);
                Some(DataPoint::new(significant_increase, current_temperature.timestamp))
            }
            _ => {
                //Should not happen, covered before
                tracing::trace!(
                    "Automatic temperature increase assumed, because there are not enough temperature measurements"
                );
                Some(DataPoint::new(true, any_timestamp))
            }
        }
    }
}
