use super::*;
use anyhow::Result;

use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AbsoluteHumidity {
    Bathroom,
    BathroomShower,
    BathroomDehumidifier,
    LivingRoom,
    Bedroom,
    RoomOfRequirements,
    Outside,
}

pub struct AbsoluteHumidityStateProvider;

impl DerivedStateProvider<AbsoluteHumidity, GramPerCubicMeter> for AbsoluteHumidityStateProvider {
    fn calculate_current(&self, id: AbsoluteHumidity, ctx: &StateCalculationContext) -> Option<GramPerCubicMeter> {
        use crate::device_state::RelativeHumidity as DeviceRelativeHumidity;
        use crate::device_state::Temperature as DeviceTemperature;
        use crate::home_state::items::RelativeHumidity as HomeRelativeHumidity;
        use crate::home_state::items::Temperature as HomeTemperature;

        let temperature_dp = match id {
            AbsoluteHumidity::LivingRoom => ctx.get(HomeTemperature::LivingRoom)?,
            AbsoluteHumidity::Bathroom => ctx.get(HomeTemperature::Bathroom)?,
            AbsoluteHumidity::Outside => ctx.get(HomeTemperature::Outside)?,
            AbsoluteHumidity::Bedroom => ctx.get(HomeTemperature::Bedroom)?,
            AbsoluteHumidity::RoomOfRequirements => ctx.get(HomeTemperature::RoomOfRequirements)?,
            AbsoluteHumidity::BathroomShower => ctx.device_state(DeviceTemperature::BathroomShower)?,
            AbsoluteHumidity::BathroomDehumidifier => ctx.device_state(DeviceTemperature::Dehumidifier)?,
        };

        let humidity_dp = match id {
            AbsoluteHumidity::LivingRoom => ctx.get(HomeRelativeHumidity::LivingRoom)?,
            AbsoluteHumidity::Bathroom => ctx.get(HomeRelativeHumidity::Bathroom)?,
            AbsoluteHumidity::Outside => ctx.get(HomeRelativeHumidity::Outside)?,
            AbsoluteHumidity::Bedroom => ctx.get(HomeRelativeHumidity::Bedroom)?,
            AbsoluteHumidity::RoomOfRequirements => ctx.get(HomeRelativeHumidity::RoomOfRequirements)?,
            AbsoluteHumidity::BathroomShower => ctx.device_state(DeviceRelativeHumidity::BathroomShower)?,
            AbsoluteHumidity::BathroomDehumidifier => ctx.device_state(DeviceRelativeHumidity::Dehumidifier)?,
        };

        let abs_humidity_value = AbsoluteHumidity::calculate_abs_humidity(temperature_dp.value, humidity_dp.value);

        Some(abs_humidity_value)
    }
}

impl AbsoluteHumidity {
    pub fn calculate_abs_humidity(temperature: DegreeCelsius, relative_humidity: Percent) -> GramPerCubicMeter {
        let t: f64 = temperature.into();
        let r: f64 = relative_humidity.into();

        // Constants
        const MW: f64 = 18.016; // Molecular weight of water vapor (kg/kmol)
        const GK: f64 = 8214.3; // Universal gas constant (J/(kmol*K))
        const T0: f64 = 273.15; // Absolute temperature of 0°C (Kelvin)

        let a = if t >= 0.0 { 7.5 } else { 7.6 };
        let b = if t >= 0.0 { 237.3 } else { 240.7 };

        // Saturation Vapor Pressure (hPa)
        let sdd = 6.1078 * 10f64.powf((a * t) / (b + t));

        // Vapor Pressure (hPa)
        let dd = sdd * (r / 100.0);

        // Temperature in Kelvin
        let tk = t + T0;

        // Absolute Feuchte (g/m3)
        let v = 10f64.powi(5) * MW / GK * dd / tk;
        GramPerCubicMeter(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_abs_humidity() {
        let temp = DegreeCelsius(20.0);
        let rh = Percent(50.0);
        let abs_humidity = AbsoluteHumidity::calculate_abs_humidity(temp, rh);
        assert!((abs_humidity.0 - 8.65).abs() < 0.1);
    }
}
