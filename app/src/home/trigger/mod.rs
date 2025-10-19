use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::adapter::homekit::{HomekitCommand, HomekitCommandTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTrigger {
    Remote(Remote),
    Homekit(HomekitCommand),
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, Id, EnumVariants,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTriggerTarget {
    Remote(RemoteTarget),
    Homekit(HomekitCommandTarget),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "remote", content = "button", rename_all = "snake_case")]
pub enum Remote {
    BedroomDoor(ButtonPress),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(tag = "remote", rename_all = "snake_case")]
#[display("Remote[{}]", _variant)]
pub enum RemoteTarget {
    BedroomDoor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonPress {
    TopSingle,
    BottomSingle,
}

impl UserTrigger {
    pub fn target(&self) -> UserTriggerTarget {
        match self {
            UserTrigger::Remote(remote) => match remote {
                Remote::BedroomDoor(_) => UserTriggerTarget::Remote(RemoteTarget::BedroomDoor),
            },
            UserTrigger::Homekit(command) => match command {
                HomekitCommand::InfraredHeaterPower(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower)
                }
                HomekitCommand::DehumidifierPower(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower)
                }
                HomekitCommand::LivingRoomTvEnergySaving(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomTvEnergySaving)
                }
                HomekitCommand::LivingRoomCeilingFanSpeed(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomCeilingFanSpeed)
                }
                HomekitCommand::BedroomCeilingFanSpeed(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomCeilingFanSpeed)
                }
                HomekitCommand::LivingRoomHeatingState(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomHeatingState)
                }
                HomekitCommand::BedroomHeatingState(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomHeatingState)
                }
                HomekitCommand::KitchenHeatingState(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::KitchenHeatingState)
                }
                HomekitCommand::RoomOfRequirementsHeatingState(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::RoomOfRequirementsHeatingState)
                }
                HomekitCommand::BathroomHeatingState(_) => {
                    UserTriggerTarget::Homekit(HomekitCommandTarget::BathroomHeatingState)
                }
            },
        }
    }
}

#[cfg(test)]
mod serialization {
    use super::*;

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    #[test]
    fn test_display_remote() {
        assert_eq!(
            UserTriggerTarget::Remote(RemoteTarget::BedroomDoor).to_string(),
            "Remote[BedroomDoor]"
        );
    }

    #[test]
    fn test_display_homekit() {
        assert_eq!(
            UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower).to_string(),
            "Homekit[InfraredHeaterPower]"
        );
    }

    #[test]
    fn test_serialize_remote() {
        assert_json_eq!(
            UserTrigger::Remote(Remote::BedroomDoor(ButtonPress::TopSingle)),
            json!({
                "type": "remote",
                "remote": "bedroom_door",
                "button": "top_single"
            })
        );

        println!(
            "{}",
            serde_json::to_string(&UserTriggerTarget::Remote(RemoteTarget::BedroomDoor)).unwrap()
        );

        assert_json_eq!(
            UserTriggerTarget::Remote(RemoteTarget::BedroomDoor),
            json!({
                "type": "remote",
                "remote": "bedroom_door"
            })
        );
    }

    #[test]
    fn test_serialize_homekit() {
        assert_json_eq!(
            UserTrigger::Homekit(HomekitCommand::InfraredHeaterPower(true)),
            json!({
                "type": "homekit",
                "command": "infrared_heater_power",
                "data": true
            })
        );

        assert_json_eq!(
            UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower),
            json!({
                "type": "homekit",
                "command": "infrared_heater_power"
            })
        );
    }
}
