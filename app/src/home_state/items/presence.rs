use r#macro::{EnumVariants, Id};

use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    LivingRoomCouch,
    BedroomBed,
}

pub struct PresenceStateProvider;

impl DerivedStateProvider<Presence, bool> for PresenceStateProvider {
    fn calculate_current(&self, id: Presence, ctx: &StateCalculationContext) -> Option<bool> {
        use crate::device_state::Presence as DevicePresence;

        ctx.device_state(match id {
            Presence::AtHomeDennis => DevicePresence::AtHomeDennis,
            Presence::AtHomeSabine => DevicePresence::AtHomeSabine,
            Presence::LivingRoomCouch => DevicePresence::LivingRoomCouch,
            Presence::BedroomBed => DevicePresence::BedroomBed,
        })
        .map(|dp| dp.value)
    }
}
