use super::*;
use crate::home::state::{RelativeHumidity, Temperature};
use anyhow::Result;

use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum DewPoint {
    BathroomShower,
    LivingRoom,
    RoomOfRequirement,
    Outside,
}

pub struct DewPointStateProvider;

impl DerivedStateProvider<DewPoint, DegreeCelsius> for DewPointStateProvider {
    fn calculate_current(&self, id: DewPoint, ctx: &StateCalculationContext) -> Option<DataPoint<DegreeCelsius>> {
        let temperature_dp = ctx.get(id.temperature())?;
        let humidity_dp = ctx.get(id.relative_humidity())?;

        let dew_point_value = DewPoint::calculate_dew_point(temperature_dp.value, humidity_dp.value);

        Some(DataPoint {
            value: dew_point_value,
            timestamp: std::cmp::max(temperature_dp.timestamp, humidity_dp.timestamp),
        })
    }
}

impl DewPoint {
    fn temperature(&self) -> Temperature {
        match self {
            DewPoint::LivingRoom => Temperature::LivingRoom,
            DewPoint::BathroomShower => Temperature::Bathroom,
            DewPoint::RoomOfRequirement => Temperature::RoomOfRequirements,
            DewPoint::Outside => Temperature::Outside,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            DewPoint::LivingRoom => RelativeHumidity::LivingRoom,
            DewPoint::BathroomShower => RelativeHumidity::Bathroom,
            DewPoint::RoomOfRequirement => RelativeHumidity::RoomOfRequirements,
            DewPoint::Outside => RelativeHumidity::Outside,
        }
    }

    pub fn calculate_dew_point(temperature: DegreeCelsius, relative_humidity: Percent) -> DegreeCelsius {
        let t: f64 = temperature.into();
        let r: f64 = relative_humidity.into();

        let a = if t >= 0.0 { 7.5 } else { 7.6 };
        let b = if t >= 0.0 { 237.3 } else { 240.7 };

        // Saturation Vapor Pressure (hPa)
        let sdd = 6.1078 * 10f64.powf((a * t) / (b + t));

        // Vapor Pressure (hPa)
        let dd = sdd * (r / 100.0);

        // Dew Point Temperature (°C)
        let v = (dd / 6.1078).log10();
        let td = (b * v) / (a - v);

        DegreeCelsius(td)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dew_point_calculation() {
        let temperature = DegreeCelsius(20.0);
        let humidity = Percent(50.0);
        let dew_point = DewPoint::calculate_dew_point(temperature, humidity);
        assert!((dew_point.0 - 9.26).abs() < 0.1); // Expected dew point around 9.26°C
    }
}
