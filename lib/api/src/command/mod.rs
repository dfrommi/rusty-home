use chrono::{DateTime, Utc};
use r#macro::CommandTarget;
use serde::{Deserialize, Serialize};
use support::unit::DegreeCelsius;

pub mod db;

#[derive(Debug, Clone, CommandTarget, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerToggle {
    Dehumidifier,
    LivingRoomNotificationLight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thermostat {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeatingTargetState {
    Off,
    Heat { temperature: DegreeCelsius },
}

#[derive(Debug)]
pub struct CommandExecution {
    pub id: i64,
    pub command: Command,
    pub state: CommandState,
    pub created: DateTime<Utc>,
    pub source: CommandSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandState {
    Pending,
    InProgress,
    Success,
    Error(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum CommandSource {
    System,
    User,
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
            CommandTarget::SetPower(PowerToggle::LivingRoomNotificationLight),
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
                target_state: HeatingTargetState::Off,
            },
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "off"
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
    fn set_heating_temperature() {
        assert_json_eq!(
            Command::SetHeating {
                device: Thermostat::RoomOfRequirements,
                target_state: HeatingTargetState::Heat {
                    temperature: DegreeCelsius(22.5)
                },
            },
            json!({
                "type": "set_heating",
                "device": "room_of_requirements",
                "mode": "heat",
                "temperature": 22.5
            })
        );
    }
}
