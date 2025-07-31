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
use crate::core::unit::{DegreeCelsius, GramsPerCubicMeter, Percent};
use r#macro::{EnumVariants, Id, mockable};

use super::dewpoint::calculate_humidity_values;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AbsoluteHumidity {
    BathroomShower,
    LivingRoomDoor,
    #[allow(dead_code)]
    KitchenOuterWall,
    RoomOfRequirementDoor,
}

impl AbsoluteHumidity {
    fn temperature(&self) -> Temperature {
        match self {
            AbsoluteHumidity::LivingRoomDoor => Temperature::LivingRoomDoor,
            AbsoluteHumidity::BathroomShower => Temperature::BathroomShower,
            AbsoluteHumidity::KitchenOuterWall => Temperature::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirementDoor => Temperature::RoomOfRequirementsDoor,
        }
    }

    fn relative_humidity(&self) -> RelativeHumidity {
        match self {
            AbsoluteHumidity::LivingRoomDoor => RelativeHumidity::LivingRoomDoor,
            AbsoluteHumidity::BathroomShower => RelativeHumidity::BathroomShower,
            AbsoluteHumidity::KitchenOuterWall => RelativeHumidity::KitchenOuterWall,
            AbsoluteHumidity::RoomOfRequirementDoor => RelativeHumidity::RoomOfRequirementsDoor,
        }
    }
}

impl Estimatable for AbsoluteHumidity {
    fn interpolate(&self, at: DateTime, df: &DataFrame<GramsPerCubicMeter>) -> Option<GramsPerCubicMeter> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<AbsoluteHumidity> for AbsoluteHumidity {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<GramsPerCubicMeter>> {
        let temperature: DataPoint<DegreeCelsius> = self.temperature().current_data_point(api).await?;
        let humidity: DataPoint<Percent> = self.relative_humidity().current_data_point(api).await?;

        let temperature = &temperature;
        let relative_humidity = &humidity;
        let calculation = calculate_humidity_values(&temperature.value, &relative_humidity.value);

        Ok(DataPoint {
            value: calculation.absolute_humidity,
            timestamp: std::cmp::max(temperature.timestamp, relative_humidity.timestamp),
        })
    }
}

impl DataFrameAccess<AbsoluteHumidity> for AbsoluteHumidity {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<GramsPerCubicMeter>> {
        let (t_series, h_series) = {
            let temp = self.temperature();
            let humidity = self.relative_humidity();
            try_join!(temp.series(range.clone(), api), humidity.series(range.clone(), api))?
        };

        DataFrame::<GramsPerCubicMeter>::combined(&t_series, &h_series, |temp, hum| {
            calculate_humidity_values(temp, hum).absolute_humidity
        })
    }
}
