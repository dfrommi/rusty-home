use crate::core::ValueObject;
use crate::core::time::DateTime;
use crate::core::unit::DegreeCelsius;
use crate::t;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::DataPoint;
use crate::home::state::{ExternalAutoControl, Powered, SetPoint};

use crate::{
    Database,
    home::{
        command::{Command, CommandExecution, CommandSource, HeatingTargetState, PowerToggle, Thermostat},
        state::macros::result,
    },
};

use super::DataPointAccess;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum UserControlled {
    Dehumidifier,
    LivingRoomThermostat,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

//TODO try to simplify
// - what is the current state and since when?
// - what is the expected state and since when?
// - is the current state as expected and reached shortly after triggering the command?
impl DataPointAccess<UserControlled> for Database {
    async fn current_data_point(&self, item: UserControlled) -> anyhow::Result<DataPoint<bool>> {
        match item {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier(self).await,
            //check expected state according to last action and compare with current state. Also
            //consider timer expiration
            UserControlled::LivingRoomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    item,
                    Thermostat::LivingRoom,
                    ExternalAutoControl::LivingRoomThermostat,
                    SetPoint::LivingRoom,
                )
                .await
            }
            UserControlled::BedroomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    item,
                    Thermostat::Bedroom,
                    ExternalAutoControl::BedroomThermostat,
                    SetPoint::Bedroom,
                )
                .await
            }
            UserControlled::KitchenThermostat => {
                current_data_point_for_thermostat(
                    self,
                    item,
                    Thermostat::Kitchen,
                    ExternalAutoControl::KitchenThermostat,
                    SetPoint::Kitchen,
                )
                .await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                current_data_point_for_thermostat(
                    self,
                    item,
                    Thermostat::RoomOfRequirements,
                    ExternalAutoControl::RoomOfRequirementsThermostat,
                    SetPoint::RoomOfRequirements,
                )
                .await
            }
            UserControlled::BathroomThermostat => {
                current_data_point_for_thermostat(
                    self,
                    item,
                    Thermostat::Bathroom,
                    ExternalAutoControl::BathroomThermostat,
                    SetPoint::Bathroom,
                )
                .await
            }
        }
    }
}

async fn current_data_point_for_dehumidifier(api: &Database) -> anyhow::Result<DataPoint<bool>> {
    let item = UserControlled::Dehumidifier;

    let power = api.current_data_point(Powered::Dehumidifier).await?;

    //user-control only valid for 15 minutes
    if power.timestamp < t!(15 minutes ago) {
        result!(false, power.timestamp, item,
            @power,
            "User controlled not active for dehumidifier, because last state change more than 15 minutes ago"
        );
    }

    let last_command = api
        .get_latest_command(PowerToggle::Dehumidifier, t!(20 minutes ago))
        .await?;

    let system_powered = match last_command {
        Some(CommandExecution {
            command: Command::SetPower { power_on, .. },
            source: CommandSource::System(_),
            created,
            ..
        }) => DataPoint::new(power_on, created),

        _ => {
            result!(true, power.timestamp, item,
                @power,
                "User controlled active, because no system command was triggered for last 20 minutes, but power state changed"
            );
        }
    };

    let power_after_command_duration = power.timestamp.elapsed_since(system_powered.timestamp);
    if power_after_command_duration > t!(30 seconds) {
        result!(true, power.timestamp, item,
            @power,
            @system_powered,
            state_command_diff = %power_after_command_duration,
            "User controlled active, because power state changed more than 30 seconds after last system command"
        );
    }

    let is_as_expected = system_powered.value == power.value;
    result!(!is_as_expected, power.timestamp, item,
        @power,
        @system_powered,
        "{}",
        if is_as_expected {
            "User controlled not active, because current state matches last system command"
        } else {
            "User controlled active, because current state does not match last system"
        },
    );
}

async fn current_data_point_for_thermostat(
    api: &Database,
    item: UserControlled,
    thermostat: Thermostat,
    auto_mode: ExternalAutoControl,
    set_point: SetPoint,
) -> anyhow::Result<DataPoint<bool>> {
    let (auto_mode_on, set_point, latest_command) = tokio::try_join!(
        api.current_data_point(auto_mode),
        api.current_data_point(set_point),
        api.get_latest_command(thermostat, t!(24 hours ago))
    )?;

    //if no command, then overridden by user only if in manual mode
    if latest_command.is_none() {
        result!(!auto_mode_on.value, auto_mode_on.timestamp, item,
            @auto_mode_on,
            "{}",
            if auto_mode_on.value {
                "User controlled not active, because no command found and automatic control is on"
            } else {
                "User controlled active, because no command found and automatic control is off"
            },
        );
    }

    let most_recent_change = std::cmp::max(auto_mode_on.timestamp, set_point.timestamp);
    let latest_command = latest_command.unwrap();
    let triggered_by_user = matches!(latest_command.source, CommandSource::User(_));

    //command after change? -> triggered but roundtrip not yet done -> command source wins
    if latest_command.created > most_recent_change {
        result!(triggered_by_user, most_recent_change, item,
            @auto_mode_on,
            @set_point,
            "{}",
            if triggered_by_user {
                "User controlled assumed active, because latest command is user-command and effect not yet reflected in state."
            } else {
                "User controlled assumed to be inactive, because latest command is system-command and effect not yet reflected in state."
            },
        );
    }

    let is_expired = get_expiration(&latest_command).map_or(false, |expiration| expiration < t!(now));

    let comand_setting_followed = matches(&latest_command, auto_mode_on.value, set_point.value);

    match (is_expired, comand_setting_followed) {
        (true, _) => {
            result!(!auto_mode_on.value, most_recent_change, item,
                @auto_mode_on,
                @set_point,
                "{}",
                if auto_mode_on.value {
                    "User controlled not active, because command expired and automatic control is on"
                } else {
                    "User controlled active, because command expired and automatic control is off"
                },
            );
        }
        (false, true) => {
            result!(triggered_by_user, most_recent_change, item,
                @auto_mode_on,
                @set_point,
                "{}",
                if triggered_by_user {
                    "User controlled is active, because last command was triggered by user and state is still reflected"
                } else {
                    "User controlled is not active, because last command was triggered by system and state is still reflected"
                },
            );
        }
        (false, false) => {
            result!(true, most_recent_change, item,
                @auto_mode_on,
                @set_point,
                "User controlled active, because current state is not reflecting expected state"
            );
        }
    }
}

fn matches(command_execution: &CommandExecution, auto_mode_enabled: bool, set_point: DegreeCelsius) -> bool {
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
            target_state: HeatingTargetState::Heat { duration: until, .. },
            ..
        } => Some(command_execution.created + until.clone()),
        _ => None,
    }
}
