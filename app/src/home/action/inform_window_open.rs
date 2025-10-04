use std::fmt::Display;

use futures::future::try_join_all;

use crate::core::HomeApi;
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::home::command::{
    Command, CommandSource, Notification, NotificationAction, NotificationRecipient, PowerToggle,
};
use crate::home::state::Presence;
use crate::t;

use crate::{core::planner::SimpleAction, home::state::ColdAirComingIn};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum InformWindowOpen {
    PushNotification(NotificationRecipient),
    NotificationLightLivingRoom,
}

impl Display for InformWindowOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InformWindowOpen::PushNotification(recipient) => write!(f, "InformWindowOpen[{}]", recipient),
            InformWindowOpen::NotificationLightLivingRoom => write!(f, "RequestClosingWindow"),
        }
    }
}

impl SimpleAction for InformWindowOpen {
    fn command(&self) -> Command {
        match self {
            InformWindowOpen::PushNotification(recipient) => Command::PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: recipient.clone(),
            },
            InformWindowOpen::NotificationLightLivingRoom => Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
        }
    }

    fn source(&self) -> CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &HomeApi) -> anyhow::Result<bool> {
        match self {
            InformWindowOpen::PushNotification(notification_recipient) => {
                self.preconditions_fulfilled_push(notification_recipient, api).await
            }
            InformWindowOpen::NotificationLightLivingRoom => self.preconditions_fulfilled_light(api).await,
        }
    }
}

impl InformWindowOpen {
    async fn preconditions_fulfilled_push(
        &self,
        recipient: &NotificationRecipient,
        api: &HomeApi,
    ) -> anyhow::Result<bool> {
        let presence_item = match recipient {
            NotificationRecipient::Dennis => Presence::AtHomeDennis,
            NotificationRecipient::Sabine => Presence::AtHomeSabine,
        };
        let at_home = presence_item.current(api).await?;

        let cold_air_coming_in = try_join_all(
            ColdAirComingIn::variants()
                .iter()
                .map(|item| item.current_data_point(api)),
        )
        .await?;

        Ok(should_send_push_notification(cold_air_coming_in, at_home))
    }

    async fn preconditions_fulfilled_light(&self, api: &HomeApi) -> anyhow::Result<bool> {
        let cold_air_coming_in = try_join_all(
            ColdAirComingIn::variants()
                .iter()
                .filter(|&it| it != &ColdAirComingIn::LivingRoom)
                .map(|item| item.current_data_point(api)),
        )
        .await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::command::NotificationRecipient;

    #[test]
    fn display_includes_recipient() {
        assert_eq!(
            InformWindowOpen::PushNotification(NotificationRecipient::Dennis).to_string(),
            "InformWindowOpen[Dennis]"
        );

        assert_eq!(
            InformWindowOpen::NotificationLightLivingRoom.to_string(),
            "RequestClosingWindow"
        );
    }
}
