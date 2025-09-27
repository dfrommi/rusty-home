use std::fmt::Display;

use crate::core::HomeApi;
use crate::core::unit::DegreeCelsius;
use crate::home::command::{Command, CommandTarget, Thermostat};
use crate::home::state::FanAirflow;

use crate::core::planner::{Action, ActionEvaluationResult};
use crate::t;

#[derive(Debug, Clone)]
pub struct FollowDefaultSetting {
    target: CommandTarget,
}

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self { target }
    }
}

impl Display for FollowDefaultSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FollowDefaultSetting[{}]", self.target)
    }
}

impl Action for FollowDefaultSetting {
    async fn evaluate(&self, _: &HomeApi) -> anyhow::Result<ActionEvaluationResult> {
        let command = match self.target.clone() {
            CommandTarget::SetPower { device } => Command::SetPower {
                device,
                power_on: false,
            },
            //Transition to Zigbee
            CommandTarget::SetHeating {
                device: Thermostat::RoomOfRequirements,
            } => Command::SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: crate::home::command::HeatingTargetState::Heat {
                    temperature: DegreeCelsius(18.0),
                    duration: t!(1 hours),
                },
            },
            CommandTarget::SetHeating { device } => Command::SetHeating {
                device,
                target_state: crate::home::command::HeatingTargetState::Auto,
            },
            CommandTarget::PushNotify {
                recipient,
                notification,
            } => Command::PushNotify {
                action: crate::home::command::NotificationAction::Dismiss,
                notification,
                recipient,
            },
            CommandTarget::SetEnergySaving { device } => Command::SetEnergySaving { device, on: true },
            CommandTarget::ControlFan { device } => Command::ControlFan {
                device,
                speed: FanAirflow::Off,
            },
            CommandTarget::SetThermostatAmbientTemperature { .. } => {
                anyhow::bail!("FollowDefaultSetting cannot be applied to SetThermostatAmbientTemperature")
            }
        };

        Ok(ActionEvaluationResult::Execute(command, super::action_source(self)))
    }
}
