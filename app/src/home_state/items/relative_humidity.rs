use r#macro::{EnumVariants, Id};

use crate::{
    core::{timeseries::DataPoint, unit::Percent},
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    LivingRoom,
    RoomOfRequirements,
    Bedroom,
    Kitchen,
    Bathroom,
}

pub struct RelativeHumidityStateProvider;

impl DerivedStateProvider<RelativeHumidity, Percent> for RelativeHumidityStateProvider {
    fn calculate_current(&self, id: RelativeHumidity, ctx: &StateCalculationContext) -> Option<DataPoint<Percent>> {
        use crate::device_state::RelativeHumidity as DeviceRelativeHumidity;

        ctx.device_state(match id {
            RelativeHumidity::Outside => DeviceRelativeHumidity::Outside,
            RelativeHumidity::LivingRoom => DeviceRelativeHumidity::LivingRoomTado,
            RelativeHumidity::RoomOfRequirements => DeviceRelativeHumidity::RoomOfRequirementsTado,
            RelativeHumidity::Bedroom => DeviceRelativeHumidity::BedroomTado,
            RelativeHumidity::Kitchen => DeviceRelativeHumidity::Kitchen,
            RelativeHumidity::Bathroom => DeviceRelativeHumidity::BathroomShower,
        })
    }
}
