use crate::core::unit::DegreeCelsius;
use crate::home::command::{
    Command, CommandExecution, EnergySavingDevice, Fan, HeatingTargetState, Notification, NotificationAction,
    NotificationRecipient, NotificationTarget, PowerToggle, Thermostat,
};
use crate::home::state::{ExternalAutoControl, FanActivity, FanAirflow, Powered, SetPoint};
use crate::port::CommandExecutionAccess;
use anyhow::Result;

use crate::{core::HomeApi, t};
use crate::{home::state::EnergySaving, port::DataPointAccess};

impl Command {
    pub async fn is_reflected_in_state(&self, api: &HomeApi) -> Result<bool> {
        match self {
            Command::SetPower { device, power_on } => is_set_power_reflected_in_state(device, *power_on, api).await,
            Command::SetHeating { device, target_state } => {
                is_set_heating_reflected_in_state(device, target_state, api).await
            }
            Command::PushNotify {
                recipient,
                notification,
                action,
            } => is_push_notify_reflected_in_state(recipient, notification, action, api).await,
            Command::SetEnergySaving { device, on } => is_set_energy_saving_reflected_in_state(device, *on, api).await,
            Command::ControlFan { device, speed } => is_fan_control_reflected_in_state(device, speed, api).await,
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
        Thermostat::LivingRoom => (SetPoint::LivingRoom, ExternalAutoControl::LivingRoomThermostat),
        Thermostat::Bedroom => (SetPoint::Bedroom, ExternalAutoControl::BedroomThermostat),
        Thermostat::RoomOfRequirements => {
            (SetPoint::RoomOfRequirements, ExternalAutoControl::RoomOfRequirementsThermostat)
        }
        Thermostat::Kitchen => (SetPoint::Kitchen, ExternalAutoControl::KitchenThermostat),
        Thermostat::Bathroom => (SetPoint::Bathroom, ExternalAutoControl::BathroomThermostat),
    };

    let (set_point, auto_mode) = tokio::try_join!(api.current(set_point), api.current(auto_mode))?;

    match target_state {
        crate::home::command::HeatingTargetState::Auto => Ok(auto_mode),
        crate::home::command::HeatingTargetState::Off => Ok(!auto_mode && set_point == DegreeCelsius(0.0)),
        crate::home::command::HeatingTargetState::Heat { temperature, .. } => {
            Ok(!auto_mode && &set_point == temperature)
        }
    }
}

async fn is_set_power_reflected_in_state<API>(device: &PowerToggle, power_on: bool, api: &API) -> Result<bool>
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

async fn is_push_notify_reflected_in_state(
    recipient: &NotificationRecipient,
    notification: &Notification,
    notify_action: &NotificationAction,
    api: &HomeApi,
) -> Result<bool> {
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
async fn is_set_energy_saving_reflected_in_state<API>(device: &EnergySavingDevice, on: bool, api: &API) -> Result<bool>
where
    API: DataPointAccess<EnergySaving>,
{
    let state_device = match device {
        crate::home::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };

    let is_energy_saving = api.current(state_device).await?;

    Ok(is_energy_saving == on)
}

async fn is_fan_control_reflected_in_state<API>(device: &Fan, airflow: &FanAirflow, api: &API) -> Result<bool>
where
    API: DataPointAccess<FanActivity>,
{
    let state_device = match device {
        crate::home::command::Fan::LivingRoomCeilingFan => FanActivity::LivingRoomCeilingFan,
        crate::home::command::Fan::BedroomCeilingFan => FanActivity::BedroomCeilingFan,
    };

    let current_flow = api.current(state_device).await?;

    Ok(current_flow == *airflow)
}
