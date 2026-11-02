use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    core::{
        domain::Radiator,
        range::Range,
        unit::{DegreeCelsius, Percent},
    },
    home_state::{HeatingDemandLimit, SetPoint, StateSnapshot},
};
use anyhow::Result;
use r#macro::Id;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HeatingTargetState {
    Off,
    Heat {
        target_temperature: Range<DegreeCelsius>,
        demand: Percent,
    },
}

impl std::fmt::Display for HeatingTargetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeatingTargetState::Off => write!(f, "Off"),
            HeatingTargetState::Heat {
                target_temperature,
                demand,
            } => write!(f, "Heat {} ({})", target_temperature, demand),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetHeating {
    pub device: Radiator,
    pub target_state: HeatingTargetState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("SetHeating[{}]", device)]
pub struct SetHeatingTarget {
    pub device: Radiator,
}

impl SetHeatingTarget {
    pub fn to_command(self, target_state: HeatingTargetState) -> SetHeating {
        SetHeating {
            device: self.device,
            target_state,
        }
    }
}

impl From<SetHeating> for SetHeatingTarget {
    fn from(command: SetHeating) -> Self {
        Self::from(&command)
    }
}

impl From<&SetHeating> for SetHeatingTarget {
    fn from(command: &SetHeating) -> Self {
        Self { device: command.device }
    }
}

impl CommandBehavior for SetHeating {
    type Target = SetHeatingTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "SetHeating",
            target: self.device.to_string(),
            state: self.target_state.to_string(),
        }
    }

    fn command_state(&self, snapshot: &StateSnapshot) -> Result<CommandState> {
        let (target_temperature, demand) = match &self.target_state {
            HeatingTargetState::Off => (Range::new(DegreeCelsius(0.0), DegreeCelsius(0.0)), Percent(0.0)),
            HeatingTargetState::Heat {
                target_temperature,
                demand,
            } => (target_temperature.clone(), *demand),
        };

        let current_setpoint_range = snapshot.try_get(SetPoint::Current(self.device))?.value;
        let current_demand = snapshot.try_get(HeatingDemandLimit::Current(self.device))?.value;
        let is_reflected = current_setpoint_range == target_temperature && current_demand == demand;

        tracing::debug!(
            "Checking if SetHeating command for device {:?} is reflected in state: current setpoint range: {:?}, target setpoint range: {:?}, current on-position: {:?}, target on-position: {:?}, is_reflected: {}",
            self.device,
            current_setpoint_range,
            target_temperature,
            current_demand,
            demand,
            is_reflected
        );

        Ok(if is_reflected {
            CommandState::Reflected
        } else {
            CommandState::NotReflected
        })
    }
}

impl CommandTargetBehavior for SetHeatingTarget {
    type Command = SetHeating;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(HeatingTargetState::Off)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_state() {
        let target = SetHeatingTarget {
            device: Radiator::Bedroom,
        };

        let command = target.to_command(HeatingTargetState::Off);

        assert_eq!(command.device, Radiator::Bedroom);
        assert!(matches!(command.target_state, HeatingTargetState::Off));
    }

    #[test]
    fn target_creates_an_off_command_by_default() {
        let target = SetHeatingTarget {
            device: Radiator::Bathroom,
        };

        let command = target.to_default_command();

        assert_eq!(command.device, Radiator::Bathroom);
        assert!(matches!(command.target_state, HeatingTargetState::Off));
    }

    #[test]
    fn command_exposes_its_target_and_display_parts() {
        let command = SetHeating {
            device: Radiator::Kitchen,
            target_state: HeatingTargetState::Off,
        };

        assert_eq!(
            command.target(),
            SetHeatingTarget {
                device: Radiator::Kitchen
            }
        );
        assert_eq!(command.display_parts().command_type, "SetHeating");
        assert_eq!(command.display_parts().target, "Kitchen");
        assert_eq!(command.display_parts().state, "Off");
    }
}
