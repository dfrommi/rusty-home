use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    home_state::{PowerAvailable, StateSnapshot},
};
use anyhow::Result;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum PowerToggle {
    BedroomAirPurifier,
    Dehumidifier,
    InfraredHeater,
    LivingRoomTvAmbilight,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetPower {
    pub device: PowerToggle,
    pub power_on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("SetPower[{}]", device)]
pub struct SetPowerTarget {
    pub device: PowerToggle,
}

impl SetPowerTarget {
    pub fn to_command(&self, power_on: bool) -> SetPower {
        SetPower {
            device: self.device.clone(),
            power_on,
        }
    }
}

impl From<SetPower> for SetPowerTarget {
    fn from(command: SetPower) -> Self {
        Self::from(&command)
    }
}

impl From<&SetPower> for SetPowerTarget {
    fn from(command: &SetPower) -> Self {
        Self {
            device: command.device.clone(),
        }
    }
}

impl CommandBehavior for SetPower {
    type Target = SetPowerTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "SetPower",
            target: self.device.to_string(),
            state: if self.power_on { "on" } else { "off" }.to_owned(),
        }
    }

    fn command_state(&self, snapshot: &StateSnapshot) -> Result<CommandState> {
        let powered_item = match self.device {
            PowerToggle::BedroomAirPurifier => PowerAvailable::BedroomAirPurifier,
            PowerToggle::Dehumidifier => PowerAvailable::Dehumidifier,
            PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
            PowerToggle::LivingRoomTvAmbilight => PowerAvailable::LivingRoomTvAmbilight,
        };

        let powered = snapshot.try_get(powered_item)?.value;
        Ok(if powered == self.power_on {
            CommandState::Reflected
        } else {
            CommandState::NotReflected
        })
    }
}

impl CommandTargetBehavior for SetPowerTarget {
    type Command = SetPower;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_power() {
        let target = SetPowerTarget {
            device: PowerToggle::Dehumidifier,
        };

        let command = target.to_command(true);

        assert_eq!(command.device, PowerToggle::Dehumidifier);
        assert!(command.power_on);
    }

    #[test]
    fn target_creates_an_off_command_by_default() {
        let target = SetPowerTarget {
            device: PowerToggle::InfraredHeater,
        };

        let command = target.to_default_command();

        assert_eq!(command.device, PowerToggle::InfraredHeater);
        assert!(!command.power_on);
    }

    #[test]
    fn command_exposes_its_target_and_display_parts() {
        let command = SetPower {
            device: PowerToggle::Dehumidifier,
            power_on: true,
        };

        assert_eq!(
            command.target(),
            SetPowerTarget {
                device: PowerToggle::Dehumidifier
            }
        );
        assert_eq!(command.display_parts().command_type, "SetPower");
        assert_eq!(command.display_parts().target, "Dehumidifier");
        assert_eq!(command.display_parts().state, "on");
    }
}
