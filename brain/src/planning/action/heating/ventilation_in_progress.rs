use std::fmt::Display;

use anyhow::Result;
use api::{
    command::{Command, SetHeating},
    state::SetPoint,
};

use crate::{
    planning::action::{Action, HeatingZone},
    port::DataPointAccess,
    state::ColdAirComingIn,
};

#[derive(Debug, Clone)]
pub struct NoHeatingDuringVentilation {
    heating_zone: HeatingZone,
}

impl NoHeatingDuringVentilation {
    pub fn new(heating_zone: HeatingZone) -> Self {
        Self { heating_zone }
    }
}

impl Display for NoHeatingDuringVentilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoHeatingDuringVentilation[{}]", self.heating_zone)
    }
}

impl<T> Action<T> for NoHeatingDuringVentilation
where
    T: DataPointAccess<ColdAirComingIn>
        + DataPointAccess<ColdAirComingIn>
        + DataPointAccess<SetPoint>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> Result<bool> {
        api.current(match self.heating_zone {
            HeatingZone::LivingRoom => ColdAirComingIn::LivingRoom,
            HeatingZone::Bedroom => ColdAirComingIn::Bedroom,
            HeatingZone::Kitchen => ColdAirComingIn::Kitchen,
            HeatingZone::RoomOfRequirements => ColdAirComingIn::RoomOfRequirements,
            HeatingZone::Bathroom => ColdAirComingIn::Bedroom,
        })
        .await
    }

    fn start_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Off,
            }
            .into(),
        )
    }

    fn stop_command(&self) -> Option<Command> {
        Some(
            SetHeating {
                device: self.heating_zone.thermostat(),
                target_state: api::command::HeatingTargetState::Auto,
            }
            .into(),
        )
    }
}
