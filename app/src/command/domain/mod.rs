mod command_state;

use crate::core::unit::{DegreeCelsius, FanAirflow, Percent, RawValue};
use crate::core::{id::ExternalId, time::DateTime};
use crate::home::{LoadBalancedThermostat, Thermostat};
use crate::trigger::UserTriggerId;
use derive_more::derive::{Display, From};
use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, From, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetPower {
        device: PowerToggle,
        power_on: bool,
    },
    SetHeating {
        device: Thermostat,
        #[serde(flatten)]
        target_state: HeatingTargetState,
    },
    SetThermostatAmbientTemperature {
        device: Thermostat,
        temperature: DegreeCelsius,
    },
    SetThermostatLoadMean {
        device: LoadBalancedThermostat,
        value: RawValue,
    },
    SetThermostatValveOpeningPosition {
        device: Thermostat,
        value: Percent,
    },
    PushNotify {
        action: NotificationAction,
        notification: Notification,
        recipient: NotificationRecipient,
    },
    SetEnergySaving {
        device: EnergySavingDevice,
        on: bool,
    },
    ControlFan {
        device: Fan,
        speed: FanAirflow,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandTarget {
    #[display("SetPower[{}]", device)]
    SetPower { device: PowerToggle },

    #[display("SetHeating[{}]", device)]
    SetHeating { device: Thermostat },

    #[display("SetThermostatAmbientTemperature[{}]", device)]
    SetThermostatAmbientTemperature { device: Thermostat },

    #[display("SetThermostatLoadMean[{}]", device)]
    SetThermostatLoadMean { device: LoadBalancedThermostat },

    #[display("SetThermostatValveOpeningPosition[{}]", device)]
    SetThermostatValveOpeningPosition { device: Thermostat },

    #[display("PushNotify[{} - {}]", notification, recipient)]
    PushNotify {
        recipient: NotificationRecipient,
        notification: Notification,
    },

    #[display("SetEnergySaving[{}]", device)]
    SetEnergySaving { device: EnergySavingDevice },

    #[display("ControlFan[{}]", device)]
    ControlFan { device: Fan },
}

impl From<Command> for CommandTarget {
    fn from(val: Command) -> Self {
        CommandTarget::from(&val)
    }
}

impl From<&Command> for CommandTarget {
    fn from(val: &Command) -> Self {
        match val {
            Command::SetPower { device, .. } => CommandTarget::SetPower { device: device.clone() },
            Command::SetHeating { device, .. } => CommandTarget::SetHeating { device: device.clone() },
            Command::SetThermostatAmbientTemperature { device, .. } => {
                CommandTarget::SetThermostatAmbientTemperature { device: device.clone() }
            }
            Command::SetThermostatLoadMean { device, .. } => {
                CommandTarget::SetThermostatLoadMean { device: device.clone() }
            }
            Command::SetThermostatValveOpeningPosition { device, .. } => {
                CommandTarget::SetThermostatValveOpeningPosition { device: device.clone() }
            }
            Command::PushNotify {
                recipient,
                notification,
                ..
            } => CommandTarget::PushNotify {
                recipient: recipient.clone(),
                notification: notification.clone(),
            },
            Command::SetEnergySaving { device, .. } => CommandTarget::SetEnergySaving { device: device.clone() },
            Command::ControlFan { device, .. } => CommandTarget::ControlFan { device: device.clone() },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub id: i64,
    pub command: Command,
    pub state: CommandState,
    pub created: DateTime,
    pub source: ExternalId,
    pub user_trigger_id: Option<UserTriggerId>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Pending,
    InProgress,
    Success,
    Error(String),
}

impl CommandExecution {
    pub fn is_user_generated(&self) -> bool {
        self.user_trigger_id.is_some()
    }
}

//
// SET POWER
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum PowerToggle {
    Dehumidifier,
    InfraredHeater,
    LivingRoomNotificationLight,
}

//
// SET HEATING
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeatingTargetState {
    Off,
    WindowOpen,
    Heat {
        temperature: DegreeCelsius,
        #[serde(default)]
        low_priority: bool,
    },
}

//
// SEND NOTIFICATION
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Notification {
    WindowOpened,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRecipient {
    Dennis,
    Sabine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAction {
    Notify,
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Eq, Id)]
pub struct NotificationTarget {
    pub recipient: NotificationRecipient,
    pub notification: Notification,
}

impl From<NotificationTarget> for CommandTarget {
    fn from(val: NotificationTarget) -> Self {
        CommandTarget::PushNotify {
            recipient: val.recipient,
            notification: val.notification,
        }
    }
}

//
// SET ENERGY SAVING
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum EnergySavingDevice {
    LivingRoomTv,
}

//
// FAN CONTROL
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Fan {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
}

#[cfg(test)]
mod test {
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn set_power() {
        assert_json_eq!(
            Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            },
            json!({
                "type": "set_power",
                "device": "living_room_notification_light",
                "power_on": true
            })
        );
    }
}
