use std::fmt::Display;

use crate::{
    core::{
        HomeApi,
        planner::{Action, ActionEvaluationResult},
    },
    home::{
        command::{PowerToggle, Thermostat},
        state::UserControlled,
    },
};

use super::DataPointAccess;
use crate::home::command::CommandTarget;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct KeepUserOverride {
    user_controlled: UserControlled,
    target: CommandTarget,
}

impl KeepUserOverride {
    pub fn new(user_controlled: UserControlled) -> Self {
        let target = match &user_controlled {
            UserControlled::RoomOfRequirementsThermostat => CommandTarget::SetHeating {
                device: Thermostat::RoomOfRequirements,
            },
            UserControlled::LivingRoomThermostat => CommandTarget::SetHeating {
                device: Thermostat::LivingRoom,
            },
            UserControlled::BedroomThermostat => CommandTarget::SetHeating {
                device: Thermostat::Bedroom,
            },
            UserControlled::KitchenThermostat => CommandTarget::SetHeating {
                device: Thermostat::Kitchen,
            },
            UserControlled::BathroomThermostat => CommandTarget::SetHeating {
                device: Thermostat::Bathroom,
            },
            UserControlled::Dehumidifier => CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
        };

        Self {
            user_controlled,
            target,
        }
    }
}

impl Action for KeepUserOverride {
    async fn evaluate(&self, api: &HomeApi) -> Result<ActionEvaluationResult> {
        let fulfilled = self.user_controlled.current(api).await?;

        if fulfilled {
            Ok(ActionEvaluationResult::Lock(self.target.clone()))
        } else {
            Ok(ActionEvaluationResult::Skip)
        }
    }
}

impl Display for KeepUserOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeepUserOverride[{}]", self.user_controlled)
    }
}
