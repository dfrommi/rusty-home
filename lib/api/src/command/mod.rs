use derive_more::derive::From;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use support::{time::DateTime, unit::DegreeCelsius};

pub mod db;

#[derive(Debug, Clone, PartialEq, From, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetPower(SetPower),
    SetHeating(SetHeating),
}

impl CommandId for Command {
    type CommandType = Command;
}

impl From<Command> for CommandTarget {
    fn from(val: Command) -> Self {
        match val {
            Command::SetPower(SetPower { device, .. }) => device.clone().into(),
            Command::SetHeating(SetHeating { device, .. }) => device.clone().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, From, Serialize, Deserialize)]
#[serde(tag = "type", content = "device", rename_all = "snake_case")]
pub enum CommandTarget {
    SetPower(PowerToggle),
    SetHeating(Thermostat),
}

impl CommandId for CommandTarget {
    type CommandType = Command;
}

impl From<&Command> for CommandTarget {
    fn from(val: &Command) -> Self {
        match val {
            Command::SetPower(SetPower { device, .. }) => device.clone().into(),
            Command::SetHeating(SetHeating { device, .. }) => device.clone().into(),
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

pub trait CommandId: Into<CommandTarget> {
    type CommandType: Into<Command> + DeserializeOwned;
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
    InfaredHeater,
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
            CommandTarget::SetPower(PowerToggle::LivingRoomNotificationLight),
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
            CommandTarget::SetHeating(Thermostat::RoomOfRequirements),
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
}
