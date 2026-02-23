use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::core::unit::{DegreeCelsius, FanAirflow};

//Don't forget to add to action planning config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum HomekitCommand {
    InfraredHeaterPower(bool),
    DehumidifierPower(bool),
    LivingRoomTvEnergySaving(bool),
    LivingRoomCeilingFanSpeed(FanAirflow),
    BedroomCeilingFanSpeed(FanAirflow),
    BedroomDehumidifierFanSpeed(FanAirflow),
    LivingRoomHeatingState(HomekitHeatingState),
    BedroomHeatingState(HomekitHeatingState),
    KitchenHeatingState(HomekitHeatingState),
    RoomOfRequirementsHeatingState(HomekitHeatingState),
    BathroomHeatingState(HomekitHeatingState),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Homekit[{}]", _variant)]
pub enum HomekitCommandTarget {
    InfraredHeaterPower,
    DehumidifierPower,
    LivingRoomTvEnergySaving,
    LivingRoomCeilingFanSpeed,
    BedroomCeilingFanSpeed,
    BedroomDehumidifierFanSpeed,
    LivingRoomHeatingState,
    BedroomHeatingState,
    KitchenHeatingState,
    RoomOfRequirementsHeatingState,
    BathroomHeatingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HomekitHeatingState {
    Off,
    Heat(DegreeCelsius),
    Auto,
}

impl From<&HomekitCommand> for HomekitCommandTarget {
    fn from(command: &HomekitCommand) -> Self {
        match command {
            HomekitCommand::InfraredHeaterPower(_) => HomekitCommandTarget::InfraredHeaterPower,
            HomekitCommand::DehumidifierPower(_) => HomekitCommandTarget::DehumidifierPower,
            HomekitCommand::LivingRoomTvEnergySaving(_) => HomekitCommandTarget::LivingRoomTvEnergySaving,
            HomekitCommand::LivingRoomCeilingFanSpeed(_) => HomekitCommandTarget::LivingRoomCeilingFanSpeed,
            HomekitCommand::BedroomCeilingFanSpeed(_) => HomekitCommandTarget::BedroomCeilingFanSpeed,
            HomekitCommand::BedroomDehumidifierFanSpeed(_) => HomekitCommandTarget::BedroomDehumidifierFanSpeed,
            HomekitCommand::LivingRoomHeatingState(_) => HomekitCommandTarget::LivingRoomHeatingState,
            HomekitCommand::BedroomHeatingState(_) => HomekitCommandTarget::BedroomHeatingState,
            HomekitCommand::KitchenHeatingState(_) => HomekitCommandTarget::KitchenHeatingState,
            HomekitCommand::RoomOfRequirementsHeatingState(_) => HomekitCommandTarget::RoomOfRequirementsHeatingState,
            HomekitCommand::BathroomHeatingState(_) => HomekitCommandTarget::BathroomHeatingState,
        }
    }
}

#[cfg(test)]
mod serialization {
    use crate::trigger::{UserTrigger, UserTriggerTarget};

    use super::*;

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    #[test]
    fn test_display_homekit() {
        assert_eq!(
            UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower).to_string(),
            "Homekit[InfraredHeaterPower]"
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
