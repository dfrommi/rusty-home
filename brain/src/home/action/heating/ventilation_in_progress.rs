use std::fmt::Display;

use anyhow::Result;
use api::{command::SetHeating, state::SetPoint};

use crate::{
    home::action::{Action, HeatingZone},
    home::state::ColdAirComingIn,
    port::DataPointAccess,
};

use super::ActionExecution;

#[derive(Debug, Clone)]
pub struct NoHeatingDuringVentilation {
    heating_zone: HeatingZone,
    execution: ActionExecution,
}

impl NoHeatingDuringVentilation {
    pub fn new(heating_zone: HeatingZone) -> Self {
        let action_name = format!("NoHeatingDuringVentilation[{}]", &heating_zone);

        Self {
            heating_zone: heating_zone.clone(),
            execution: ActionExecution::from_start_and_stop(
                action_name.as_str(),
                SetHeating {
                    device: heating_zone.thermostat(),
                    target_state: api::command::HeatingTargetState::Off,
                },
                SetHeating {
                    device: heating_zone.thermostat(),
                    target_state: api::command::HeatingTargetState::Auto,
                },
            ),
        }
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

    fn execution(&self) -> &ActionExecution {
        &self.execution
    }
}
