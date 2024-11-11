use std::fmt::Display;

use support::t;

use crate::{
    adapter::persistence::{CommandRepository, DataPoint},
    home_api,
};
use api::{
    command::{CommandExecution, CommandSource, PowerToggle, SetPower, Thermostat},
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
                    Thermostat::LivingRoom,
                    ExternalAutoControl::LivingRoomThermostat,
                    SetPoint::LivingRoom,
                )
                .await
            }
            UserControlled::BedroomThermostat => {
                current_data_point_for_thermostat(
                    Thermostat::Bedroom,
                    ExternalAutoControl::BedroomThermostat,
                    SetPoint::Bedroom,
                )
                .await
            }
            UserControlled::KitchenThermostat => {
                current_data_point_for_thermostat(
                    Thermostat::Kitchen,
                    ExternalAutoControl::KitchenThermostat,
                    SetPoint::Kitchen,
                )
                .await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                current_data_point_for_thermostat(
                    Thermostat::RoomOfRequirements,
                    ExternalAutoControl::RoomOfRequirementsThermostat,
                    SetPoint::RoomOfRequirements,
                )
                .await
            }
            UserControlled::BathroomThermostat => {
                current_data_point_for_thermostat(
                    Thermostat::Bathroom,
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
    if power.timestamp < t!(15 minutes ago) {
        return Ok(DataPoint {
            value: false,
            timestamp: power.timestamp,
        });
    }

    let last_command = home_api()
        .get_latest_command_since(PowerToggle::Dehumidifier, t!(2 minutes ago))
        .await?;

    let was_triggered_by_system = match last_command {
        Some(CommandExecution {
            command: SetPower { power_on, .. },
            source: CommandSource::System(_),
            ..
        }) => power_on == power.value,
        _ => false,
    };

    Ok(DataPoint {
        value: !was_triggered_by_system,
        timestamp: power.timestamp,
    })
}

async fn current_data_point_for_thermostat(
    thermostat: Thermostat,
    auto_mode: ExternalAutoControl,
    set_point: SetPoint,
) -> anyhow::Result<DataPoint<bool>> {
    let (auto_mode_on, set_point, latest_command) = tokio::try_join!(
        auto_mode.current_data_point(),
        set_point.current_data_point(),
        home_api().get_latest_command_since(thermostat, t!(24 hours ago))
    )?;

    let most_recent_change = std::cmp::max(auto_mode_on.timestamp, set_point.timestamp);

    //if no command, then overridden by user only if in manual mode
    if latest_command.is_none() {
        return Ok(auto_mode_on.map_value(|v| !v));
    }

    let latest_command = latest_command.unwrap();
    let triggered_by_user = matches!(latest_command.source, CommandSource::User(_));

    //command after change? -> triggered but roundtrip not yet done -> command source wins
    if latest_command.created > most_recent_change {
        return Ok(DataPoint::new(triggered_by_user, most_recent_change));
    }

    let is_expired = latest_command
        .command
        .get_expiration()
        .map_or(false, |expiration| expiration < t!(now));

    let comand_setting_followed = latest_command
        .command
        .matches(auto_mode_on.value, set_point.value);

    match (is_expired, comand_setting_followed) {
        (true, _) => Ok(DataPoint::new(auto_mode_on.value, most_recent_change)),
        (false, true) => Ok(DataPoint::new(triggered_by_user, most_recent_change)),
        (false, false) => Ok(DataPoint::new(true, most_recent_change)),
    }
}
