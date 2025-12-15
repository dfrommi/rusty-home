use r#macro::Id;

use crate::command::{Command, CommandTarget, HeatingTargetState, NotificationAction};
use crate::core::unit::{FanAirflow, Percent, RawValue};
use crate::automation::HeatingZone;
use super::{Rule, RuleEvaluationContext, RuleResult};

#[derive(Debug, Clone, Id)]
pub struct FollowDefaultSetting(CommandTarget);

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self(target)
    }
}

impl Rule for FollowDefaultSetting {
    fn evaluate(&self, _: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        let command = match self.0.clone() {
            CommandTarget::SetPower { device } => Command::SetPower {
                device,
                power_on: false,
            },
            CommandTarget::SetHeating { device } => {
                let heating_zone = HeatingZone::for_thermostat(&device);

                Command::SetHeating {
                    device,
                    target_state: HeatingTargetState::Heat {
                        temperature: heating_zone.default_setpoint(),
                        low_priority: true,
                    },
                }
            }
            CommandTarget::PushNotify {
                recipient,
                notification,
            } => Command::PushNotify {
                action: NotificationAction::Dismiss,
                notification,
                recipient,
            },
            CommandTarget::SetEnergySaving { device } => Command::SetEnergySaving { device, on: true },
            CommandTarget::ControlFan { device } => Command::ControlFan {
                device,
                speed: FanAirflow::Off,
            },
            CommandTarget::SetThermostatLoadMean { device } => Command::SetThermostatLoadMean {
                device,
                value: RawValue(-8000.0),
            },
            CommandTarget::SetThermostatAmbientTemperature { .. } => {
                anyhow::bail!("FollowDefaultSetting cannot be applied to SetThermostatAmbientTemperature")
            }
            CommandTarget::SetThermostatValveOpeningPosition { device } => Command::SetThermostatValveOpeningPosition {
                device,
                value: Percent(0.0),
            },
        };

        Ok(RuleResult::Execute(vec![command]))
    }
}
