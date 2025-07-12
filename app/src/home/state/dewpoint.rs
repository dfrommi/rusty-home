use crate::core::{
    HomeApi,
    timeseries::{
        DataFrame, DataPoint, TimeSeries,
        interpolate::{self, Estimatable},
    },
};

use super::*;
use crate::home::state::{RelativeHumidity, Temperature};
use anyhow::Result;

use crate::core::time::{DateTime, DateTimeRange};
use crate::core::unit::{DegreeCelsius, Percent};
use r#macro::{EnumVariants, Id, mockable};
use tokio::try_join;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum DewPoint {
    BathroomShower,
    LivingRoomDoor,
    #[allow(dead_code)]
    KitchenOuterWall,
    RoomOfRequirementDoor,
}

impl DewPoint {
    fn temperature(&self) -> Temperature {
        match self {
            DewPoint::LivingRoomDoor => Temperature::LivingRoomDoor,
            DewPoint::BathroomShower => Temperature::BathroomShower,
            DewPoint::KitchenOuterWall => Temperature::KitchenOuterWall,
            DewPoint::RoomOfRequirementDoor => Temperature::RoomOfRequirementsDoor,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            DewPoint::LivingRoomDoor => RelativeHumidity::LivingRoomDoor,
            DewPoint::BathroomShower => RelativeHumidity::BathroomShower,
            DewPoint::KitchenOuterWall => RelativeHumidity::KitchenOuterWall,
            DewPoint::RoomOfRequirementDoor => RelativeHumidity::RoomOfRequirementsDoor,
        }
    }
}

impl Estimatable for DewPoint {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<DewPoint> for DewPoint {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<DegreeCelsius>> {
        let temperature: DataPoint<DegreeCelsius> = self.temperature().current_data_point(api).await?;
        let humidity: DataPoint<Percent> = self.relative_humidity().current_data_point(api).await?;
        let dewpoint = dewpoint(&temperature, &humidity);

        Ok(dewpoint)
    }
}

impl TimeSeriesAccess<DewPoint> for DewPoint {
    #[mockable]
    async fn series(&self, range: DateTimeRange, api: &crate::core::HomeApi) -> Result<TimeSeries<DewPoint>> {
        let (t_series, h_series) = {
            let temp = self.temperature();
            let humidity = self.relative_humidity();
            try_join!(temp.series(range.clone(), api), humidity.series(range.clone(), api))?
        };

        TimeSeries::combined(&t_series, &h_series, self.clone(), calculate_dew_point)
    }
}

fn dewpoint(
    temperature: &DataPoint<DegreeCelsius>,
    relative_humidity: &DataPoint<Percent>,
) -> DataPoint<DegreeCelsius> {
    let dp = calculate_dew_point(&temperature.value, &relative_humidity.value);

    DataPoint {
        value: dp,
        timestamp: std::cmp::max(temperature.timestamp, relative_humidity.timestamp),
    }
}

#[allow(dead_code)] //more parameters than currently needed are calculated
pub fn calculate_dew_point(temperature: &DegreeCelsius, relative_humidity: &Percent) -> DegreeCelsius {
    let t: f64 = temperature.into();
    let r: f64 = relative_humidity.into();

    // Constants
    const MW: f64 = 18.016; // Molecular weight of water vapor (kg/kmol)
    const GK: f64 = 8214.3; // Universal gas constant (J/(kmol*K))
    const T0: f64 = 273.15; // Absolute temperature of 0°C (Kelvin)

    let a = if t >= 0.0 { 7.5 } else { 7.6 };
    let b = if t >= 0.0 { 237.3 } else { 240.7 };

    // Temperature in Kelvin
    //let tk = t + T0;

    // Saturation Vapor Pressure (hPa)
    let sdd = 6.1078 * 10f64.powf((a * t) / (b + t));

    // Vapor Pressure (hPa)
    let dd = sdd * (r / 100.0);

    // Absolute Feuchte (g/m3)
    //let af = 10f64.powi(5) * MW / GK * dd / tk;

    // Dew Point Temperature (°C)
    let v = (dd / 6.1078).log10();
    DegreeCelsius((b * v) / (a - v))
}
