use std::fmt::Display;

use support::{t, time::DateTime, unit::DegreeCelsius, DataPoint, ValueObject};

use api::{
    command::{
        Command, CommandExecution, CommandSource, HeatingTargetState, PowerToggle, Thermostat,
    },
    state::{ExternalAutoControl, Powered, SetPoint},
};

use super::{CommandAccess, DataPointAccess};

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

impl ValueObject for UserControlled {
    type ValueType = bool;
}

impl<T> DataPointAccess<UserControlled> for T
where
    T: DataPointAccess<Powered>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<SetPoint>
        + CommandAccess,
{
    async fn current_data_point(&self, item: UserControlled) -> anyhow::Result<DataPoint<bool>> {
        match item {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier(self).await,
            //check expected state according to last action and compare with current state. Also
            //consider timer expiration
            UserControlled::LivingRoomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    Thermostat::LivingRoom,
                    ExternalAutoControl::LivingRoomThermostat,
                    SetPoint::LivingRoom,
                )
                .await
            }
            UserControlled::BedroomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    Thermostat::Bedroom,
                    ExternalAutoControl::BedroomThermostat,
                    SetPoint::Bedroom,
                )
                .await
            }
            UserControlled::KitchenThermostat => {
                current_data_point_for_thermostat(
                    self,
                    Thermostat::Kitchen,
                    ExternalAutoControl::KitchenThermostat,
                    SetPoint::Kitchen,
                )
                .await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                current_data_point_for_thermostat(
                    self,
                    Thermostat::RoomOfRequirements,
                    ExternalAutoControl::RoomOfRequirementsThermostat,
                    SetPoint::RoomOfRequirements,
                )
                .await
            }
            UserControlled::BathroomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    Thermostat::Bathroom,
                    ExternalAutoControl::BathroomThermostat,
                    SetPoint::Bathroom,
                )
                .await
            }
        }
    }
}

async fn current_data_point_for_dehumidifier(
    api: &(impl DataPointAccess<Powered> + CommandAccess),
) -> anyhow::Result<DataPoint<bool>> {
    let power = api.current_data_point(Powered::Dehumidifier).await?;

    //user-control only valid for 15 minutes
    if power.timestamp < t!(15 minutes ago) {
        return Ok(DataPoint {
            value: false,
            timestamp: power.timestamp,
        });
    }

    let last_command = api
        .get_latest_command(PowerToggle::Dehumidifier, t!(2 minutes ago))
        .await?;

    let was_triggered_by_system = match last_command {
        Some(CommandExecution {
            command: Command::SetPower { power_on, .. },
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
    api: &(impl DataPointAccess<ExternalAutoControl> + DataPointAccess<SetPoint> + CommandAccess),
    thermostat: Thermostat,
    auto_mode: ExternalAutoControl,
    set_point: SetPoint,
) -> anyhow::Result<DataPoint<bool>> {
    let (auto_mode_on, set_point, latest_command) = tokio::try_join!(
        api.current_data_point(auto_mode),
        api.current_data_point(set_point),
        api.get_latest_command(thermostat, t!(24 hours ago))
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

    let is_expired =
        get_expiration(&latest_command).map_or(false, |expiration| expiration < t!(now));

    let comand_setting_followed = matches(&latest_command, auto_mode_on.value, set_point.value);

    match (is_expired, comand_setting_followed) {
        (true, _) => Ok(DataPoint::new(!auto_mode_on.value, most_recent_change)),
        (false, true) => Ok(DataPoint::new(triggered_by_user, most_recent_change)),
        (false, false) => Ok(DataPoint::new(true, most_recent_change)),
    }
}

fn matches(
    command_execution: &CommandExecution,
    auto_mode_enabled: bool,
    set_point: DegreeCelsius,
) -> bool {
    match command_execution.command {
        Command::SetHeating {
            target_state: HeatingTargetState::Auto,
            ..
        } => auto_mode_enabled,
        Command::SetHeating {
            target_state: HeatingTargetState::Heat { temperature, .. },
            ..
        } => !auto_mode_enabled && set_point == temperature,
        Command::SetHeating {
            target_state: HeatingTargetState::Off,
            ..
        } => !auto_mode_enabled && set_point == DegreeCelsius(0.0),
        _ => false,
    }
}

pub fn get_expiration(command_execution: &CommandExecution) -> Option<DateTime> {
    match &command_execution.command {
        Command::SetHeating {
            target_state:
                HeatingTargetState::Heat {
                    duration: until, ..
                },
            ..
        } => Some(command_execution.created.clone() + until.clone()),
        _ => None,
    }
}
