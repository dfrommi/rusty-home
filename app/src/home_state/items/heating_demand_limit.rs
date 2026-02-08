use crate::{
    automation::Radiator,
    core::{range::Range, unit::Percent},
    home_state::{
        TargetHeatingDemand,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
};
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum HeatingDemandLimit {
    Current(Radiator),
    Target(Radiator),
}

pub struct HeatingDemandLimitStateProvider;

impl DerivedStateProvider<HeatingDemandLimit, Range<Percent>> for HeatingDemandLimitStateProvider {
    fn calculate_current(&self, id: HeatingDemandLimit, ctx: &StateCalculationContext) -> Option<Range<Percent>> {
        use crate::device_state::HeatingDemandLimit as DeviceHeatingDemandLimit;

        match id {
            HeatingDemandLimit::Current(radiator) => {
                let (upper, lower) = match radiator {
                    Radiator::LivingRoomBig => (
                        DeviceHeatingDemandLimit::LivingRoomBigLower,
                        DeviceHeatingDemandLimit::LivingRoomBigUpper,
                    ),
                    Radiator::LivingRoomSmall => (
                        DeviceHeatingDemandLimit::LivingRoomSmallLower,
                        DeviceHeatingDemandLimit::LivingRoomSmallUpper,
                    ),
                    Radiator::Bedroom => {
                        (DeviceHeatingDemandLimit::BedroomLower, DeviceHeatingDemandLimit::BedroomUpper)
                    }
                    Radiator::Kitchen => {
                        (DeviceHeatingDemandLimit::KitchenLower, DeviceHeatingDemandLimit::KitchenUpper)
                    }
                    Radiator::RoomOfRequirements => (
                        DeviceHeatingDemandLimit::RoomOfRequirementsLower,
                        DeviceHeatingDemandLimit::RoomOfRequirementsUpper,
                    ),
                    Radiator::Bathroom => {
                        (DeviceHeatingDemandLimit::BathroomLower, DeviceHeatingDemandLimit::BathroomUpper)
                    }
                };

                let upper_value = ctx.device_state(upper)?.value;
                let lower_value = ctx.device_state(lower).map(|v| v.value).unwrap_or(Percent(0.0));

                Range::new(lower_value, upper_value)
            }
            HeatingDemandLimit::Target(radiator) => {
                let target_demand = ctx.get(TargetHeatingDemand::ControlAndObserve(radiator))?;
                Range::new(Percent(0.0), target_demand.value)
            }
        }
        .into()
    }
}
