use r#macro::{EnumVariants, Id, mockable, trace_state};

use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};
use crate::port::{DataFrameAccess, DataPointAccess};

use super::KiloCubicMeter;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalWaterConsumption {
    KitchenCold,
    KitchenWarm,
    BathroomCold,
    BathroomWarm,
}

impl Estimatable for TotalWaterConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<KiloCubicMeter>) -> Option<KiloCubicMeter> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<TotalWaterConsumption> for TotalWaterConsumption {
    #[trace_state]
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<KiloCubicMeter>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<TotalWaterConsumption> for TotalWaterConsumption {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<KiloCubicMeter>> {
        api.get_data_frame(self, range).await
    }
}
