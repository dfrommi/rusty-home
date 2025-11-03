mod command_state;

use crate::core::unit::DegreeCelsius;
use crate::core::{id::ExternalId, time::DateTime};
use crate::home::Thermostat;
use derive_more::derive::{Display, From};
use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::home::state::FanAirflow;

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

pub fn is_user_generated(source: &ExternalId) -> bool {
    source.type_name() == "user_trigger_action"
}

pub fn is_system_generated(source: &ExternalId) -> bool {
    !is_user_generated(source)
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
        assert_json_eq!(
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight
            },
            json!({
                "type": "set_power",
                "device": "living_room_notification_light",
            })
        );
    }

    #[test]
    fn set_power_target() {
        assert_json_eq!(
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight
            },
            json!({
                "type": "set_power",
                "device": "living_room_notification_light",
            })
        );
    }

    #[test]
    fn set_heating_off() {
        assert_json_eq!(
            Command::SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Off
            },
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "off"
            })
        );
    }

    #[test]
    fn set_heating_temperature() {
        assert_json_eq!(
            Command::SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Heat {
                    temperature: DegreeCelsius::from(22.5),
                    low_priority: false
                },
            },
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "heat",
                "temperature": 22.5,
                "low_priority": false
            })
        );
    }

    #[test]
    fn set_heating_target() {
        assert_json_eq!(
            CommandTarget::SetHeating {
                device: Thermostat::RoomOfRequirements
            },
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
            })
        );
    }

    #[test]
    fn push_notify() {
        assert_json_eq!(
            Command::PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            },
            json!({
                "type": "push_notify",
                "action": "notify",
                "recipient": "dennis",
                "notification": "window_opened"
            })
        );
    }

    #[test]
    fn push_notify_target() {
        assert_json_eq!(
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: Notification::WindowOpened
            },
            json!({
                "type": "push_notify",
                "recipient": "dennis",
                "notification": "window_opened"
            })
        );
    }

    #[test]
    fn send_notification() {
        assert_json_eq!(
            Command::PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            },
            json!({
                "type": "push_notify",
                "action": "notify",
                "recipient": "dennis",
                "notification": "window_opened"
            })
        );
    }
}
