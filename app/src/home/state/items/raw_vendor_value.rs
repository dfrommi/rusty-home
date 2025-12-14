use crate::core::unit::RawValue;
use crate::port::{DataFrameAccess, DataPointAccess};
use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Estimatable, algo},
        },
    },
    home::Thermostat,
};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RawVendorValue {
    AllyLoadEstimate(Thermostat),
    AllyLoadMean(Thermostat),
}

impl Estimatable for RawVendorValue {
    fn interpolate(&self, at: DateTime, df: &DataFrame<RawValue>) -> Option<RawValue> {
        algo::last_seen(at, df)
    }
}

impl DataPointAccess<RawValue> for RawVendorValue {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<RawValue>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<RawValue> for RawVendorValue {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<RawValue>> {
        api.get_data_frame(self, range).await
    }
}
