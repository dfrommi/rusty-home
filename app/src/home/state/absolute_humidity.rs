use crate::core::{
    HomeApi,
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};

use super::*;
use crate::home::state::{RelativeHumidity, Temperature};
use crate::port::DataFrameAccess;
use anyhow::Result;
use futures::try_join;

use crate::core::time::{DateTime, DateTimeRange};
use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AbsoluteHumidity {
    BathroomShower,
    LivingRoom,
    Kitchen,
    KitchenOuterWall,
    RoomOfRequirements,
    Outside,
}

impl AbsoluteHumidity {
    fn temperature(&self) -> Temperature {
        match self {
            AbsoluteHumidity::LivingRoom => Temperature::LivingRoom,
            AbsoluteHumidity::BathroomShower => Temperature::BathroomShower,
            AbsoluteHumidity::Kitchen => Temperature::Kitchen,
            AbsoluteHumidity::KitchenOuterWall => Temperature::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirements => Temperature::RoomOfRequirements,
            AbsoluteHumidity::Outside => Temperature::Outside,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            AbsoluteHumidity::LivingRoom => RelativeHumidity::LivingRoom,
            AbsoluteHumidity::BathroomShower => RelativeHumidity::BathroomShower,
            AbsoluteHumidity::Kitchen => RelativeHumidity::Kitchen,
            AbsoluteHumidity::KitchenOuterWall => RelativeHumidity::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirements => RelativeHumidity::RoomOfRequirements,
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

impl Estimatable for AbsoluteHumidity {
    fn interpolate(&self, at: DateTime, df: &DataFrame<GramPerCubicMeter>) -> Option<GramPerCubicMeter> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<GramPerCubicMeter> for AbsoluteHumidity {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<GramPerCubicMeter>> {
        let temperature: DataPoint<DegreeCelsius> = self.temperature().current_data_point(api).await?;
        let humidity: DataPoint<Percent> = self.relative_humidity().current_data_point(api).await?;

        let dp = Self::calculate_abs_humidity(temperature.value, humidity.value);

        Ok(DataPoint {
            value: dp,
            timestamp: std::cmp::max(temperature.timestamp, humidity.timestamp),
        })
    }
}

impl DataFrameAccess<GramPerCubicMeter> for AbsoluteHumidity {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<GramPerCubicMeter>> {
        let (t_series, h_series) = {
            let temp = self.temperature();
            let humidity = self.relative_humidity();
            try_join!(temp.series(range.clone(), api), humidity.series(range.clone(), api))?
        };

        DataFrame::<GramPerCubicMeter>::combined(t_series, h_series, Self::calculate_abs_humidity)
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
