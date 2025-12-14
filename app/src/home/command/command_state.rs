use crate::core::unit::{DegreeCelsius, FanAirflow, Percent, RawValue};
use crate::home::LoadBalancedThermostat;
use crate::home::command::{
    Command, CommandExecution, CommandTarget, EnergySavingDevice, Fan, HeatingTargetState, Notification,
    NotificationAction, NotificationRecipient, NotificationTarget, PowerToggle, Thermostat,
};
use crate::home::state::{FanActivity, OpenedArea, PowerAvailable, RawVendorValue, SetPoint, StateSnapshot};
use crate::t;
use anyhow::Result;

use crate::core::HomeApi;
use crate::home::state::EnergySaving;

impl Command {
    pub async fn is_reflected_in_state(&self, snapshot: &StateSnapshot, api: &HomeApi) -> Result<bool> {
        match self {
            Command::SetPower { device, power_on } => is_set_power_reflected_in_state(device, *power_on, snapshot),
            Command::SetHeating { device, target_state } => {
                is_set_heating_reflected_in_state(device, target_state, snapshot)
            }
            Command::SetThermostatAmbientTemperature { device, temperature } => {
                is_set_thermostat_ambient_temperature_reflected_in_state(device, *temperature, api).await
            }
            Command::SetThermostatLoadMean { device, value } => {
                is_set_thermostat_load_mean_reflected_in_state(device, *value, api, snapshot).await
            }
            Command::SetThermostatValveOpeningPosition { device, value } => {
                is_set_thermostat_valve_opening_position_reflected_in_state(device, value, snapshot)
            }
            Command::PushNotify {
                recipient,
                notification,
                action,
            } => is_push_notify_reflected_in_state(recipient, notification, action, api).await,
            Command::SetEnergySaving { device, on } => {
                is_set_energy_saving_reflected_in_state(device, *on, api, snapshot).await
            }
            Command::ControlFan { device, speed } => is_fan_control_reflected_in_state(device, speed, snapshot),
        }
    }
}

fn is_set_heating_reflected_in_state(
    device: &Thermostat,
    target_state: &HeatingTargetState,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let set_point_item = match device {
        Thermostat::LivingRoomBig => SetPoint::LivingRoomBig,
        Thermostat::LivingRoomSmall => SetPoint::LivingRoomSmall,
        Thermostat::Bedroom => SetPoint::Bedroom,
        Thermostat::RoomOfRequirements => SetPoint::RoomOfRequirements,
        Thermostat::Kitchen => SetPoint::Kitchen,
        Thermostat::Bathroom => SetPoint::Bathroom,
    };

    let set_point = snapshot.try_get(set_point_item)?.value;

    match target_state {
        crate::home::command::HeatingTargetState::Off => Ok(set_point == DegreeCelsius(0.0)),
        crate::home::command::HeatingTargetState::Heat { temperature, .. } => Ok(&set_point == temperature), //priority not reflected in state
        crate::home::command::HeatingTargetState::WindowOpen => {
            let open_item = match device {
                Thermostat::LivingRoomBig => OpenedArea::LivingRoomRadiatorThermostatBig,
                Thermostat::LivingRoomSmall => OpenedArea::LivingRoomRadiatorThermostatSmall,
                Thermostat::Bedroom => OpenedArea::BedroomRadiatorThermostat,
                Thermostat::Kitchen => OpenedArea::KitchenRadiatorThermostat,
                Thermostat::RoomOfRequirements => OpenedArea::RoomOfRequirementsThermostat,
                Thermostat::Bathroom => OpenedArea::BathroomThermostat,
            };
            Ok(snapshot.try_get(open_item)?.value)
        }
    }
}

fn is_set_thermostat_valve_opening_position_reflected_in_state(
    device: &Thermostat,
    value: &Percent,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let heating_demand = snapshot.try_get(device.heating_demand())?.value;
    Ok(heating_demand.0 as i32 == value.0 as i32)
}

async fn is_set_thermostat_ambient_temperature_reflected_in_state(
    device: &Thermostat,
    temperature: DegreeCelsius,
    api: &HomeApi,
) -> Result<bool> {
    //TODO from temperature state

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
        }) => {
            //Always send with changes
            if (cmd_temp.0 - temperature.0).abs() > 0.01 {
                return Ok(false); //not reflected
            }

            Ok(created.elapsed() < t!(25 minutes))
        }
        Some(cmd) => anyhow::bail!("Unexpected command type returned: {cmd:?}"),
        None => Ok(false),
    }
}

async fn is_set_thermostat_load_mean_reflected_in_state(
    device: &LoadBalancedThermostat,
    value: RawValue,
    api: &HomeApi,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let current_value = snapshot.try_get(RawVendorValue::AllyLoadMean(device.into()))?.value;
    let latest_command_ts = api
        .get_latest_command(CommandTarget::SetThermostatLoadMean { device: device.clone() }, t!(2 hours ago))
        .await?
        .map(|cmd| cmd.created)
        .unwrap_or(t!(24 hours ago));

    let is_reflected = (current_value.0 - value.0).abs() < 5.0 && latest_command_ts.elapsed() < t!(15 minutes);

    Ok(is_reflected)
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
async fn is_set_energy_saving_reflected_in_state(
    device: &EnergySavingDevice,
    on: bool,
    api: &HomeApi,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let state_device = match device {
        crate::home::command::EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };

    let is_energy_saving = snapshot.try_get(state_device)?.value;

    let recent_command = api
        .get_latest_command(CommandTarget::SetEnergySaving { device: device.clone() }, t!(24 hours ago))
        .await?
        .is_some();

    //sent in last 24 hours and state matches => retrigger daily in case of external changes
    Ok(recent_command && is_energy_saving == on)
}

fn is_fan_control_reflected_in_state(device: &Fan, airflow: &FanAirflow, snapshot: &StateSnapshot) -> Result<bool> {
    let state_device = match device {
        crate::home::command::Fan::LivingRoomCeilingFan => FanActivity::LivingRoomCeilingFan,
        crate::home::command::Fan::BedroomCeilingFan => FanActivity::BedroomCeilingFan,
    };

    let current_flow = snapshot.try_get(state_device)?.value;

    Ok(current_flow == *airflow)
}
