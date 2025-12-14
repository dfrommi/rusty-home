use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::DataPoint;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::port::DataFrameAccess;
use crate::t;
use r#macro::{EnumVariants, Id, trace_state};

use super::{DataPointAccess, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergySaving {
    LivingRoomTv,
}

impl DataPointAccess<bool> for EnergySaving {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        api.current_data_point(self).await
    }
}

impl Estimatable for EnergySaving {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<bool> for EnergySaving {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
