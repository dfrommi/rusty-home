use crate::port::{DataFrameAccess, DataPointAccess};
use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Estimatable, algo},
        },
        unit::Percent,
    },
    home::Thermostat,
};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Load {
    Thermostat(Thermostat),
}

impl Estimatable for Load {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::linear(at, df)
    }
}

impl DataPointAccess<Percent> for Load {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Percent>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Percent> for Load {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Percent>> {
        api.get_data_frame(self, range).await
    }
}
