use crate::port::{DataFrameAccess, DataPointAccess};
use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Estimatable, algo},
        },
        unit::DegreeCelsius,
    },
    home::{HeatingZone, state::TargetHeatingMode},
};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum SetPoint {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl Estimatable for SetPoint {
    fn interpolate(&self, at: DateTime, df: &DataFrame<DegreeCelsius>) -> Option<DegreeCelsius> {
        algo::last_seen(at, df)
    }
}

impl DataPointAccess<DegreeCelsius> for SetPoint {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<DegreeCelsius>> {
        //TODO temp workaround for migration
        match self {
            SetPoint::RoomOfRequirements => {
                let mode = TargetHeatingMode::RoomOfRequirements.current_data_point(api).await?;
                let value = HeatingZone::RoomOfRequirements.setpoint_for_mode(&mode.value);
                Ok(DataPoint::new(value, mode.timestamp))
            }

            _ => api.current_data_point(self).await,
        }
    }
}

impl DataFrameAccess<DegreeCelsius> for SetPoint {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<DegreeCelsius>> {
        api.get_data_frame(self, range).await
    }
}
