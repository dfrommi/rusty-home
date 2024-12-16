use anyhow::Result;
use api::{
    command::{
        Command, CommandExecution, NotificationTarget, PowerToggle, PushNotify, SetEnergySaving,
        SetHeating, SetPower, Thermostat,
    },
    state::{ExternalAutoControl, Powered, SetPoint},
};
use support::{t, unit::DegreeCelsius};

use crate::{
    port::{CommandAccess, DataPointAccess},
    state::EnergySaving,
};

pub trait CommandState<API> {
    async fn is_running(&self, api: &API) -> Result<bool>;
}

impl<API> CommandState<API> for Command
where
    API: DataPointAccess<Powered>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<SetPoint>
        + DataPointAccess<EnergySaving>
        + CommandAccess<NotificationTarget>,
{
    async fn is_running(&self, api: &API) -> Result<bool> {
        match self {
            Command::SetPower(command) => command.is_running(api).await,
            Command::SetHeating(command) => command.is_running(api).await,
            Command::PushNotify(command) => command.is_running(api).await,
            Command::SetEnergySaving(command) => command.is_running(api).await,
        }
    }
}

impl<API> CommandState<API> for SetHeating
where
    API: DataPointAccess<SetPoint> + DataPointAccess<ExternalAutoControl>,
{
    async fn is_running(&self, api: &API) -> Result<bool> {
        let (set_point, auto_mode) = match self.device {
            Thermostat::LivingRoom => (
                SetPoint::LivingRoom,
                ExternalAutoControl::LivingRoomThermostat,
            ),
            Thermostat::Bedroom => (SetPoint::Bedroom, ExternalAutoControl::BedroomThermostat),
            Thermostat::RoomOfRequirements => (
                SetPoint::RoomOfRequirements,
                ExternalAutoControl::RoomOfRequirementsThermostat,
            ),
            Thermostat::Kitchen => (SetPoint::Kitchen, ExternalAutoControl::KitchenThermostat),
            Thermostat::Bathroom => (SetPoint::Bathroom, ExternalAutoControl::BathroomThermostat),
        };

        let (set_point, auto_mode) =
            tokio::try_join!(api.current(set_point), api.current(auto_mode))?;

        match self.target_state {
            api::command::HeatingTargetState::Auto => Ok(auto_mode),
            api::command::HeatingTargetState::Off => {
                Ok(!auto_mode && set_point == DegreeCelsius(0.0))
            }
            api::command::HeatingTargetState::Heat { temperature, .. } => {
                Ok(!auto_mode && set_point == temperature)
            }
        }
    }
}

impl<API: DataPointAccess<Powered>> CommandState<API> for SetPower {
    async fn is_running(&self, api: &API) -> Result<bool> {
        let powered_item = match self.device {
            PowerToggle::Dehumidifier => Powered::Dehumidifier,
            PowerToggle::LivingRoomNotificationLight => Powered::LivingRoomNotificationLight,
            PowerToggle::InfaredHeater => Powered::InfraredHeater,
        };

        let powered = api.current(powered_item).await?;

        Ok(powered == self.power_on)
    }
}

impl<API: CommandAccess<NotificationTarget>> CommandState<API> for PushNotify {
    async fn is_running(&self, api: &API) -> Result<bool> {
        let target = NotificationTarget {
            recipient: self.recipient.clone(),
            notification: self.notification.clone(),
        };

        let latest_command = api.get_latest_command(target, t!(24 hours ago)).await?;

        match latest_command {
            Some(CommandExecution {
                command: PushNotify { action, .. },
                ..
            }) => Ok(action == self.action),
            _ => Ok(false),
        }
    }
}

//Energy saving not reflected on HA. Trying to guess from actions
impl<API> CommandState<API> for SetEnergySaving
where
    API: DataPointAccess<EnergySaving>,
{
    async fn is_running(&self, api: &API) -> Result<bool> {
        let state_device = match self.device {
            api::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
        };

        let is_energy_saving = api.current(state_device).await?;

        Ok(is_energy_saving == self.on)
    }
}
