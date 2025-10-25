use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::home::Thermostat;
use crate::home::command::CommandTarget;
use crate::port::DataFrameAccess;
use crate::t;
use r#macro::{EnumVariants, Id, trace_state};

use crate::core::timeseries::DataPoint;
use crate::home::state::Powered;

use crate::home::command::{Command, CommandExecution, PowerToggle, is_system_generated, is_user_generated};

use super::{DataPointAccess, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum UserControlled {
    Dehumidifier,
    LivingRoomThermostatBig,
    LivingRoomThermostatSmall,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
    BathroomThermostat,
}

//TODO try to simplify
// - what is the current state and since when?
// - what is the expected state and since when?
// - is the current state as expected and reached shortly after triggering the command?
impl DataPointAccess<UserControlled> for UserControlled {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        match self {
            UserControlled::Dehumidifier => current_data_point_for_dehumidifier(api).await,
            //check expected state according to last action and compare with current state. Also
            //consider timer expiration
            UserControlled::LivingRoomThermostatBig => {
                current_data_point_for_thermostat(api, self, Thermostat::LivingRoomBig).await
            }
            UserControlled::LivingRoomThermostatSmall => {
                current_data_point_for_thermostat(api, self, Thermostat::LivingRoomSmall).await
            }
            UserControlled::BedroomThermostat => {
                current_data_point_for_thermostat(api, self, Thermostat::Bedroom).await
            }
            UserControlled::KitchenThermostat => {
                current_data_point_for_thermostat(api, self, Thermostat::Kitchen).await
            }
            UserControlled::RoomOfRequirementsThermostat => {
                current_data_point_for_thermostat(api, self, Thermostat::RoomOfRequirements).await
            }
            UserControlled::BathroomThermostat => {
                current_data_point_for_thermostat(api, self, Thermostat::Bathroom).await
            }
        }
    }
}

async fn current_data_point_for_dehumidifier(api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
    let power = Powered::Dehumidifier.current_data_point(api).await?;

    //user-control only valid for 15 minutes
    if power.timestamp < t!(15 minutes ago) {
        tracing::trace!(
            "User controlled not active for dehumidifier, because last state change more than 15 minutes ago"
        );
        return Ok(DataPoint::new(false, power.timestamp));
    }

    let last_command = api
        .get_latest_command(
            CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
            t!(20 minutes ago),
        )
        .await?;

    let system_powered = match last_command {
        Some(CommandExecution {
            command: Command::SetPower { power_on, .. },
            source,
            created,
            ..
        }) if is_system_generated(&source) => DataPoint::new(power_on, created),

        _ => {
            tracing::trace!(
                "User controlled active, because no system command was triggered for last 20 minutes, but power state changed"
            );
            return Ok(DataPoint::new(true, power.timestamp));
        }
    };

    let power_after_command_duration = power.timestamp.elapsed_since(system_powered.timestamp);
    if power_after_command_duration > t!(30 seconds) {
        tracing::trace!(
            "User controlled active, because power state changed more than 30 seconds after last system command"
        );
        return Ok(DataPoint::new(true, power.timestamp));
    }

    let is_as_expected = system_powered.value == power.value;
    let message = if is_as_expected {
        "User controlled not active, because current state matches last system command"
    } else {
        "User controlled active, because current state does not match last system"
    };
    tracing::trace!("{}", message);
    Ok(DataPoint::new(!is_as_expected, power.timestamp))
}

async fn current_data_point_for_thermostat(
    api: &HomeApi,
    _item: &UserControlled,
    thermostat: Thermostat,
) -> anyhow::Result<DataPoint<bool>> {
    let latest_command_exec = api
        .get_latest_command(CommandTarget::SetHeating { device: thermostat }, t!(24 hours ago))
        .await?;
    let timestamp = latest_command_exec
        .as_ref()
        .map(|exec| exec.created)
        .unwrap_or_else(|| t!(now));

    match latest_command_exec {
        Some(command) if is_user_generated(&command.source) => {
            tracing::trace!("User controlled based on latest command source is User");
            Ok(DataPoint::new(true, timestamp))
        }
        _ => {
            tracing::trace!("Not user controlled because latest command is not a user-command");
            Ok(DataPoint::new(false, timestamp))
        }
    }
}

impl Estimatable for UserControlled {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<UserControlled> for UserControlled {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
