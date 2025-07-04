mod auto_temp_increase;
mod ir_heater_auto_turn_off;
mod support_ventilation_with_fan;
mod ventilation_in_progress;
mod wait_for_sleeping;
mod wait_for_ventilation;

use crate::home::command::Thermostat;

pub use auto_temp_increase::NoHeatingDuringAutomaticTemperatureIncrease;
pub use ir_heater_auto_turn_off::IrHeaterAutoTurnOff;
pub use support_ventilation_with_fan::SupportVentilationWithFan;
pub use ventilation_in_progress::NoHeatingDuringVentilation;
pub use wait_for_sleeping::ExtendHeatingUntilSleeping;
pub use wait_for_ventilation::DeferHeatingUntilVentilationDone;

use super::*;

#[derive(Debug, Clone, derive_more::Display)]
pub enum HeatingZone {
    #[display("LivingRoom")]
    LivingRoom,
    #[display("Bedroom")]
    Bedroom,
    #[display("Kitchen")]
    Kitchen,
    #[display("RoomOfRequirements")]
    RoomOfRequirements,
    #[display("Bathroom")]
    Bathroom,
}

impl HeatingZone {
    pub fn thermostat(&self) -> Thermostat {
        match self {
            HeatingZone::LivingRoom => Thermostat::LivingRoom,
            HeatingZone::Bedroom => Thermostat::Bedroom,
            HeatingZone::Kitchen => Thermostat::Kitchen,
            HeatingZone::RoomOfRequirements => Thermostat::RoomOfRequirements,
            HeatingZone::Bathroom => Thermostat::Bathroom,
        }
    }
}
