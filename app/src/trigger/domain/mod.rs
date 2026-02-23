mod homekit;

pub use homekit::*;

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
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::From, derive_more::Display, Id, EnumVariants,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserTriggerTarget {
    Homekit(HomekitCommandTarget),
}

impl UserTrigger {
    pub fn target(&self) -> UserTriggerTarget {
        match self {
            UserTrigger::Homekit(command) => UserTriggerTarget::Homekit(command.into()),
        }
    }
}
