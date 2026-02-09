use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{Command, CommandTarget, NotificationAction};
use crate::core::unit::{FanAirflow, Percent};

#[derive(Debug, Clone, Id)]
pub struct FollowDefaultSetting(CommandTarget);

impl FollowDefaultSetting {
    pub fn new(target: CommandTarget) -> Self {
        Self(target)
    }
}

impl Rule for FollowDefaultSetting {
    fn evaluate(&self, _: &RuleEvaluationContext) -> anyhow::Result<RuleResult> {
        tracing::info!("Applying default setting");
        let command = match self.0.clone() {
            CommandTarget::SetPower { device } => Command::SetPower {
                device,
                power_on: false,
            },
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
            CommandTarget::SetThermostatValveOpeningPosition { device } => Command::SetThermostatValveOpeningPosition {
                device,
                value: Percent(0.0),
            },
            CommandTarget::SetHeating { device } => Command::SetHeating {
                device,
                target_state: crate::command::HeatingTargetState::Off,
            },
        };

        Ok(RuleResult::Execute(vec![command]))
    }
}
