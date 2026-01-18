use crate::command::CommandClient;
use crate::core::unit::{FanAirflow, Percent};
use crate::home_state::{FanActivity, PowerAvailable, StateSnapshot};
use crate::t;
use anyhow::Result;

use crate::home_state::EnergySaving;

use super::{
    Command, CommandExecution, CommandTarget, EnergySavingDevice, Fan, Notification, NotificationAction,
    NotificationRecipient, NotificationTarget, PowerToggle, Radiator,
};

impl Command {
    pub async fn is_reflected_in_state(
        &self,
        snapshot: &StateSnapshot,
        command_client: &CommandClient,
    ) -> Result<bool> {
        match self {
            Command::SetPower { device, power_on } => is_set_power_reflected_in_state(device, *power_on, snapshot),
            Command::SetThermostatValveOpeningPosition { device, value } => {
                is_set_thermostat_valve_opening_position_reflected_in_state(device, value, snapshot)
            }
            Command::PushNotify {
                recipient,
                notification,
                action,
            } => is_push_notify_reflected_in_state(recipient, notification, action, command_client).await,
            Command::SetEnergySaving { device, on } => {
                is_set_energy_saving_reflected_in_state(device, *on, command_client, snapshot).await
            }
            Command::ControlFan { device, speed } => is_fan_control_reflected_in_state(device, speed, snapshot),
        }
    }
}

fn is_set_thermostat_valve_opening_position_reflected_in_state(
    device: &Radiator,
    value: &Percent,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let heating_demand = snapshot.try_get(device.heating_demand())?.value;
    Ok(heating_demand.0 as i32 == value.0 as i32)
}

fn is_set_power_reflected_in_state(device: &PowerToggle, power_on: bool, snapshot: &StateSnapshot) -> Result<bool> {
    let powered_item = match device {
        PowerToggle::Dehumidifier => PowerAvailable::Dehumidifier,
        PowerToggle::LivingRoomNotificationLight => PowerAvailable::LivingRoomNotificationLight,
        PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
    };

    let powered = snapshot.try_get(powered_item)?.value;
    Ok(powered == power_on)
}

async fn is_push_notify_reflected_in_state(
    recipient: &NotificationRecipient,
    notification: &Notification,
    notify_action: &NotificationAction,
    command_client: &CommandClient,
) -> Result<bool> {
    let target = NotificationTarget {
        recipient: recipient.clone(),
        notification: notification.clone(),
    };

    let latest_command = command_client.get_latest_command(target, t!(24 hours ago)).await?;

    match latest_command {
        Some(CommandExecution {
            command: Command::PushNotify { action, .. },
            ..
        }) => Ok(&action == notify_action),
        _ => Ok(false),
    }
}

//Energy saving not reflected on HA. Trying to guess from actions
async fn is_set_energy_saving_reflected_in_state(
    device: &EnergySavingDevice,
    on: bool,
    command_client: &CommandClient,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let state_device = match device {
        EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };

    let is_energy_saving = snapshot.try_get(state_device)?.value;

    let recent_command = command_client
        .get_latest_command(CommandTarget::SetEnergySaving { device: device.clone() }, t!(24 hours ago))
        .await?
        .is_some();

    //sent in last 24 hours and state matches => retrigger daily in case of external changes
    Ok(recent_command && is_energy_saving == on)
}

fn is_fan_control_reflected_in_state(device: &Fan, airflow: &FanAirflow, snapshot: &StateSnapshot) -> Result<bool> {
    let state_device = match device {
        Fan::LivingRoomCeilingFan => FanActivity::LivingRoomCeilingFan,
        Fan::BedroomCeilingFan => FanActivity::BedroomCeilingFan,
    };

    let current_flow = snapshot.try_get(state_device)?.value;

    Ok(current_flow == *airflow)
}
