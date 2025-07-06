use crate::core::timeseries::{
    DataFrame, DataPoint, TimeSeries,
    interpolate::{self, Estimatable},
};

use super::*;
use crate::home::state::{RelativeHumidity, Temperature};
use anyhow::Result;

use r#macro::{EnumVariants, Id};
use support::{
    ValueObject,
    unit::{DegreeCelsius, Percent},
};
use crate::core::time::{DateTime, DateTimeRange};
use tokio::try_join;

#[derive(Debug, Clone, Id, EnumVariants)]
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

impl ValueObject for DewPoint {
    type ValueType = DegreeCelsius;
}

impl Estimatable for DewPoint {
    type Type = DegreeCelsius;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        interpolate::algo::linear(at, df)
    }
}

impl<T> DataPointAccess<DewPoint> for T
where
    T: DataPointAccess<Temperature> + DataPointAccess<RelativeHumidity>,
{
    async fn current_data_point(&self, item: DewPoint) -> Result<DataPoint<DegreeCelsius>> {
        let temperature: DataPoint<DegreeCelsius> =
            self.current_data_point(item.temperature()).await?;
        let humidity: DataPoint<Percent> =
            self.current_data_point(item.relative_humidity()).await?;
        let dewpoint = dewpoint(&temperature, &humidity);

        Ok(dewpoint)
    }
}

impl<T> TimeSeriesAccess<DewPoint> for T
where
    T: TimeSeriesAccess<Temperature> + TimeSeriesAccess<RelativeHumidity>,
{
    async fn series(&self, item: DewPoint, range: DateTimeRange) -> Result<TimeSeries<DewPoint>> {
        let (t_series, h_series) = {
            let temp = item.temperature();
            let humidity = item.relative_humidity();
            try_join!(
                self.series(temp, range.clone()),
                self.series(humidity, range.clone())
            )?
        };

        TimeSeries::combined(&t_series, &h_series, item, calculate_dew_point)
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
pub fn calculate_dew_point(
    temperature: &DegreeCelsius,
    relative_humidity: &Percent,
) -> DegreeCelsius {
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
