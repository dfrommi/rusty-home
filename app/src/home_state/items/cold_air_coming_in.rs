use crate::core::unit::DegreeCelsius;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::{core::timeseries::DataPoint, home_state::Temperature};
use r#macro::{EnumVariants, Id};

use super::OpenedArea;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

pub struct ColdAirComingInStateProvider;

impl DerivedStateProvider<ColdAirComingIn, bool> for ColdAirComingInStateProvider {
    fn calculate_current(&self, id: ColdAirComingIn, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        let outside_temp = ctx.get(Temperature::Outside)?;

        if outside_temp.value > DegreeCelsius(22.0) {
            tracing::trace!("No cold air coming in, temperature outside is too high");
            return Some(DataPoint::new(false, outside_temp.timestamp));
        }

        let window_opened = match id {
            ColdAirComingIn::LivingRoom => ctx.get(OpenedArea::LivingRoomWindowOrDoor)?,
            ColdAirComingIn::Bedroom => ctx.get(OpenedArea::BedroomWindow)?,
            ColdAirComingIn::Kitchen => ctx.get(OpenedArea::KitchenWindow)?,
            ColdAirComingIn::RoomOfRequirements => ctx.get(OpenedArea::RoomOfRequirementsWindow)?,
        };

        let message = if window_opened.value {
            "Cold air coming in, because it's cold outside and window is open"
        } else {
            "No cold air coming in, because window is closed"
        };
        tracing::trace!("{}", message);
        Some(DataPoint::new(window_opened.value, window_opened.timestamp))
    }
}
