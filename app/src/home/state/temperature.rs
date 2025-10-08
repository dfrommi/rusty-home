use crate::core::HomeApi;
use crate::core::timeseries::DataPoint;
use crate::core::unit::DegreeCelsius;
use crate::home::command::Thermostat;
use crate::port::DataFrameAccess;
use crate::{
    core::time::{DateTime, DateTimeRange},
    port::DataPointAccess,
};
use r#macro::{EnumVariants, Id, mockable};

use crate::core::timeseries::{
    DataFrame,
    interpolate::{Estimatable, algo},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
//TODO remove EnumVariants, only for state-debug
pub enum Temperature {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    BedroomOuterWall,
    Kitchen,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
    Thermostat(Thermostat),
}

impl Estimatable for Temperature {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        algo::linear(at, df)
    }
}

impl DataPointAccess<Temperature> for Temperature {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<DegreeCelsius>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Temperature> for Temperature {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<DegreeCelsius>> {
        api.get_data_frame(self, range).await
    }
}
