mod remote;

pub use remote::*;

use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

use crate::core::domain::HeatingZone;
use crate::core::unit::{DegreeCelsius, FanAirflow};
use crate::home_state::FanActivity;

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, sqlx::Type,
)]
#[sqlx(transparent)]
pub struct UserTriggerId(i64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id)]
#[serde(rename_all = "snake_case")]
pub enum OnOffDevice {
    Dehumidifier,
    InfraredHeater,
    LivingRoomTvEnergySaving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTrigger {
    DevicePower { device: OnOffDevice, on: bool },
    FanSpeed { fan: FanActivity, airflow: FanAirflow },
    Heating { zone: HeatingZone, request: HeatingRequest },
    OpenDoor { door: Door },
    Remote(RemoteTrigger),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, Id)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTriggerTarget {
    #[display("DevicePower[{}]", _0)]
    DevicePower(OnOffDevice),
    #[display("FanSpeed[{}]", _0)]
    FanSpeed(FanActivity),
    #[display("Heating[{}]", _0)]
    Heating(HeatingZone),
    #[display("OpenDoor[{}]", _0)]
    OpenDoor(Door),
    Remote(RemoteTriggerTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    Building,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeatingRequest {
    Off,
    Heat(DegreeCelsius),
    Auto,
}

impl UserTrigger {
    pub fn target(&self) -> UserTriggerTarget {
        match self {
            UserTrigger::DevicePower { device, .. } => UserTriggerTarget::DevicePower(device.clone()),
            UserTrigger::FanSpeed { fan, .. } => UserTriggerTarget::FanSpeed(*fan),
            UserTrigger::Heating { zone, .. } => UserTriggerTarget::Heating(*zone),
            UserTrigger::OpenDoor { door } => UserTriggerTarget::OpenDoor(door.clone()),
            UserTrigger::Remote(command) => UserTriggerTarget::Remote(command.into()),
        }
    }
}

#[cfg(test)]
mod serialization {
    use super::*;

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    #[test]
    fn test_serialize_device_power() {
        assert_json_eq!(
            UserTrigger::DevicePower {
                device: OnOffDevice::InfraredHeater,
                on: true
            },
            json!({
                "type": "device_power",
                "device": "infrared_heater",
                "on": true
            })
        );
    }

    #[test]
    fn test_display_device_power() {
        assert_eq!(
            UserTriggerTarget::DevicePower(OnOffDevice::InfraredHeater).to_string(),
            "DevicePower[InfraredHeater]"
        );
    }
}
