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
pub enum HeatingDemand {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for HeatingDemand {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::last_seen(at, df)
    }
}

impl DataPointAccess<Percent> for HeatingDemand {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Percent>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Percent> for HeatingDemand {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Percent>> {
        api.get_data_frame(self, range).await
    }
}
