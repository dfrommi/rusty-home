use r#macro::{EnumVariants, Id, trace_state};

use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};
use crate::port::{DataFrameAccess, DataPointAccess};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Powered {
    Dehumidifier,
    LivingRoomNotificationLight,
    InfraredHeater,
    LivingRoomTv,
}

impl Estimatable for Powered {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataPointAccess<Powered> for Powered {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Powered> for Powered {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        api.get_data_frame(self, range).await
    }
}
