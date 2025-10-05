use r#macro::Id;

use crate::core::HomeApi;
use crate::home::action::{Rule, RuleResult};
use crate::home::command::{Command, CommandTarget};
use crate::home::common::HeatingZone;
use crate::home::state::{FanAirflow, HeatingMode};

#[derive(Debug, Clone, Id)]
pub struct FollowDefaultSetting(CommandTarget);

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self(target)
    }
}

impl Rule for FollowDefaultSetting {
    async fn evaluate(&self, _: &HomeApi) -> anyhow::Result<RuleResult> {
        let command = match self.0.clone() {
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

        Ok(RuleResult::Execute(vec![command]))
    }
}
