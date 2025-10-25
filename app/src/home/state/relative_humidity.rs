use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{Estimatable, algo},
    },
    unit::Percent,
};
use crate::port::{DataFrameAccess, DataPointAccess};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    BedroomOuterWall,
    Kitchen,
    KitchenOuterWall,
    BathroomShower,
    Dehumidifier,
}

impl Estimatable for RelativeHumidity {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::linear(at, df)
    }
}

impl DataPointAccess<RelativeHumidity> for RelativeHumidity {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Percent>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<RelativeHumidity> for RelativeHumidity {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Percent>> {
        api.get_data_frame(self, range).await
    }
}
