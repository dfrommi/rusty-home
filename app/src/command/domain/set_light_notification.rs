use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandTargetBehavior},
    home_state::{PowerAvailable, StateSnapshot},
};
use anyhow::Result;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLight {
    LivingRoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, derive_more::Display, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum LightSignal {
    WindowOpened,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetLightNotification {
    pub device: NotificationLight,
    pub signal: LightSignal,
    pub on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("SetLightNotification[{}]", device)]
pub struct SetLightNotificationTarget {
    pub device: NotificationLight,
}

impl SetLightNotificationTarget {
    pub fn to_command(self, signal: LightSignal, on: bool) -> SetLightNotification {
        SetLightNotification {
            device: self.device,
            signal,
            on,
        }
    }
}

impl From<SetLightNotification> for SetLightNotificationTarget {
    fn from(command: SetLightNotification) -> Self {
        Self::from(&command)
    }
}

impl From<&SetLightNotification> for SetLightNotificationTarget {
    fn from(command: &SetLightNotification) -> Self {
        Self { device: command.device }
    }
}

impl CommandBehavior for SetLightNotification {
    type Target = SetLightNotificationTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "SetLightNotification",
            target: self.device.to_string(),
            state: if self.on { "on" } else { "off" }.to_owned(),
        }
    }

    fn is_reflected_in_state(&self, snapshot: &StateSnapshot) -> Result<Option<bool>> {
        let state_item = match self.device {
            NotificationLight::LivingRoom => PowerAvailable::LivingRoomNotificationLight,
        };
        let powered = snapshot.try_get(state_item)?.value;
        Ok(Some(powered == self.on))
    }
}

impl CommandTargetBehavior for SetLightNotificationTarget {
    type Command = SetLightNotification;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(LightSignal::WindowOpened, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_signal_and_state() {
        let target = SetLightNotificationTarget {
            device: NotificationLight::LivingRoom,
        };

        let command = target.to_command(LightSignal::WindowOpened, true);

        assert_eq!(command.signal, LightSignal::WindowOpened);
        assert_eq!(command.device, NotificationLight::LivingRoom);
        assert!(command.on);
    }

    #[test]
    fn target_creates_a_cleared_command_by_default() {
        let target = SetLightNotificationTarget {
            device: NotificationLight::LivingRoom,
        };

        let command = target.to_default_command();

        assert_eq!(command.signal, LightSignal::WindowOpened);
        assert!(!command.on);
    }

    #[test]
    fn light_notification_reflects_its_power_state() {
        let command = SetLightNotification {
            device: NotificationLight::LivingRoom,
            signal: LightSignal::WindowOpened,
            on: true,
        };

        assert_eq!(command.display_parts().command_type, "SetLightNotification");
        assert_eq!(command.display_parts().target, "LivingRoom");
        assert_eq!(command.display_parts().state, "on");
        assert!(!command.deduplicate_when_unobservable());
    }
}
