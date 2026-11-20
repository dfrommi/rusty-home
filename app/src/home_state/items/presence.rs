use r#macro::{EnumVariants, Id};

use crate::{
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
    t,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Presence {
    AtHomeDennis,
    AtHomeSabine,
    LivingRoomCouch,
    RecentInLivingRoom,
}

pub struct PresenceStateProvider;

impl DerivedStateProvider<Presence, bool> for PresenceStateProvider {
    fn calculate_current(&self, id: Presence, ctx: &StateCalculationContext) -> Option<bool> {
        use crate::device_state::Presence as DevicePresence;

        match id {
            Presence::AtHomeDennis => ctx.device_state(DevicePresence::AtHomeDennis)?.value,
            Presence::AtHomeSabine => ctx.device_state(DevicePresence::AtHomeSabine)?.value,
            Presence::LivingRoomCouch => ctx.device_state(DevicePresence::LivingRoomCouch)?.value,
            Presence::RecentInLivingRoom => ctx
                .device_states(DevicePresence::LivingRoomCouch, t!(5 minutes ago))?
                .any(|dp| dp.value),
        }
        .into()
    }
}
