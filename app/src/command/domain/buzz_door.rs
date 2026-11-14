use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    home_state::StateSnapshot,
};
use anyhow::Result;
use derive_more::derive::Display;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Lock {
    BuildingEntrance,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuzzDoor {
    pub device: Lock,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, Id)]
#[display("BuzzDoor[{}]", device)]
pub struct BuzzDoorTarget {
    pub device: Lock,
}

impl From<BuzzDoor> for BuzzDoorTarget {
    fn from(command: BuzzDoor) -> Self {
        Self::from(&command)
    }
}

impl From<&BuzzDoor> for BuzzDoorTarget {
    fn from(command: &BuzzDoor) -> Self {
        Self {
            device: command.device.clone(),
        }
    }
}

impl CommandBehavior for BuzzDoor {
    type Target = BuzzDoorTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "BuzzDoor",
            target: self.device.to_string(),
            state: "buzz".to_owned(),
        }
    }

    fn command_state(&self, _snapshot: &StateSnapshot) -> Result<CommandState> {
        Ok(CommandState::UnobservableRepeatable)
    }
}

impl CommandTargetBehavior for BuzzDoorTarget {
    type Command = BuzzDoor;

    fn to_default_command(&self) -> Option<Self::Command> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buzzer_target_has_no_default_command() {
        let target = BuzzDoorTarget {
            device: Lock::BuildingEntrance,
        };

        assert!(target.to_default_command().is_none());
        assert_eq!(target.to_string(), "BuzzDoor[BuildingEntrance]");
    }

    #[test]
    fn buzzing_is_an_unobservable_repeatable_action() {
        let command = BuzzDoor {
            device: Lock::BuildingEntrance,
        };

        assert_eq!(command.display_parts().command_type, "BuzzDoor");
        assert_eq!(command.display_parts().target, "BuildingEntrance");
        assert_eq!(command.display_parts().state, "buzz");
        assert!(matches!(
            command.command_state(&StateSnapshot::default()),
            Ok(CommandState::UnobservableRepeatable)
        ));
    }
}
