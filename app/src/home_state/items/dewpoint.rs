use super::*;
use anyhow::Result;

use crate::automation::HeatingZone;
use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum DewPoint {
    BathroomShower,
    BathroomDehumidifier,
    HeatingZone(HeatingZone),
    Outside,
}

pub struct DewPointStateProvider;

impl DerivedStateProvider<DewPoint, DegreeCelsius> for DewPointStateProvider {
    fn calculate_current(&self, id: DewPoint, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        use crate::device_state::RelativeHumidity as DeviceRelativeHumidity;
        use crate::device_state::Temperature as DeviceTemperature;
        use crate::home_state::items::RelativeHumidity as HomeRelativeHumidity;
        use crate::home_state::items::Temperature as HomeTemperature;

        let temperature_dp = match id {
            DewPoint::HeatingZone(heating_zone) => ctx.get(HomeTemperature::HeatingZone(heating_zone))?,
            DewPoint::Outside => ctx.get(HomeTemperature::Outside)?,
            DewPoint::BathroomShower => ctx.device_state(DeviceTemperature::BathroomShower)?,
            DewPoint::BathroomDehumidifier => ctx.device_state(DeviceTemperature::Dehumidifier)?,
        };

        let humidity_dp = match id {
            DewPoint::HeatingZone(heating_zone) => ctx.get(HomeRelativeHumidity::HeatingZone(heating_zone))?,
            DewPoint::Outside => ctx.get(HomeRelativeHumidity::Outside)?,
            DewPoint::BathroomShower => ctx.device_state(DeviceRelativeHumidity::BathroomShower)?,
            DewPoint::BathroomDehumidifier => ctx.device_state(DeviceRelativeHumidity::Dehumidifier)?,
        };

        let dew_point_value = DewPoint::calculate_dew_point(temperature_dp.value, humidity_dp.value);

        Some(dew_point_value)
    }
}

impl DewPoint {
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
