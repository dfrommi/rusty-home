use std::fmt::Display;

use crate::core::HomeApi;
use crate::home::command::{Command, CommandTarget};
use crate::home::common::HeatingZone;
use crate::home::state::{FanAirflow, HeatingMode};

use crate::core::planner::{Action, ActionEvaluationResult};

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
            CommandTarget::SetHeating { device } => {
                let heating_zone = HeatingZone::for_thermostat(&device);

                Command::SetHeating {
                    device,
                    target_state: heating_zone.heating_state(&HeatingMode::EnergySaving),
                }
            }
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
