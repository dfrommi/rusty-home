use crate::automation::RoomWithWindow;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;
use r#macro::{EnumVariants, Id};

use super::Opened;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Ventilation {
    Room(RoomWithWindow),
    AcrossAllRooms,
}

pub struct VentilationStateProvider;

impl DerivedStateProvider<Ventilation, bool> for VentilationStateProvider {
    fn calculate_current(&self, id: Ventilation, ctx: &StateCalculationContext) -> Option<bool> {
        match id {
            Ventilation::Room(room) => {
                let window_opened = ctx.get(Opened::Room(room))?;
                Some(window_opened.value && window_opened.timestamp.elapsed() >= t!(20 seconds))
            }
            Ventilation::AcrossAllRooms => {
                for room in RoomWithWindow::variants().iter() {
                    let window_opened = ctx.get(Opened::Room(*room))?;
                    if !window_opened.value {
                        return Some(false);
                    }
                }
                Some(true)
            }
        }
    }
}
