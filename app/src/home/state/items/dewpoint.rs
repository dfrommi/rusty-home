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
pub enum DewPoint {
    BathroomShower,
    LivingRoom,
    Kitchen,
    KitchenOuterWall,
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
            DewPoint::BathroomShower => Temperature::BathroomShower,
            DewPoint::Kitchen => Temperature::Kitchen,
            DewPoint::KitchenOuterWall => Temperature::KitchenOuterWall,
            DewPoint::RoomOfRequirement => Temperature::RoomOfRequirements,
            DewPoint::Outside => Temperature::Outside,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            DewPoint::LivingRoom => RelativeHumidity::LivingRoom,
            DewPoint::BathroomShower => RelativeHumidity::BathroomShower,
            DewPoint::Kitchen => RelativeHumidity::Kitchen,
            DewPoint::KitchenOuterWall => RelativeHumidity::KitchenOuterWall,
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

impl Estimatable for DewPoint {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<DegreeCelsius> for DewPoint {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<DegreeCelsius>> {
        let temperature: DataPoint<DegreeCelsius> = self.temperature().current_data_point(api).await?;
        let humidity: DataPoint<Percent> = self.relative_humidity().current_data_point(api).await?;

        let dp = Self::calculate_dew_point(temperature.value, humidity.value);

        Ok(DataPoint {
            value: dp,
            timestamp: std::cmp::max(temperature.timestamp, humidity.timestamp),
        })
    }
}

impl DataFrameAccess<DegreeCelsius> for DewPoint {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<DegreeCelsius>> {
        let (t_series, h_series) = {
            let temp = self.temperature();
            let humidity = self.relative_humidity();
            try_join!(temp.series(range.clone(), api), humidity.series(range.clone(), api))?
        };

        DataFrame::<DegreeCelsius>::combined(t_series, h_series, DewPoint::calculate_dew_point)
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
