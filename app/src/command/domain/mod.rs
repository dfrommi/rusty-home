mod control_fan;
mod open_door;
mod push_notify;
mod set_energy_saving;
mod set_heating;
mod set_light_notification;
mod set_power;

pub use control_fan::{ControlFan, ControlFanTarget, Fan};
pub use open_door::{Lock, OpenDoor, OpenDoorTarget};
pub use push_notify::{PushNotification, PushNotify, PushNotifyOperation, PushNotifyTarget, PushRecipient};
pub use set_energy_saving::{EnergySavingDevice, SetEnergySaving, SetEnergySavingTarget};
pub use set_heating::{HeatingTargetState, SetHeating, SetHeatingTarget};
pub use set_light_notification::{LightSignal, NotificationLight, SetLightNotification, SetLightNotificationTarget};
pub use set_power::{PowerToggle, SetPower, SetPowerTarget};

use derive_more::derive::From;
use serde::Serialize;

pub trait CommandBehavior: Clone + PartialEq + Sized {
    type Target: CommandTargetBehavior<Command = Self>;

    fn target(&self) -> Self::Target;
    fn display_parts(&self) -> CommandDisplayParts;
    fn deduplicate_when_unobservable(&self) -> bool {
        false
    }
    fn is_reflected_in_state(&self, snapshot: &crate::home_state::StateSnapshot) -> anyhow::Result<Option<bool>>;
}

pub trait CommandTargetBehavior: Clone + Eq + std::hash::Hash + Sized {
    type Command: CommandBehavior<Target = Self>;

    fn to_default_command(&self) -> Self::Command;
}

#[derive(Debug, Clone, PartialEq, From, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    SetPower(SetPower),
    SetHeating(SetHeating),
    PushNotify(PushNotify),
    SetLightNotification(SetLightNotification),
    SetEnergySaving(SetEnergySaving),
    ControlFan(ControlFan),
    OpenDoor(OpenDoor),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, r#macro::IdDelegation)]
pub enum CommandTarget {
    SetPower(SetPowerTarget),
    SetHeating(SetHeatingTarget),
    PushNotify(PushNotifyTarget),
    SetLightNotification(SetLightNotificationTarget),
    SetEnergySaving(SetEnergySavingTarget),
    ControlFan(ControlFanTarget),
    OpenDoor(OpenDoorTarget),
}

impl CommandBehavior for Command {
    type Target = CommandTarget;

    fn target(&self) -> Self::Target {
        match self {
            Command::SetPower(command) => CommandTarget::SetPower(command.target()),
            Command::SetHeating(command) => CommandTarget::SetHeating(command.target()),
            Command::PushNotify(command) => CommandTarget::PushNotify(command.target()),
            Command::SetLightNotification(command) => CommandTarget::SetLightNotification(command.target()),
            Command::SetEnergySaving(command) => CommandTarget::SetEnergySaving(command.target()),
            Command::ControlFan(command) => CommandTarget::ControlFan(command.target()),
            Command::OpenDoor(command) => CommandTarget::OpenDoor(command.target()),
        }
    }

    fn display_parts(&self) -> CommandDisplayParts {
        match self {
            Command::SetPower(command) => command.display_parts(),
            Command::SetHeating(command) => command.display_parts(),
            Command::PushNotify(command) => command.display_parts(),
            Command::SetLightNotification(command) => command.display_parts(),
            Command::SetEnergySaving(command) => command.display_parts(),
            Command::ControlFan(command) => command.display_parts(),
            Command::OpenDoor(command) => command.display_parts(),
        }
    }

    fn deduplicate_when_unobservable(&self) -> bool {
        match self {
            Command::SetPower(command) => command.deduplicate_when_unobservable(),
            Command::SetHeating(command) => command.deduplicate_when_unobservable(),
            Command::PushNotify(command) => command.deduplicate_when_unobservable(),
            Command::SetLightNotification(command) => command.deduplicate_when_unobservable(),
            Command::SetEnergySaving(command) => command.deduplicate_when_unobservable(),
            Command::ControlFan(command) => command.deduplicate_when_unobservable(),
            Command::OpenDoor(command) => command.deduplicate_when_unobservable(),
        }
    }

    fn is_reflected_in_state(&self, snapshot: &crate::home_state::StateSnapshot) -> anyhow::Result<Option<bool>> {
        match self {
            Command::SetPower(command) => command.is_reflected_in_state(snapshot),
            Command::SetHeating(command) => command.is_reflected_in_state(snapshot),
            Command::PushNotify(command) => command.is_reflected_in_state(snapshot),
            Command::SetLightNotification(command) => command.is_reflected_in_state(snapshot),
            Command::SetEnergySaving(command) => command.is_reflected_in_state(snapshot),
            Command::ControlFan(command) => command.is_reflected_in_state(snapshot),
            Command::OpenDoor(command) => command.is_reflected_in_state(snapshot),
        }
    }
}

impl CommandTargetBehavior for CommandTarget {
    type Command = Command;

    fn to_default_command(&self) -> Self::Command {
        match self {
            CommandTarget::SetPower(target) => target.to_default_command().into(),
            CommandTarget::SetHeating(target) => target.to_default_command().into(),
            CommandTarget::PushNotify(target) => target.to_default_command().into(),
            CommandTarget::SetLightNotification(target) => target.to_default_command().into(),
            CommandTarget::SetEnergySaving(target) => target.to_default_command().into(),
            CommandTarget::ControlFan(target) => target.to_default_command().into(),
            CommandTarget::OpenDoor(target) => target.to_default_command().into(),
        }
    }
}

impl From<Command> for CommandTarget {
    fn from(command: Command) -> Self {
        CommandTarget::from(&command)
    }
}

impl From<&Command> for CommandTarget {
    fn from(command: &Command) -> Self {
        command.target()
    }
}

impl Command {
    pub fn display_parts(&self) -> CommandDisplayParts {
        CommandBehavior::display_parts(self)
    }

    pub fn deduplicate_when_unobservable(&self) -> bool {
        CommandBehavior::deduplicate_when_unobservable(self)
    }

    pub fn is_reflected_in_state(&self, snapshot: &crate::home_state::StateSnapshot) -> anyhow::Result<Option<bool>> {
        CommandBehavior::is_reflected_in_state(self, snapshot)
    }
}

impl CommandTarget {
    pub fn to_default_command(&self) -> Command {
        CommandTargetBehavior::to_default_command(self)
    }
}

impl std::fmt::Display for CommandTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandTarget::SetPower(target) => target.fmt(f),
            CommandTarget::SetHeating(target) => target.fmt(f),
            CommandTarget::PushNotify(target) => target.fmt(f),
            CommandTarget::SetLightNotification(target) => target.fmt(f),
            CommandTarget::SetEnergySaving(target) => target.fmt(f),
            CommandTarget::ControlFan(target) => target.fmt(f),
            CommandTarget::OpenDoor(target) => target.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandDisplayParts {
    pub command_type: &'static str,
    pub target: String,
    pub state: String,
}

#[cfg(test)]
mod test {
    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn light_notification_serializes_its_signal_and_state() {
        assert_json_eq!(
            Command::SetLightNotification(SetLightNotification {
                device: NotificationLight::LivingRoom,
                signal: LightSignal::WindowOpened,
                on: true,
            }),
            json!({
                "set_light_notification": {
                    "device": "living_room",
                    "signal": "window_opened",
                    "on": true
                }
            })
        );
    }

    #[test]
    fn notification_targets_have_clear_ids_and_display() {
        let push_command = Command::PushNotify(PushNotify {
            notification: PushNotification::WindowOpened,
            recipient: PushRecipient::Dennis,
            operation: PushNotifyOperation::Show,
        });
        let push_target = CommandTarget::from(&push_command);

        assert_eq!(push_target.ext_id().to_string(), "push_notify_target::window_opened::dennis");
        assert_eq!(push_target.to_string(), "PushNotify[WindowOpened @ Dennis]");
        assert_eq!(push_command.display_parts().target, "WindowOpened @ Dennis");
        assert_eq!(push_command.display_parts().state, "Show");

        let light_command = Command::SetLightNotification(SetLightNotification {
            device: NotificationLight::LivingRoom,
            signal: LightSignal::WindowOpened,
            on: true,
        });
        let light_target = CommandTarget::from(&light_command);

        assert_eq!(light_target.ext_id().to_string(), "set_light_notification_target::living_room");
        assert_eq!(light_target.to_string(), "SetLightNotification[LivingRoom]");
    }
}
