use std::fmt::Display;

use crate::home::command::Command;
use anyhow::Result;

use crate::{
    core::planner::SimpleAction,
    home::{action::HeatingZone, state::ColdAirComingIn},
    port::DataPointAccess,
};

#[derive(Debug, Clone)]
pub struct NoHeatingDuringVentilation {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringVentilation {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self {
            heating_zone: heating_zone.clone(),
        }
    }
}

impl Display for NoHeatingDuringVentilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoHeatingDuringVentilation[{}]", self.heating_zone)
    }
}

impl SimpleAction for NoHeatingDuringVentilation {
    fn command(&self) -> Command {
        Command::SetHeating {
            device: self.heating_zone.thermostat(),
            target_state: crate::home::command::HeatingTargetState::Off,
        }
    }

    fn source(&self) -> crate::home::command::CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> Result<bool> {
        api.current(match self.heating_zone {
            HeatingZone::LivingRoom => ColdAirComingIn::LivingRoom,
            HeatingZone::Bedroom => ColdAirComingIn::Bedroom,
            HeatingZone::Kitchen => ColdAirComingIn::Kitchen,
            HeatingZone::RoomOfRequirements => ColdAirComingIn::RoomOfRequirements,
            HeatingZone::Bathroom => ColdAirComingIn::Bedroom,
        })
        .await
    }
}
