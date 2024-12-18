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

pub trait CommandState<C> {
    async fn is_reflected_in_state(&self, command: &C) -> Result<bool>;
}

impl<API> CommandState<Command> for API
where
    API: CommandState<SetPower>
        + CommandState<SetHeating>
        + CommandState<PushNotify>
        + CommandState<SetEnergySaving>,
{
    async fn is_reflected_in_state(&self, command: &Command) -> Result<bool> {
        match command {
            Command::SetPower(command) => self.is_reflected_in_state(command).await,
            Command::SetHeating(command) => self.is_reflected_in_state(command).await,
            Command::PushNotify(command) => self.is_reflected_in_state(command).await,
            Command::SetEnergySaving(command) => self.is_reflected_in_state(command).await,
        }
    }
}

impl<API> CommandState<SetHeating> for API
where
    API: DataPointAccess<SetPoint> + DataPointAccess<ExternalAutoControl>,
{
    async fn is_reflected_in_state(&self, command: &SetHeating) -> Result<bool> {
        let (set_point, auto_mode) = match command.device {
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
            tokio::try_join!(self.current(set_point), self.current(auto_mode))?;

        match command.target_state {
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

impl<API> CommandState<SetPower> for API
where
    API: DataPointAccess<Powered>,
{
    async fn is_reflected_in_state(&self, command: &SetPower) -> Result<bool> {
        let powered_item = match command.device {
            PowerToggle::Dehumidifier => Powered::Dehumidifier,
            PowerToggle::LivingRoomNotificationLight => Powered::LivingRoomNotificationLight,
            PowerToggle::InfraredHeater => Powered::InfraredHeater,
        };

        let powered = self.current(powered_item).await?;

        Ok(powered == command.power_on)
    }
}

impl<API> CommandState<PushNotify> for API
where
    API: CommandAccess<NotificationTarget>,
{
    async fn is_reflected_in_state(&self, command: &PushNotify) -> Result<bool> {
        let target = NotificationTarget {
            recipient: command.recipient.clone(),
            notification: command.notification.clone(),
        };

        let latest_command = self.get_latest_command(target, t!(24 hours ago)).await?;

        match latest_command {
            Some(CommandExecution {
                command: PushNotify { action, .. },
                ..
            }) => Ok(action == command.action),
            _ => Ok(false),
        }
    }
}

//Energy saving not reflected on HA. Trying to guess from actions
impl<API> CommandState<SetEnergySaving> for API
where
    API: DataPointAccess<EnergySaving>,
{
    async fn is_reflected_in_state(&self, command: &SetEnergySaving) -> Result<bool> {
        let state_device = match command.device {
            api::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
        };

        let is_energy_saving = self.current(state_device).await?;

        Ok(is_energy_saving == command.on)
    }
}
