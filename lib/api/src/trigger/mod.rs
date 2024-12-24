use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTrigger {
    Remote(Remote),
    Homekit(Homekit),
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::From, derive_more::Display)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTriggerTarget {
    Remote(RemoteTarget),
    Homekit(HomekitTarget),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "remote", content = "button", rename_all = "snake_case")]
pub enum Remote {
    BedroomDoor(ButtonPress),
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum Homekit {
    InfraredHeaterPower(bool),
    DehumidifierPower(bool),
    LivingRoomTvEnergySaving(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Homekit[{}]", _variant)]
pub enum HomekitTarget {
    InfraredHeaterPower,
    DehumidifierPower,
    LivingRoomTvEnergySaving,
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
            UserTriggerTarget::Homekit(HomekitTarget::InfraredHeaterPower).to_string(),
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
            UserTrigger::Homekit(Homekit::InfraredHeaterPower(true)),
            json!({
                "type": "homekit",
                "command": "infrared_heater_power",
                "data": true
            })
        );

        assert_json_eq!(
            UserTriggerTarget::Homekit(HomekitTarget::InfraredHeaterPower),
            json!({
                "type": "homekit",
                "command": "infrared_heater_power"
            })
        );
    }
}
