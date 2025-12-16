use r#macro::{EnumVariants, Id};

use super::{Rule, RuleEvaluationContext, RuleResult};
use crate::command::{Command, Notification, NotificationAction, NotificationRecipient, PowerToggle};
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::home_state::Presence;
use crate::t;

use crate::home_state::ColdAirComingIn;

#[derive(Debug, Clone, Id, EnumVariants)]
pub enum InformWindowOpen {
    PushNotification(NotificationRecipient),
    NotificationLightLivingRoom,
}

impl Rule for InformWindowOpen {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<super::RuleResult> {
        let command = match self {
            InformWindowOpen::PushNotification(recipient) if self.preconditions_fulfilled_push(recipient, ctx)? => {
                Command::PushNotify {
                    action: NotificationAction::Notify,
                    notification: Notification::WindowOpened,
                    recipient: recipient.clone(),
                }
            }
            InformWindowOpen::NotificationLightLivingRoom if self.preconditions_fulfilled_light(ctx)? => {
                Command::SetPower {
                    device: PowerToggle::LivingRoomNotificationLight,
                    power_on: true,
                }
            }

            _ => return Ok(RuleResult::Skip),
        };

        Ok(RuleResult::Execute(vec![command]))
    }
}

impl InformWindowOpen {
    fn preconditions_fulfilled_push(
        &self,
        recipient: &NotificationRecipient,
        ctx: &RuleEvaluationContext,
    ) -> anyhow::Result<bool> {
        let presence_item = match recipient {
            NotificationRecipient::Dennis => Presence::AtHomeDennis,
            NotificationRecipient::Sabine => Presence::AtHomeSabine,
        };

        let at_home = ctx.current(presence_item)?;

        let cold_air_coming_in = ColdAirComingIn::variants()
            .iter()
            .map(|item| ctx.current_dp(item.clone()))
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(should_send_push_notification(cold_air_coming_in, at_home))
    }

    fn preconditions_fulfilled_light(&self, ctx: &RuleEvaluationContext) -> anyhow::Result<bool> {
        let cold_air_coming_in = ColdAirComingIn::variants()
            .iter()
            .filter(|&it| it != &ColdAirComingIn::LivingRoom)
            .map(|item| ctx.current_dp(item.clone()))
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(should_turn_on_light(cold_air_coming_in))
    }
}

fn should_send_push_notification(cold_air_coming_in: Vec<DataPoint<bool>>, recipient_at_home: bool) -> bool {
    if !recipient_at_home {
        return false;
    }

    let active_values: Vec<DateTime> = cold_air_coming_in
        .into_iter()
        .filter(|v| v.value)
        .map(|v| v.timestamp)
        .collect();

    match (active_values.iter().min(), active_values.iter().max()) {
        (Some(_), Some(max_dt)) => t!(now).elapsed_since(*max_dt) > t!(3 minutes),
        _ => false,
    }
}

fn should_turn_on_light(cold_air_coming_in: Vec<DataPoint<bool>>) -> bool {
    cold_air_coming_in.into_iter().any(|dp| dp.value)
}
