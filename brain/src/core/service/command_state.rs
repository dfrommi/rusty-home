use anyhow::Result;
use api::{
    command::{
        Command, CommandExecution, EnergySavingDevice, HeatingTargetState, Notification,
        NotificationAction, NotificationRecipient, NotificationTarget, PowerToggle, Thermostat,
    },
    state::{ExternalAutoControl, Powered, SetPoint},
};
use support::{t, unit::DegreeCelsius};

use crate::{
    home::state::EnergySaving,
    port::{CommandAccess, DataPointAccess},
};

#[allow(async_fn_in_trait)]
pub trait CommandState {
    async fn is_reflected_in_state(&self, command: &Command) -> Result<bool>;
}

impl<API> CommandState for API
where
    API: DataPointAccess<SetPoint>
        + DataPointAccess<ExternalAutoControl>
        + DataPointAccess<Powered>
        + CommandAccess
        + DataPointAccess<EnergySaving>,
{
    async fn is_reflected_in_state(&self, command: &Command) -> Result<bool> {
        match command {
            Command::SetPower { device, power_on } => {
                is_set_power_reflected_in_state(device, *power_on, self).await
            }
            Command::SetHeating {
                device,
                target_state,
            } => is_set_heating_reflected_in_state(device, target_state, self).await,
            Command::PushNotify {
                recipient,
                notification,
                action,
            } => is_push_notify_reflected_in_state(recipient, notification, action, self).await,
            Command::SetEnergySaving { device, on } => {
                is_set_energy_saving_reflected_in_state(device, *on, self).await
            }
        }
    }
}

async fn is_set_heating_reflected_in_state<API>(
    device: &Thermostat,
    target_state: &HeatingTargetState,
    api: &API,
) -> Result<bool>
where
    API: DataPointAccess<SetPoint> + DataPointAccess<ExternalAutoControl>,
{
    let (set_point, auto_mode) = match device {
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

    let (set_point, auto_mode) = tokio::try_join!(api.current(set_point), api.current(auto_mode))?;

    match target_state {
        api::command::HeatingTargetState::Auto => Ok(auto_mode),
        api::command::HeatingTargetState::Off => Ok(!auto_mode && set_point == DegreeCelsius(0.0)),
        api::command::HeatingTargetState::Heat { temperature, .. } => {
            Ok(!auto_mode && &set_point == temperature)
        }
    }
}

async fn is_set_power_reflected_in_state<API>(
    device: &PowerToggle,
    power_on: bool,
    api: &API,
) -> Result<bool>
where
    API: DataPointAccess<Powered>,
{
    let powered_item = match device {
        PowerToggle::Dehumidifier => Powered::Dehumidifier,
        PowerToggle::LivingRoomNotificationLight => Powered::LivingRoomNotificationLight,
        PowerToggle::InfraredHeater => Powered::InfraredHeater,
    };

    let powered = api.current(powered_item).await?;
    Ok(powered == power_on)
}

async fn is_push_notify_reflected_in_state<API>(
    recipient: &NotificationRecipient,
    notification: &Notification,
    notify_action: &NotificationAction,
    api: &API,
) -> Result<bool>
where
    API: CommandAccess,
{
    let target = NotificationTarget {
        recipient: recipient.clone(),
        notification: notification.clone(),
    };

    let latest_command = api.get_latest_command(target, t!(24 hours ago)).await?;

    match latest_command {
        Some(CommandExecution {
            command: Command::PushNotify { action, .. },
            ..
        }) => Ok(&action == notify_action),
        _ => Ok(false),
    }
}

//Energy saving not reflected on HA. Trying to guess from actions
async fn is_set_energy_saving_reflected_in_state<API>(
    device: &EnergySavingDevice,
    on: bool,
    api: &API,
) -> Result<bool>
where
    API: DataPointAccess<EnergySaving>,
{
    let state_device = match device {
        api::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };

    let is_energy_saving = api.current(state_device).await?;

    Ok(is_energy_saving == on)
}
