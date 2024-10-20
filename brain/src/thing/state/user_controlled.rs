use chrono::{Duration, Utc};

use crate::{adapter::persistence::DataPoint, home_api};
use api::command::{CommandExecution, CommandSource, CommandTarget, PowerToggle};

use super::DataPointAccess;

pub enum UserControlled {
    Dehumidifier,
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

impl DataPointAccess<bool> for UserControlled {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        match self {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier().await,
            UserControlled::LivingRoomThermostat => {
                home_api()
                    .get_latest(&api::state::UserControlled::LivingRoomThermostat)
                    .await
            }
            UserControlled::BedroomThermostat => {
                home_api()
                    .get_latest(&api::state::UserControlled::BedroomThermostat)
                    .await
            }
            UserControlled::KitchenThermostat => {
                home_api()
                    .get_latest(&api::state::UserControlled::KitchenThermostat)
                    .await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                home_api()
                    .get_latest(&api::state::UserControlled::RoomOfRequirementsThermostat)
                    .await
            }
            UserControlled::BathroomThermostat => {
                home_api()
                    .get_latest(&api::state::UserControlled::BathroomThermostat)
                    .await
            }
        }
    }
}

async fn current_data_point_for_dehumidifier() -> anyhow::Result<DataPoint<bool>> {
    let state = super::Powered::Dehumidifier.current_data_point().await?;
    let maybe_command = home_api()
        .get_latest_command(&CommandTarget::SetPower(PowerToggle::Dehumidifier))
        .await?;

    let user_controlled = match maybe_command {
        Some(CommandExecution {
            created, source, ..
        }) => source == CommandSource::User && created > Utc::now() - Duration::minutes(15),

        #[allow(unreachable_patterns)] //will be relavant when more commands are added
        Some(c) => {
            tracing::error!("Returned command not matching query: {:?}", c);
            false
        }
        None => true,
    };

    Ok(DataPoint {
        value: user_controlled,
        timestamp: state.timestamp,
    })
}
