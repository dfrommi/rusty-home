use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandTargetBehavior},
    home_state::{EnergySaving, StateSnapshot},
};
use anyhow::Result;
use derive_more::derive::Display;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum EnergySavingDevice {
    LivingRoomTv,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetEnergySaving {
    pub device: EnergySavingDevice,
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("SetEnergySaving[{}]", device)]
pub struct SetEnergySavingTarget {
    pub device: EnergySavingDevice,
}

impl SetEnergySavingTarget {
    pub fn to_command(&self, on: bool) -> SetEnergySaving {
        SetEnergySaving {
            device: self.device.clone(),
            on,
        }
    }
}

impl From<SetEnergySaving> for SetEnergySavingTarget {
    fn from(command: SetEnergySaving) -> Self {
        Self::from(&command)
    }
}

impl From<&SetEnergySaving> for SetEnergySavingTarget {
    fn from(command: &SetEnergySaving) -> Self {
        Self {
            device: command.device.clone(),
        }
    }
}

impl CommandBehavior for SetEnergySaving {
    type Target = SetEnergySavingTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "SetEnergySaving",
            target: self.device.to_string(),
            state: if self.on { "on" } else { "off" }.to_owned(),
        }
    }

    fn is_reflected_in_state(&self, snapshot: &StateSnapshot) -> Result<Option<bool>> {
        let state_device = match self.device {
            EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
        };
        Ok(Some(snapshot.try_get(state_device)?.value == self.on))
    }
}

impl CommandTargetBehavior for SetEnergySavingTarget {
    type Command = SetEnergySaving;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_state() {
        let target = SetEnergySavingTarget {
            device: EnergySavingDevice::LivingRoomTv,
        };

        let command = target.to_command(false);

        assert_eq!(command.device, EnergySavingDevice::LivingRoomTv);
        assert!(!command.on);
    }

    #[test]
    fn target_enables_energy_saving_by_default() {
        let target = SetEnergySavingTarget {
            device: EnergySavingDevice::LivingRoomTv,
        };

        let command = target.to_default_command();

        assert!(command.on);
    }

    #[test]
    fn command_exposes_its_target_and_display_parts() {
        let command = SetEnergySaving {
            device: EnergySavingDevice::LivingRoomTv,
            on: true,
        };

        assert_eq!(
            command.target(),
            SetEnergySavingTarget {
                device: EnergySavingDevice::LivingRoomTv
            }
        );
        assert_eq!(command.display_parts().command_type, "SetEnergySaving");
        assert_eq!(command.display_parts().target, "LivingRoomTv");
        assert_eq!(command.display_parts().state, "on");
        assert!(!command.deduplicate_when_unobservable());
    }
}
