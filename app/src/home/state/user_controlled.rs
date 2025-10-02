use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::home::command::CommandTarget;
use crate::port::DataFrameAccess;
use crate::t;
use r#macro::{EnumVariants, Id, mockable};

use crate::core::timeseries::DataPoint;
use crate::home::state::Powered;

use crate::home::{
    command::{Command, CommandExecution, CommandSource, PowerToggle, Thermostat},
    state::macros::result,
};

use super::{DataPointAccess, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum UserControlled {
    Dehumidifier,
    LivingRoomThermostatBig,
    LivingRoomThermostatSmall,
    BedroomThermostat,
    KitchenThermostat,
    RoomOfRequirementsThermostat,
}

//TODO try to simplify
// - what is the current state and since when?
// - what is the expected state and since when?
// - is the current state as expected and reached shortly after triggering the command?
impl DataPointAccess<UserControlled> for UserControlled {
    #[mockable]
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
        }
    }
}

async fn current_data_point_for_dehumidifier(api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
    let item = UserControlled::Dehumidifier;

    let power = Powered::Dehumidifier.current_data_point(api).await?;

    //user-control only valid for 15 minutes
    if power.timestamp < t!(15 minutes ago) {
        result!(false, power.timestamp, item,
            @power,
            "User controlled not active for dehumidifier, because last state change more than 15 minutes ago"
        );
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
    api: &HomeApi,
    item: &UserControlled,
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
        Some(CommandExecution {
            source: CommandSource::User(_),
            ..
        }) => {
            result!(true, timestamp, item, "User controlled based on latest command source is User");
        }
        _ => {
            result!(
                false,
                timestamp,
                item,
                "Not user controlled because latest command is not a user-command"
            );
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
