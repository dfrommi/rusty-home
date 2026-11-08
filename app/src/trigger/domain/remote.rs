use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum RemoteTrigger {
    BedroomDoorRemote(DualButtonPress),
    BedframeRemote(DimmerSwitchButtonPress),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Remote[{}]", _variant)]
pub enum RemoteTriggerTarget {
    BedroomDoorRemote,
    BedframeRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DualButtonPress {
    SingleOff,
    SingleOn,
    HoldOff,
    HoldOn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DimmerSwitchButtonPress {
    SingleOn,
    HoldOn,
    SingleOff,
    HoldOff,
    SingleDimUp,
    HoldDimUp,
    SingleDimDown,
    HoldDimDown,
}

impl From<&RemoteTrigger> for RemoteTriggerTarget {
    fn from(val: &RemoteTrigger) -> Self {
        match val {
            RemoteTrigger::BedroomDoorRemote(_) => RemoteTriggerTarget::BedroomDoorRemote,
            RemoteTrigger::BedframeRemote(_) => RemoteTriggerTarget::BedframeRemote,
        }
    }
}
