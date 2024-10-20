use chrono::{Duration, Utc};

use crate::{adapter::persistence::DataPoint, home_api};
use api::command::{CommandSource, PowerToggle};

use super::DataPointAccess;

pub enum UserControlled {
    Dehumidifier,
}

impl DataPointAccess<bool> for UserControlled {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        match self {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier().await,
        }
    }
}

async fn current_data_point_for_dehumidifier() -> anyhow::Result<DataPoint<bool>> {
    let state = super::Powered::Dehumidifier.current_data_point().await?;

    let is_latest_and_by_user = home_api()
        .is_latest_command_since(
            &api::command::Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: state.value,
            },
            Utc::now() - Duration::minutes(15),
            Some(&CommandSource::User),
        )
        .await?;

    Ok(DataPoint {
        value: is_latest_and_by_user,
        timestamp: state.timestamp,
    })
}
