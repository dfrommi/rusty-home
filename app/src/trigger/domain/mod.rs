mod homekit;
mod remote;

pub use homekit::*;
pub use remote::*;

use r#macro::{EnumVariants, Id};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, sqlx::Type,
)]
#[sqlx(transparent)]
pub struct UserTriggerId(i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTrigger {
    Homekit(HomekitCommand),
    Remote(RemoteTrigger),
    LockDoorOpen(Door),
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, Id, EnumVariants,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTriggerTarget {
    Homekit(HomekitCommandTarget),
    Remote(RemoteTriggerTarget),
    LockDoorOpen(Door),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    Building,
}

impl UserTrigger {
    pub fn target(&self) -> UserTriggerTarget {
        match self {
            UserTrigger::Homekit(command) => UserTriggerTarget::Homekit(command.into()),
            UserTrigger::Remote(command) => UserTriggerTarget::Remote(command.into()),
            UserTrigger::LockDoorOpen(door) => UserTriggerTarget::LockDoorOpen(door.clone()),
        }
    }
}
