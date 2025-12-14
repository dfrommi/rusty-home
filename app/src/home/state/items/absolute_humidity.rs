use super::*;
use crate::home::state::{RelativeHumidity, Temperature};
use anyhow::Result;

use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AbsoluteHumidity {
    BathroomShower,
    BathroomDehumidifier,
    LivingRoom,
    Kitchen,
    KitchenOuterWall,
    RoomOfRequirements,
    LivingRoomTado,
    RoomOfRequirementsTado,
    BedroomTado,
    Outside,
}

pub struct AbsoluteHumidityStateProvider;

impl DerivedStateProvider<AbsoluteHumidity, GramPerCubicMeter> for AbsoluteHumidityStateProvider {
    fn calculate_current(
        &self,
        id: AbsoluteHumidity,
        ctx: &StateCalculationContext,
    ) -> Option<DataPoint<GramPerCubicMeter>> {
        let temperature_dp = ctx.get(id.temperature())?;
        let humidity_dp = ctx.get(id.relative_humidity())?;

        let abs_humidity_value = AbsoluteHumidity::calculate_abs_humidity(temperature_dp.value, humidity_dp.value);

        Some(DataPoint {
            value: abs_humidity_value,
            timestamp: std::cmp::max(temperature_dp.timestamp, humidity_dp.timestamp),
        })
    }
}

impl AbsoluteHumidity {
    fn temperature(&self) -> Temperature {
        match self {
            AbsoluteHumidity::LivingRoom => Temperature::LivingRoom,
            AbsoluteHumidity::BathroomShower => Temperature::BathroomShower,
            AbsoluteHumidity::BathroomDehumidifier => Temperature::Dehumidifier,
            AbsoluteHumidity::Kitchen => Temperature::Kitchen,
            AbsoluteHumidity::KitchenOuterWall => Temperature::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirements => Temperature::RoomOfRequirements,
            AbsoluteHumidity::LivingRoomTado => Temperature::LivingRoomTado,
            AbsoluteHumidity::RoomOfRequirementsTado => Temperature::RoomOfRequirementsTado,
            AbsoluteHumidity::BedroomTado => Temperature::BedroomTado,
            AbsoluteHumidity::Outside => Temperature::Outside,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            AbsoluteHumidity::LivingRoom => RelativeHumidity::LivingRoom,
            AbsoluteHumidity::BathroomShower => RelativeHumidity::BathroomShower,
            AbsoluteHumidity::BathroomDehumidifier => RelativeHumidity::Dehumidifier,
            AbsoluteHumidity::Kitchen => RelativeHumidity::Kitchen,
            AbsoluteHumidity::KitchenOuterWall => RelativeHumidity::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirements => RelativeHumidity::RoomOfRequirements,
            AbsoluteHumidity::LivingRoomTado => RelativeHumidity::LivingRoomTado,
            AbsoluteHumidity::RoomOfRequirementsTado => RelativeHumidity::RoomOfRequirementsTado,
            AbsoluteHumidity::BedroomTado => RelativeHumidity::BedroomTado,
            AbsoluteHumidity::Outside => RelativeHumidity::Outside,
        }
    }

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
