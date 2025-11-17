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
pub enum HeatingDemand {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl HeatingDemand {
    pub fn scaling_factor(&self) -> f64 {
        match self {
            HeatingDemand::LivingRoomBig => Thermostat::LivingRoomBig.heating_factor(),
            HeatingDemand::LivingRoomSmall => Thermostat::LivingRoomSmall.heating_factor(),
            HeatingDemand::Bedroom => Thermostat::Bedroom.heating_factor(),
            HeatingDemand::Kitchen => Thermostat::Kitchen.heating_factor(),
            HeatingDemand::RoomOfRequirements => Thermostat::RoomOfRequirements.heating_factor(),
            HeatingDemand::Bathroom => Thermostat::Bathroom.heating_factor(),
        }
    }
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
