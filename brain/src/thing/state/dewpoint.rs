use super::*;
use crate::error::Result;
use api::state::{RelativeHumidity, Temperature};

use support::unit::{DegreeCelsius, Percent};

#[derive(Debug, Clone)]
pub enum DewPoint {
    BathroomShower,
    LivingRoomDoor,
    #[allow(dead_code)]
    KitchenOuterWall,
    RoomOfRequirementDoor,
}

impl DewPoint {
    fn dewpoint(
        &self,
        temperature: &DataPoint<DegreeCelsius>,
        relative_humidity: &DataPoint<Percent>,
    ) -> DataPoint<DegreeCelsius> {
        let dp = calculate_dew_point(&temperature.value, &relative_humidity.value);

        DataPoint {
            value: dp,
            timestamp: std::cmp::max(temperature.timestamp, relative_humidity.timestamp),
        }
    }

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

impl DataPointAccess<DegreeCelsius> for DewPoint {
    async fn current_data_point(&self) -> Result<DataPoint<DegreeCelsius>> {
        let t_value = self.temperature().current_data_point().await?;
        let h_value = self.relative_humidity().current_data_point().await?;

        Ok(self.dewpoint(&t_value, &h_value))
    }
}

impl TimeSeriesAccess<DegreeCelsius> for DewPoint {
    async fn series_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<TimeSeries<DegreeCelsius>> {
        println!("Getting TS of {:?}", self);

        let series =
            MultiTimeSeriesAccess::new(self.temperature(), self.relative_humidity(), |t, h| {
                calculate_dew_point(t, h)
            });

        series.series_since(since).await
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
