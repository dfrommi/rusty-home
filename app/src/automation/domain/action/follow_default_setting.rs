use r#macro::Id;

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{Command, CommandTarget, NotificationDestination, NotificationKind, NotificationOperation};
use crate::core::unit::FanAirflow;

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
            CommandTarget::NotifyPhone {
                recipient,
                notification,
            } => Command::Notify {
                notification,
                target: NotificationDestination::Phone { recipient },
                operation: NotificationOperation::Dismiss,
            },
            CommandTarget::NotifyLight { device } => Command::Notify {
                // The indicator has one configured notification kind today; dismissal clears the light output.
                notification: NotificationKind::WindowOpened,
                target: NotificationDestination::Light { device },
                operation: NotificationOperation::Dismiss,
            },
            CommandTarget::SetEnergySaving { device } => Command::SetEnergySaving { device, on: true },
            CommandTarget::ControlFan { device } => Command::ControlFan {
                device,
                speed: FanAirflow::Off,
            },
            CommandTarget::SetHeating { device } => Command::SetHeating {
                device,
                target_state: crate::command::HeatingTargetState::Off,
            },
            CommandTarget::OpenDoor { .. } => {
                tracing::warn!("No default setting defined for OpenDoor, skipping");
                return Ok(RuleResult::Skip);
            }
        };

        Ok(RuleResult::Execute(command))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::command::{NotificationLight, NotificationRecipient, PowerToggle};
    use crate::home_state::StateSnapshot;

    fn evaluate_default(target: CommandTarget) -> anyhow::Result<Command> {
        let rule = FollowDefaultSetting::new(target);
        match rule.evaluate(&RuleEvaluationContext::new(StateSnapshot::default()))? {
            RuleResult::Execute(command) => Ok(command),
            RuleResult::ExecuteTrigger(_, _) | RuleResult::Skip => anyhow::bail!("expected a default command"),
        }
    }

    #[test]
    fn ambilight_default_is_off() {
        assert_eq!(
            evaluate_default(CommandTarget::SetPower {
                device: PowerToggle::LivingRoomTvAmbilight,
            })
            .unwrap(),
            Command::SetPower {
                device: PowerToggle::LivingRoomTvAmbilight,
                power_on: false,
            }
        );
    }

    #[test]
    fn phone_notification_default_dismisses_its_notification_slot() {
        assert_eq!(
            evaluate_default(CommandTarget::NotifyPhone {
                recipient: NotificationRecipient::Dennis,
                notification: NotificationKind::WindowOpened,
            })
            .unwrap(),
            Command::Notify {
                notification: NotificationKind::WindowOpened,
                target: NotificationDestination::Phone {
                    recipient: NotificationRecipient::Dennis,
                },
                operation: NotificationOperation::Dismiss,
            }
        );
    }

    #[test]
    fn light_notification_default_dismisses_the_light() {
        assert_eq!(
            evaluate_default(CommandTarget::NotifyLight {
                device: NotificationLight::LivingRoom,
            })
            .unwrap(),
            Command::Notify {
                notification: NotificationKind::WindowOpened,
                target: NotificationDestination::Light {
                    device: NotificationLight::LivingRoom,
                },
                operation: NotificationOperation::Dismiss,
            }
        );
    }
}
