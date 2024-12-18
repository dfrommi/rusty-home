use derive_more::derive::{Display, From};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use support::{time::DateTime, unit::DegreeCelsius};

pub mod db;

#[derive(Debug, Clone, PartialEq, From, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetPower(SetPower),
    SetHeating(SetHeating),
    PushNotify(PushNotify),
    SetEnergySaving(SetEnergySaving),
}

impl CommandId for Command {
    type CommandType = Command;
}

#[derive(Debug, Clone, PartialEq, Eq, From, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandTarget {
    SetPower {
        device: PowerToggle,
    },
    SetHeating {
        device: Thermostat,
    },
    PushNotify {
        recipient: NotificationRecipient,
        notification: Notification,
    },
    SetEnergySaving {
        device: EnergySavingDevice,
    },
}

pub trait CommandId: Into<CommandTarget> {
    type CommandType: Into<Command> + DeserializeOwned;
}

impl CommandId for CommandTarget {
    type CommandType = Command;
}

impl From<Command> for CommandTarget {
    fn from(val: Command) -> Self {
        CommandTarget::from(&val)
    }
}

impl From<&Command> for CommandTarget {
    fn from(val: &Command) -> Self {
        match val {
            Command::SetPower(SetPower { device, .. }) => CommandTarget::SetPower {
                device: device.clone(),
            },
            Command::SetHeating(SetHeating { device, .. }) => CommandTarget::SetHeating {
                device: device.clone(),
            },
            Command::PushNotify(PushNotify {
                recipient,
                notification,
                ..
            }) => CommandTarget::PushNotify {
                recipient: recipient.clone(),
                notification: notification.clone(),
            },
            Command::SetEnergySaving(SetEnergySaving { device, .. }) => {
                CommandTarget::SetEnergySaving {
                    device: device.clone(),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CommandExecution<C: Into<Command>> {
    pub id: i64,
    pub command: C,
    pub state: CommandState,
    pub created: DateTime,
    pub source: CommandSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Pending,
    InProgress,
    Success,
    Error(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSource {
    System(String),
    User(String),
}

//
// SET POWER
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetPower {
    pub device: PowerToggle,
    pub power_on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerToggle {
    Dehumidifier,
    InfraredHeater,
    LivingRoomNotificationLight,
}
impl CommandId for PowerToggle {
    type CommandType = SetPower;
}

//
// SET HEATING
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetHeating {
    pub device: Thermostat,
    #[serde(flatten)]
    pub target_state: HeatingTargetState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thermostat {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeatingTargetState {
    Auto,
    Off, //TODO support off-timer (not supported via HA)
    Heat {
        temperature: DegreeCelsius,
        until: DateTime,
    },
}

impl CommandId for Thermostat {
    type CommandType = SetHeating;
}

impl SetHeating {
    pub fn matches(&self, auto_mode_enabled: bool, set_point: DegreeCelsius) -> bool {
        match self {
            SetHeating {
                target_state: HeatingTargetState::Auto,
                ..
            } => auto_mode_enabled,
            SetHeating {
                target_state: HeatingTargetState::Heat { temperature, .. },
                ..
            } => !auto_mode_enabled && set_point == *temperature,
            SetHeating {
                target_state: HeatingTargetState::Off,
                ..
            } => !auto_mode_enabled && set_point == DegreeCelsius(0.0),
        }
    }

    pub fn get_expiration(&self) -> Option<DateTime> {
        match self {
            SetHeating {
                target_state: HeatingTargetState::Heat { until, .. },
                ..
            } => Some(*until),
            _ => None,
        }
    }
}

//
// SEND NOTIFICATION
//
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PushNotify {
    pub action: NotificationAction,
    pub notification: Notification,
    pub recipient: NotificationRecipient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notification {
    WindowOpened,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl CommandId for NotificationTarget {
    type CommandType = PushNotify;
}

//
// SET ENERGY SAVING
//
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SetEnergySaving {
    pub device: EnergySavingDevice,
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnergySavingDevice {
    LivingRoomTv,
}

impl CommandId for EnergySavingDevice {
    type CommandType = SetEnergySaving;
}

#[cfg(test)]
mod test {
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn set_power() {
        assert_json_eq!(
            Command::SetPower(SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on: true,
            }),
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
    fn set_heating_auto() {
        assert_json_eq!(
            Command::SetHeating(SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Auto,
            }),
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "auto"
            })
        );
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
    fn set_heating_off() {
        assert_json_eq!(
            Command::SetHeating(SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Off
            }),
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
            Command::SetHeating(SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Heat {
                    temperature: DegreeCelsius::from(22.5),
                    until: DateTime::from_iso("2024-10-14T13:37:42Z").unwrap()
                },
            }),
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "heat",
                "temperature": 22.5,
                "until": "2024-10-14T15:37:42+02:00"
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
            Command::PushNotify(PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            }),
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
            Command::PushNotify(PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            }),
            json!({
                "type": "push_notify",
                "action": "notify",
                "recipient": "dennis",
                "notification": "window_opened"
            })
        );
    }
}
