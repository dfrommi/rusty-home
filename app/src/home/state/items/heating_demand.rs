use crate::{
    core::{timeseries::DataPoint, unit::Percent},
    home::state::calc::{DerivedStateProvider, StateCalculationContext},
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemand {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

pub struct HeatingDemandStateProvider;

impl DerivedStateProvider<HeatingDemand, Percent> for HeatingDemandStateProvider {
    fn calculate_current(&self, id: HeatingDemand, ctx: &StateCalculationContext) -> Option<DataPoint<Percent>> {
        use crate::device_state::HeatingDemand as DeviceHeatingDemand;

        ctx.device_state(match id {
            HeatingDemand::LivingRoomBig => DeviceHeatingDemand::LivingRoomBig,
            HeatingDemand::LivingRoomSmall => DeviceHeatingDemand::LivingRoomSmall,
            HeatingDemand::Bedroom => DeviceHeatingDemand::Bedroom,
            HeatingDemand::Kitchen => DeviceHeatingDemand::Kitchen,
            HeatingDemand::RoomOfRequirements => DeviceHeatingDemand::RoomOfRequirements,
            HeatingDemand::Bathroom => DeviceHeatingDemand::Bathroom,
        })
    }
}
