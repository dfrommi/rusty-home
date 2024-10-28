use std::fmt::Display;

use chrono::{Duration, Utc};

use crate::{adapter::persistence::DataPoint, home_api};
use api::{
    command::{CommandSource, PowerToggle},
    state::{ExternalAutoControl, Powered, SetPoint},
};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum UserControlled {
    Dehumidifier,
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

impl Display for UserControlled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let target = match self {
            UserControlled::Dehumidifier => "dehumidifier",
            UserControlled::LivingRoomThermostat => "living room thermostat",
            UserControlled::BedroomThermostat => "bedroom thermostat",
            UserControlled::KitchenThermostat => "kitchen thermostat",
            UserControlled::RoomOfRequirementsThermostat => "room of requirements thermostat",
            UserControlled::BathroomThermostat => "bathroom thermostat",
        };
        write!(f, "UserControlled[{}]", target)
    }
}

impl DataPointAccess<bool> for UserControlled {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        match self {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier().await,
            //check expected state according to last action and compare with current state. Also
            //consider timer expiration
            UserControlled::LivingRoomThermostat => {
                current_data_point_for_thermostat(
                    ExternalAutoControl::LivingRoomThermostat,
                    SetPoint::LivingRoom,
                )
                .await
            }
            UserControlled::BedroomThermostat => {
                current_data_point_for_thermostat(
                    ExternalAutoControl::BedroomThermostat,
                    SetPoint::Bedroom,
                )
                .await
            }
            UserControlled::KitchenThermostat => {
                current_data_point_for_thermostat(
                    ExternalAutoControl::KitchenThermostat,
                    SetPoint::Kitchen,
                )
                .await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                current_data_point_for_thermostat(
                    ExternalAutoControl::RoomOfRequirementsThermostat,
                    SetPoint::RoomOfRequirements,
                )
                .await
            }
            UserControlled::BathroomThermostat => {
                current_data_point_for_thermostat(
                    ExternalAutoControl::BathroomThermostat,
                    SetPoint::Bathroom,
                )
                .await
            }
        }
    }
}

async fn current_data_point_for_dehumidifier() -> anyhow::Result<DataPoint<bool>> {
    let power = Powered::Dehumidifier.current_data_point().await?;

    //user-control only valid for 15 minutes
    if power.timestamp < Utc::now() - Duration::minutes(15) {
        return Ok(DataPoint {
            value: false,
            timestamp: power.timestamp,
        });
    }

    let was_triggered_by_system = home_api()
        .is_latest_command_since(
            &api::command::Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on: power.value,
            },
            power.timestamp - Duration::minutes(2),
            Some(&CommandSource::System),
        )
        .await?;

    Ok(DataPoint {
        value: !was_triggered_by_system,
        timestamp: power.timestamp,
    })
}

//TODO check last executed command and timestamp
async fn current_data_point_for_thermostat(
    auto_mode: ExternalAutoControl,
    set_point: SetPoint,
) -> anyhow::Result<DataPoint<bool>> {
    let (auto_mode_on, set_point) = tokio::try_join!(
        auto_mode.current_data_point(),
        set_point.current_data_point()
    )?;

    let set_point_value = *set_point.value.as_ref();
    //assumption: user never sets to 0.0
    let system_triggered = auto_mode_on.value
        || set_point_value == 0.0
        || set_point_value.fract() == 0.0
        || set_point_value.fract() == 0.5;

    Ok(DataPoint {
        value: !system_triggered,
        timestamp: std::cmp::max(auto_mode_on.timestamp, set_point.timestamp),
    })
}
