use crate::automation::RoomWithWindow;
use crate::core::unit::DegreeCelsius;
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use r#macro::{EnumVariants, Id};

use super::Opened;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ColdAirComingIn {
    Room(RoomWithWindow),
}

pub struct ColdAirComingInStateProvider;

impl DerivedStateProvider<ColdAirComingIn, bool> for ColdAirComingInStateProvider {
    fn calculate_current(&self, id: ColdAirComingIn, ctx: &StateCalculationContext) -> Option<bool> {
        let outside_temp = ctx.get(Temperature::Outside)?;

        if outside_temp.value > DegreeCelsius(22.0) {
            tracing::trace!("No cold air coming in, temperature outside is too high");
            return Some(false);
        }

        let window_opened = match id {
            ColdAirComingIn::Room(room) => ctx.get(Opened::Room(room))?,
        };

        let message = if window_opened.value {
            "Cold air coming in, because it's cold outside and window is open"
        } else {
            "No cold air coming in, because window is closed"
        };
        tracing::trace!("{}", message);
        Some(window_opened.value)
    }
}
