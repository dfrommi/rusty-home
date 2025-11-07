use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{Estimatable, algo},
    },
    unit::Lux,
};

use crate::port::{DataFrameAccess, DataPointAccess};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum LightLevel {
    LivingRoom,
    Kitchen,
    RoomOfRequirements,
}

impl Estimatable for LightLevel {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Lux>) -> Option<Lux> {
        algo::linear(at, df)
    }
}

impl DataPointAccess<Lux> for LightLevel {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Lux>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Lux> for LightLevel {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Lux>> {
        api.get_data_frame(self, range).await
    }
}
