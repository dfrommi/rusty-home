use r#macro::{EnumVariants, Id, trace_state};

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
    home::Thermostat,
};

use super::HeatingUnit;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum TotalRadiatorConsumption {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl TotalRadiatorConsumption {
    pub fn scaling_factor(&self) -> f64 {
        match self {
            TotalRadiatorConsumption::LivingRoomBig => Thermostat::LivingRoomBig.heating_factor(),
            TotalRadiatorConsumption::LivingRoomSmall => Thermostat::LivingRoomSmall.heating_factor(),
            TotalRadiatorConsumption::Bedroom => Thermostat::Bedroom.heating_factor(),
            TotalRadiatorConsumption::Kitchen => Thermostat::Kitchen.heating_factor(),
            TotalRadiatorConsumption::RoomOfRequirements => Thermostat::RoomOfRequirements.heating_factor(),
            TotalRadiatorConsumption::Bathroom => Thermostat::Bathroom.heating_factor(),
        }
    }
}

impl Estimatable for TotalRadiatorConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<HeatingUnit>) -> Option<HeatingUnit> {
        interpolate::algo::linear(at, df)
    }
}

impl DataPointAccess<HeatingUnit> for TotalRadiatorConsumption {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<HeatingUnit>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<HeatingUnit> for TotalRadiatorConsumption {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<HeatingUnit>> {
        api.get_data_frame(self, range).await
    }
}
