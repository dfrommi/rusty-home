use crate::core::unit::DegreeCelsius;
use crate::home::command::{
    Command, CommandExecution, CommandTarget, EnergySavingDevice, Fan, HeatingTargetState, Notification,
    NotificationAction, NotificationRecipient, NotificationTarget, PowerToggle, Thermostat,
};
use crate::home::state::{ExternalAutoControl, FanActivity, FanAirflow, Opened, Powered, SetPoint};
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
            Command::SetThermostatAmbientTemperature { device, temperature } => {
                is_set_thermmostat_ambient_templerature_reflected_in_state(device, *temperature, api).await
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

async fn is_set_heating_reflected_in_state(
    device: &Thermostat,
    target_state: &HeatingTargetState,
    api: &HomeApi,
) -> Result<bool> {
    let set_point = match device {
        Thermostat::LivingRoom => SetPoint::LivingRoom,
        Thermostat::Bedroom => SetPoint::Bedroom,
        Thermostat::RoomOfRequirements => SetPoint::RoomOfRequirements,
        Thermostat::Kitchen => SetPoint::Kitchen,
        Thermostat::Bathroom => SetPoint::Bathroom,
    }
    .current(api)
    .await?;

    match target_state {
        crate::home::command::HeatingTargetState::Auto => Ok(true), //TODO remove
        crate::home::command::HeatingTargetState::Off => Ok(set_point == DegreeCelsius(0.0)),
        crate::home::command::HeatingTargetState::Heat { temperature, .. } => Ok(&set_point == temperature),
        crate::home::command::HeatingTargetState::WindowOpen => match device {
            Thermostat::LivingRoom => Ok(Opened::LivingRoomRadiatorThermostatBig.current(api).await?),
            Thermostat::Bedroom => Ok(Opened::BedroomRadiatorThermostat.current(api).await?),
            Thermostat::Kitchen => Ok(Opened::KitchenRadiatorThermostat.current(api).await?),
            Thermostat::RoomOfRequirements => Ok(Opened::RoomOfRequirementsThermostat.current(api).await?),
            Thermostat::Bathroom => todo!("No smart heating in bath yet"),
        },
    }
}

async fn is_set_thermmostat_ambient_templerature_reflected_in_state(
    device: &Thermostat,
    temperature: DegreeCelsius,
    api: &HomeApi,
) -> Result<bool> {
    let latest_command = api
        .get_latest_command(
            CommandTarget::SetThermostatAmbientTemperature { device: device.clone() },
            t!(2 hours ago),
        )
        .await?;

    //see guidelines for device at https://www.zigbee2mqtt.io/devices/014G2461.html#external-measured-room-sensor-numeric
    match latest_command {
        Some(CommandExecution {
            command: Command::SetThermostatAmbientTemperature {
                temperature: cmd_temp, ..
            },
            created,
            ..
        }) => Ok(created.elapsed() < t!(1 hours) || (cmd_temp.0 - temperature.0).abs() < 0.1),
        Some(cmd) => anyhow::bail!("Unexpected command type returned: {cmd:?}"),
        None => Ok(false),
    }
}

async fn is_set_power_reflected_in_state(device: &PowerToggle, power_on: bool, api: &HomeApi) -> Result<bool> {
    let powered_item = match device {
        PowerToggle::Dehumidifier => Powered::Dehumidifier,
        PowerToggle::LivingRoomNotificationLight => Powered::LivingRoomNotificationLight,
        PowerToggle::InfraredHeater => Powered::InfraredHeater,
    };

    let powered = powered_item.current(api).await?;
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
async fn is_set_energy_saving_reflected_in_state(device: &EnergySavingDevice, on: bool, api: &HomeApi) -> Result<bool> {
    let state_device = match device {
        crate::home::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };

    let is_energy_saving = state_device.current(api).await?;

    Ok(is_energy_saving == on)
}

async fn is_fan_control_reflected_in_state(device: &Fan, airflow: &FanAirflow, api: &HomeApi) -> Result<bool> {
    let state_device = match device {
        crate::home::command::Fan::LivingRoomCeilingFan => FanActivity::LivingRoomCeilingFan,
        crate::home::command::Fan::BedroomCeilingFan => FanActivity::BedroomCeilingFan,
    };

    let current_flow = state_device.current(api).await?;

    Ok(current_flow == *airflow)
}
