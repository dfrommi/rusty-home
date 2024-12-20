use std::fmt::Display;

use api::command::{
    Notification, NotificationAction, NotificationRecipient, NotificationTarget, PushNotify,
};
use support::{t, time::DateTime, DataPoint};

use crate::state::ColdAirComingIn;

use super::{Action, CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub struct InformWindowOpen {
    recipient: NotificationRecipient,
}

impl InformWindowOpen {
    pub fn new(recipient: NotificationRecipient) -> Self {
        Self { recipient }
    }
}

impl<T> Action<T> for InformWindowOpen
where
    T: DataPointAccess<ColdAirComingIn> + CommandAccess<NotificationTarget>,
{
    async fn preconditions_fulfilled(&self, api: &T) -> anyhow::Result<bool> {
        match cold_air_coming_in(api).await? {
            Some((_, max_dt)) => Ok(t!(now).elapsed_since(max_dt) > t!(3 minutes)),
            None => Ok(false),
        }
    }

    fn start_command(&self) -> Option<api::command::Command> {
        Some(api::command::Command::PushNotify(PushNotify {
            action: NotificationAction::Notify,
            notification: Notification::WindowOpened,
            recipient: self.recipient.clone(),
        }))
    }

    fn stop_command(&self) -> Option<api::command::Command> {
        Some(api::command::Command::PushNotify(PushNotify {
            action: NotificationAction::Dismiss,
            notification: Notification::WindowOpened,
            recipient: self.recipient.clone(),
        }))
    }
}

impl Display for InformWindowOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InformWindowOpen[{}]", self.recipient)
    }
}

async fn cold_air_coming_in<T>(api: &T) -> anyhow::Result<Option<(DateTime, DateTime)>>
where
    T: DataPointAccess<ColdAirComingIn>,
{
    let result: anyhow::Result<Vec<DataPoint<bool>>> = futures::future::join_all([
        api.current_data_point(ColdAirComingIn::LivingRoom),
        api.current_data_point(ColdAirComingIn::Bedroom),
        api.current_data_point(ColdAirComingIn::Kitchen),
        api.current_data_point(ColdAirComingIn::RoomOfRequirements),
    ])
    .await
    .into_iter()
    .collect();

    let active_values: Vec<DateTime> = result?
        .into_iter()
        .filter(|v| v.value)
        .map(|v| v.timestamp)
        .collect();

    match (active_values.iter().min(), active_values.iter().max()) {
        (Some(min), Some(max)) => Ok(Some((*min, *max))),
        _ => Ok(None),
    }
}
