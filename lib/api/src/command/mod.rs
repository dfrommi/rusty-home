use derive_more::derive::{Display, From};
use serde::{Deserialize, Serialize};
use support::{
    time::{DateTime, Duration},
    unit::DegreeCelsius,
};

pub mod db;

#[derive(Debug, Clone, PartialEq, From, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetPower(SetPower),
    SetHeating(SetHeating),
    PushNotify(PushNotify),
    SetEnergySaving(SetEnergySaving),
}

#[derive(Debug, Clone, PartialEq, Eq, From, Serialize, Deserialize, derive_more::Display)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandTarget {
    #[display("SetPower[{}]", device)]
    SetPower { device: PowerToggle },

    #[display("SetHeating[{}]", device)]
    SetHeating { device: Thermostat },

    #[display("PushNotify[{} - {}]", notification, recipient)]
    PushNotify {
        recipient: NotificationRecipient,
        notification: Notification,
    },

    #[display("SetEnergySaving[{}]", device)]
    SetEnergySaving { device: EnergySavingDevice },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
#[serde(rename_all = "snake_case")]
pub enum PowerToggle {
    Dehumidifier,
    InfraredHeater,
    LivingRoomNotificationLight,
}

impl From<&SetPower> for CommandTarget {
    fn from(val: &SetPower) -> Self {
        CommandTarget::SetPower {
            device: val.device.clone(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
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
        duration: Duration,
    },
}

impl From<&SetHeating> for CommandTarget {
    fn from(val: &SetHeating) -> Self {
        CommandTarget::SetHeating {
            device: val.device.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
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

impl From<&PushNotify> for CommandTarget {
    fn from(val: &PushNotify) -> Self {
        CommandTarget::PushNotify {
            recipient: val.recipient.clone(),
            notification: val.notification.clone(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum EnergySavingDevice {
    LivingRoomTv,
}

impl From<&SetEnergySaving> for CommandTarget {
    fn from(val: &SetEnergySaving) -> Self {
        CommandTarget::SetEnergySaving {
            device: val.device.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use assert_json_diff::assert_json_eq;
    use serde_json::json;
    use support::t;

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
                    duration: t!(2 hours),
                },
            }),
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "heat",
                "temperature": 22.5,
                "duration": "PT2H"
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
