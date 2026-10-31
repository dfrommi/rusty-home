use crate::{
    core::domain::Radiator,
    core::unit::Percent,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemandLimit {
    Current(Radiator),
}

pub struct HeatingDemandLimitStateProvider;

impl DerivedStateProvider<HeatingDemandLimit, Percent> for HeatingDemandLimitStateProvider {
    fn calculate_current(&self, id: HeatingDemandLimit, ctx: &StateCalculationContext) -> Option<Percent> {
        use crate::device_state::HeatingDemandLimit as DeviceHeatingDemandLimit;

        let HeatingDemandLimit::Current(radiator) = id;
        let upper = match radiator {
            Radiator::LivingRoomBig => DeviceHeatingDemandLimit::LivingRoomBigUpper,
            Radiator::LivingRoomSmall => DeviceHeatingDemandLimit::LivingRoomSmallUpper,
            Radiator::Bedroom => DeviceHeatingDemandLimit::BedroomUpper,
            Radiator::Kitchen => DeviceHeatingDemandLimit::KitchenUpper,
            Radiator::RoomOfRequirements => DeviceHeatingDemandLimit::RoomOfRequirementsUpper,
            Radiator::Bathroom => DeviceHeatingDemandLimit::BathroomUpper,
        };

        Some(ctx.device_state(upper)?.value)
    }
}
