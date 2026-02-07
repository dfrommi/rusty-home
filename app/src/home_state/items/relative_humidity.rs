use r#macro::{EnumVariants, Id};

use crate::{
    automation::Room,
    core::unit::Percent,
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum RelativeHumidity {
    Outside,
    Room(Room),
}

pub struct RelativeHumidityStateProvider;

impl DerivedStateProvider<RelativeHumidity, Percent> for RelativeHumidityStateProvider {
    fn calculate_current(&self, id: RelativeHumidity, ctx: &StateCalculationContext) -> Option<Percent> {
        use crate::device_state::RelativeHumidity as DeviceRelativeHumidity;

        match id {
            RelativeHumidity::Outside => ctx.device_state(DeviceRelativeHumidity::Outside)?.value,
            RelativeHumidity::Room(room) => match room {
                Room::LivingRoom => ctx.device_state(DeviceRelativeHumidity::LivingRoomTado)?.value,
                Room::RoomOfRequirements => ctx.device_state(DeviceRelativeHumidity::RoomOfRequirementsTado)?.value,
                Room::Bedroom => ctx.device_state(DeviceRelativeHumidity::BedroomTado)?.value,
                Room::Kitchen => ctx.device_state(DeviceRelativeHumidity::Kitchen)?.value,
                Room::Bathroom => {
                    let shower = ctx.device_state(DeviceRelativeHumidity::BathroomShower);
                    let dehumidifier = ctx.device_state(DeviceRelativeHumidity::Dehumidifier);

                    match (shower, dehumidifier) {
                        (Some(shower), Some(dehumidifier)) => Percent((shower.value.0 + dehumidifier.value.0) / 2.0),
                        (Some(shower), None) => shower.value,
                        (None, Some(dehumidifier)) => dehumidifier.value,
                        (None, None) => return None,
                    }
                }
            },
        }
        .into()
    }
}
