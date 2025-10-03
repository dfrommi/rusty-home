use r#macro::{EnumVariants, Id};

use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};
use crate::port::{DataFrameAccess, DataPointAccess};

use super::HeatingUnit;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for TotalRadiatorConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<HeatingUnit>) -> Option<HeatingUnit> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<TotalRadiatorConsumption> for TotalRadiatorConsumption {
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<HeatingUnit>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<TotalRadiatorConsumption> for TotalRadiatorConsumption {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<HeatingUnit>> {
        api.get_data_frame(self, range).await
    }
}
