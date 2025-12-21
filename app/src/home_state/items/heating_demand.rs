use crate::{
    core::unit::Percent,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
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
    fn calculate_current(&self, id: HeatingDemand, ctx: &StateCalculationContext) -> Option<Percent> {
        use crate::device_state::HeatingDemand as DeviceHeatingDemand;

        ctx.device_state(match id {
            HeatingDemand::LivingRoomBig => DeviceHeatingDemand::LivingRoomBig,
            HeatingDemand::LivingRoomSmall => DeviceHeatingDemand::LivingRoomSmall,
            HeatingDemand::Bedroom => DeviceHeatingDemand::Bedroom,
            HeatingDemand::Kitchen => DeviceHeatingDemand::Kitchen,
            HeatingDemand::RoomOfRequirements => DeviceHeatingDemand::RoomOfRequirements,
            HeatingDemand::Bathroom => DeviceHeatingDemand::Bathroom,
        })
        .map(|dp| dp.value)
    }
}
