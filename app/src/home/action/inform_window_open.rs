use std::fmt::Display;

use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::home::command::{Command, CommandSource, Notification, NotificationAction, NotificationRecipient};
use crate::home::state::Presence;
use crate::t;

use crate::{core::planner::SimpleAction, home::state::ColdAirComingIn};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub struct InformWindowOpen {
    recipient: NotificationRecipient,
}

impl InformWindowOpen {
    pub fn new(recipient: NotificationRecipient) -> Self {
        Self {
            recipient: recipient.clone(),
        }
    }
}

impl Display for InformWindowOpen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InformWindowOpen[{}]", self.recipient)
    }
}

impl SimpleAction for InformWindowOpen {
    fn command(&self) -> Command {
        Command::PushNotify {
            action: NotificationAction::Notify,
            notification: Notification::WindowOpened,
            recipient: self.recipient.clone(),
        }
    }

    fn source(&self) -> CommandSource {
        super::action_source(self)
    }

    async fn preconditions_fulfilled(&self, api: &crate::core::HomeApi) -> anyhow::Result<bool> {
        let presence_item = match self.recipient {
            NotificationRecipient::Dennis => Presence::AtHomeDennis,
            NotificationRecipient::Sabine => Presence::AtHomeSabine,
        };

        let at_home = api.current(presence_item).await?;
        if !at_home {
            return Ok(false);
        }

        match cold_air_coming_in(api).await? {
            Some((_, max_dt)) => Ok(t!(now).elapsed_since(max_dt) > t!(3 minutes)),
            None => Ok(false),
        }
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

    let active_values: Vec<DateTime> = result?.into_iter().filter(|v| v.value).map(|v| v.timestamp).collect();

    match (active_values.iter().min(), active_values.iter().max()) {
        (Some(min), Some(max)) => Ok(Some((*min, *max))),
        _ => Ok(None),
    }
}
