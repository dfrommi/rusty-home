use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandTargetBehavior},
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
pub struct OpenDoor {
    pub device: Lock,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("OpenDoor[{}]", device)]
pub struct OpenDoorTarget {
    pub device: Lock,
}

impl OpenDoorTarget {
    pub fn to_command(&self, open: bool) -> OpenDoor {
        OpenDoor {
            device: self.device.clone(),
            open,
        }
    }
}

impl From<OpenDoor> for OpenDoorTarget {
    fn from(command: OpenDoor) -> Self {
        Self::from(&command)
    }
}

impl From<&OpenDoor> for OpenDoorTarget {
    fn from(command: &OpenDoor) -> Self {
        Self {
            device: command.device.clone(),
        }
    }
}

impl CommandBehavior for OpenDoor {
    type Target = OpenDoorTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "OpenDoor",
            target: self.device.to_string(),
            state: if self.open { "open" } else { "close" }.to_owned(),
        }
    }

    fn is_reflected_in_state(&self, _snapshot: &StateSnapshot) -> Result<Option<bool>> {
        Ok(None)
    }
}

impl CommandTargetBehavior for OpenDoorTarget {
    type Command = OpenDoor;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_door_state() {
        let target = OpenDoorTarget {
            device: Lock::BuildingEntrance,
        };

        let command = target.to_command(true);

        assert_eq!(command.device, Lock::BuildingEntrance);
        assert!(command.open);
    }

    #[test]
    fn target_creates_a_closed_command_by_default() {
        let target = OpenDoorTarget {
            device: Lock::BuildingEntrance,
        };

        let command = target.to_default_command();

        assert!(!command.open);
    }

    #[test]
    fn door_commands_have_no_persistent_state_reflection() {
        let command = OpenDoor {
            device: Lock::BuildingEntrance,
            open: true,
        };

        assert_eq!(command.display_parts().command_type, "OpenDoor");
        assert_eq!(command.display_parts().target, "BuildingEntrance");
        assert_eq!(command.display_parts().state, "open");
        assert!(matches!(command.is_reflected_in_state(&StateSnapshot::default()), Ok(None)));
    }
}
