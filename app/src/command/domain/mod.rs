mod command_state;

use crate::core::domain::Radiator;
use crate::core::range::Range;
use crate::core::unit::{DegreeCelsius, FanAirflow, Percent};
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
        device: Radiator,
        target_state: HeatingTargetState,
    },

    Notify {
        notification: NotificationKind,
        target: NotificationDestination,
        operation: NotificationOperation,
    },
    SetEnergySaving {
        device: EnergySavingDevice,
        on: bool,
    },
    ControlFan {
        device: Fan,
        speed: FanAirflow,
    },
    OpenDoor {
        device: Lock,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandTarget {
    #[display("SetPower[{}]", device)]
    SetPower { device: PowerToggle },

    #[display("SetHeating[{}]", device)]
    SetHeating { device: Radiator },

    #[display("NotifyPhone[{} @ {}]", notification, recipient)]
    NotifyPhone {
        notification: NotificationKind,
        recipient: NotificationRecipient,
    },

    #[display("NotifyLight[{}]", device)]
    NotifyLight { device: NotificationLight },

    #[display("SetEnergySaving[{}]", device)]
    SetEnergySaving { device: EnergySavingDevice },

    #[display("ControlFan[{}]", device)]
    ControlFan { device: Fan },

    #[display("OpenDoor[{}]", device)]
    OpenDoor { device: Lock },
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
            Command::SetHeating { device, .. } => CommandTarget::SetHeating { device: *device },
            Command::Notify {
                notification,
                target: NotificationDestination::Phone { recipient },
                ..
            } => CommandTarget::NotifyPhone {
                recipient: *recipient,
                notification: *notification,
            },
            Command::Notify {
                target: NotificationDestination::Light { device },
                ..
            } => CommandTarget::NotifyLight { device: *device },
            Command::SetEnergySaving { device, .. } => CommandTarget::SetEnergySaving { device: device.clone() },
            Command::ControlFan { device, .. } => CommandTarget::ControlFan { device: device.clone() },
            Command::OpenDoor { device } => CommandTarget::OpenDoor { device: device.clone() },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandDisplayParts {
    pub command_type: &'static str,
    pub target: String,
    pub state: String,
}

impl Command {
    pub fn display_parts(&self) -> CommandDisplayParts {
        match self {
            Command::SetPower { device, power_on } => CommandDisplayParts {
                command_type: "SetPower",
                target: device.to_string(),
                state: if *power_on { "on".to_string() } else { "off".to_string() },
            },
            Command::SetHeating { device, target_state } => CommandDisplayParts {
                command_type: "SetHeating",
                target: device.to_string(),
                state: target_state.to_string(),
            },
            Command::Notify {
                notification,
                target,
                operation,
            } => CommandDisplayParts {
                command_type: "Notify",
                target: format!("{notification} @ {target}"),
                state: operation.to_string(),
            },
            Command::SetEnergySaving { device, on } => CommandDisplayParts {
                command_type: "SetEnergySaving",
                target: device.to_string(),
                state: if *on { "on" } else { "off" }.to_string(),
            },
            Command::ControlFan { device, speed } => CommandDisplayParts {
                command_type: "ControlFan",
                target: device.to_string(),
                state: speed.to_string(),
            },
            Command::OpenDoor { device } => CommandDisplayParts {
                command_type: "OpenDoor",
                target: device.to_string(),
                state: "open".to_string(),
            },
        }
    }

    pub fn deduplicate_when_unobservable(&self) -> bool {
        matches!(
            self,
            Command::Notify {
                target: NotificationDestination::Phone { .. },
                ..
            }
        )
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
    LivingRoomTvAmbilight,
}

//
// SET HEATING
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeatingTargetState {
    Off,
    Heat {
        target_temperature: Range<DegreeCelsius>,
        demand_limit: Range<Percent>,
    },
}

impl std::fmt::Display for HeatingTargetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeatingTargetState::Off => write!(f, "Off"),
            HeatingTargetState::Heat {
                target_temperature,
                demand_limit,
            } => write!(f, "Heat {} ({})", target_temperature, demand_limit),
        }
    }
}

//
// NOTIFICATIONS
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    WindowOpened,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRecipient {
    Dennis,
    Sabine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLight {
    LivingRoom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationDestination {
    #[display("Phone({recipient})")]
    Phone { recipient: NotificationRecipient },
    #[display("Light({device})")]
    Light { device: NotificationLight },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[serde(rename_all = "snake_case")]
pub enum NotificationOperation {
    Show,
    Dismiss,
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
    BedroomDehumidifier,
    LivingRoomAirPurifier,
}

//
// OPEN DOOR
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Lock {
    BuildingEntrance,
}

#[cfg(test)]
mod test {
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn notify_serializes_its_intent_and_destination() {
        assert_json_eq!(
            Command::Notify {
                notification: NotificationKind::WindowOpened,
                target: NotificationDestination::Light {
                    device: NotificationLight::LivingRoom,
                },
                operation: NotificationOperation::Show,
            },
            json!({
                "type": "notify",
                "notification": "window_opened",
                "target": {
                    "type": "light",
                    "device": "living_room"
                },
                "operation": "show"
            })
        );
    }

    #[test]
    fn notification_command_targets_have_clear_ids_and_display() {
        let phone_command = Command::Notify {
            notification: NotificationKind::WindowOpened,
            target: NotificationDestination::Phone {
                recipient: NotificationRecipient::Dennis,
            },
            operation: NotificationOperation::Show,
        };
        let phone_target = CommandTarget::from(&phone_command);

        assert_eq!(
            phone_target.ext_id().to_string(),
            "command_target::notify_phone::window_opened::dennis"
        );
        assert_eq!(phone_target.to_string(), "NotifyPhone[WindowOpened @ Dennis]");
        assert_eq!(phone_command.display_parts().target, "WindowOpened @ Phone(Dennis)");
        assert_eq!(phone_command.display_parts().state, "Show");

        let light_command = Command::Notify {
            notification: NotificationKind::WindowOpened,
            target: NotificationDestination::Light {
                device: NotificationLight::LivingRoom,
            },
            operation: NotificationOperation::Show,
        };
        let light_target = CommandTarget::from(&light_command);

        assert_eq!(light_target.ext_id().to_string(), "command_target::notify_light::living_room");
        assert_eq!(light_target.to_string(), "NotifyLight[LivingRoom]");
    }
}
