use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    core::unit::FanAirflow,
    home_state::{FanActivity, StateSnapshot},
};
use anyhow::Result;
use derive_more::derive::Display;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum Fan {
    BedroomDehumidifier,
    LivingRoomAirPurifier,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ControlFan {
    pub device: Fan,
    pub speed: FanAirflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("ControlFan[{}]", device)]
pub struct ControlFanTarget {
    pub device: Fan,
}

impl ControlFanTarget {
    pub fn to_command(&self, speed: FanAirflow) -> ControlFan {
        ControlFan {
            device: self.device.clone(),
            speed,
        }
    }
}

impl From<ControlFan> for ControlFanTarget {
    fn from(command: ControlFan) -> Self {
        Self::from(&command)
    }
}

impl From<&ControlFan> for ControlFanTarget {
    fn from(command: &ControlFan) -> Self {
        Self {
            device: command.device.clone(),
        }
    }
}

impl CommandBehavior for ControlFan {
    type Target = ControlFanTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "ControlFan",
            target: self.device.to_string(),
            state: self.speed.to_string(),
        }
    }

    fn command_state(&self, snapshot: &StateSnapshot) -> Result<CommandState> {
        let state_device = match self.device {
            Fan::BedroomDehumidifier => FanActivity::BedroomDehumidifier,
            Fan::LivingRoomAirPurifier => FanActivity::LivingRoomAirPurifier,
        };

        let current_flow = snapshot.try_get(state_device)?.value;
        Ok(if current_flow == self.speed {
            CommandState::Reflected
        } else {
            CommandState::NotReflected
        })
    }
}

impl CommandTargetBehavior for ControlFanTarget {
    type Command = ControlFan;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(FanAirflow::Off)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_speed() {
        let target = ControlFanTarget {
            device: Fan::BedroomDehumidifier,
        };

        let command = target.to_command(FanAirflow::Off);

        assert_eq!(command.device, Fan::BedroomDehumidifier);
        assert_eq!(command.speed, FanAirflow::Off);
    }

    #[test]
    fn target_turns_the_fan_off_by_default() {
        let target = ControlFanTarget {
            device: Fan::LivingRoomAirPurifier,
        };

        let command = target.to_default_command();

        assert_eq!(command.speed, FanAirflow::Off);
    }

    #[test]
    fn command_exposes_its_target_and_display_parts() {
        let command = ControlFan {
            device: Fan::BedroomDehumidifier,
            speed: FanAirflow::Off,
        };

        assert_eq!(
            command.target(),
            ControlFanTarget {
                device: Fan::BedroomDehumidifier
            }
        );
        assert_eq!(command.display_parts().command_type, "ControlFan");
        assert_eq!(command.display_parts().target, "BedroomDehumidifier");
        assert_eq!(command.display_parts().state, "off");
    }
}
