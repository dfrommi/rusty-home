use r#macro::{EnumVariants, Id, trace_state};

use crate::core::unit::Watt;
use crate::home::state::CurrentPowerUsage;
use crate::home::state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::port::{DataFrameAccess, DataPointAccess};
use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{self, Estimatable},
        },
    },
    home::state::PowerAvailable,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum IsRunning {
    LivingRoomTv,
    RoomOfRequirementsMonitor,
}

pub struct IsRunningStateProvider;

impl DerivedStateProvider<IsRunning, bool> for IsRunningStateProvider {
    fn calculate_current(&self, id: IsRunning, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        match id {
            IsRunning::LivingRoomTv => ctx.get(PowerAvailable::LivingRoomTv),
            IsRunning::RoomOfRequirementsMonitor => {
                let power_usage_dp = ctx.get(CurrentPowerUsage::RoomOfRequirementsMonitor)?;
                Some(DataPoint::new(power_usage_dp.value > Watt(15.0), power_usage_dp.timestamp))
            }
        }
    }
}

impl Estimatable for IsRunning {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataPointAccess<bool> for IsRunning {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        match self {
            IsRunning::LivingRoomTv => api.current_data_point(&PowerAvailable::LivingRoomTv).await,
            IsRunning::RoomOfRequirementsMonitor => Ok(api
                .current_data_point(&CurrentPowerUsage::RoomOfRequirementsMonitor)
                .await?
                .map_value(|&power_usage| power_usage > Watt(15.0))),
        }
    }
}

impl DataFrameAccess<bool> for IsRunning {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        match self {
            IsRunning::LivingRoomTv => api.get_data_frame(&PowerAvailable::LivingRoomTv, range).await,
            IsRunning::RoomOfRequirementsMonitor => {
                let df = api
                    .get_data_frame(&CurrentPowerUsage::RoomOfRequirementsMonitor, range)
                    .await?;
                Ok(df.map(|power_usage| power_usage.value > Watt(15.0)))
            }
        }
    }
}
