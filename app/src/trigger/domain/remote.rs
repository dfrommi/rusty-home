use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum RemoteTrigger {
    BedroomDoorRemote(DualButtonPress),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(tag = "command", rename_all = "snake_case")]
#[display("Remote[{}]", _variant)]
pub enum RemoteTriggerTarget {
    BedroomDoorRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DualButtonPress {
    SingleOff,
    SingleOn,
    HoldOff,
    HoldOn,
}

impl From<&RemoteTrigger> for RemoteTriggerTarget {
    fn from(val: &RemoteTrigger) -> Self {
        match val {
            RemoteTrigger::BedroomDoorRemote(_) => RemoteTriggerTarget::BedroomDoorRemote,
        }
    }
}
