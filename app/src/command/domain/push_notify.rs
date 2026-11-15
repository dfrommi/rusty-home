use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    core::time::DateTime,
    home_state::StateSnapshot,
};
use anyhow::Result;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PushNotify {
    Dismiss(PushNotifyTarget),
    Show(PushRecipient, PushNotification),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, derive_more::Display)]
#[serde(rename_all = "snake_case")]
pub enum PushNotification {
    WindowOpened,
    WashingMachine(WashingMachineState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, derive_more::Display)]
#[serde(rename_all = "snake_case")]
pub enum WashingMachineState {
    Done,
    #[display("InProgess({})", estimated_end)]
    InProgress {
        started_at: DateTime,
        estimated_end: DateTime,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Id, Serialize, derive_more::Display)]
#[serde(rename_all = "snake_case")]
pub enum PushNotifyTarget {
    #[display("WindowOpened @ {}", _0)]
    WindowOpened(PushRecipient),
    #[display("WashingMachine @ {}", _0)]
    WashingMachine(PushRecipient),
}

impl PushNotifyTarget {
    pub fn recipient(&self) -> PushRecipient {
        match self {
            PushNotifyTarget::WindowOpened(recipient) => *recipient,
            PushNotifyTarget::WashingMachine(recipient) => *recipient,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum PushRecipient {
    Dennis,
    Sabine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, derive_more::Display)]
#[serde(rename_all = "snake_case")]
pub enum PushNotifyOperation {
    Show,
    Dismiss,
}

impl From<&PushNotify> for PushNotifyTarget {
    fn from(command: &PushNotify) -> Self {
        match command {
            PushNotify::Dismiss(target) => *target,
            PushNotify::Show(recipient, PushNotification::WindowOpened) => PushNotifyTarget::WindowOpened(*recipient),
            PushNotify::Show(recipient, PushNotification::WashingMachine { .. }) => {
                PushNotifyTarget::WashingMachine(*recipient)
            }
        }
    }
}

impl CommandBehavior for PushNotify {
    type Target = PushNotifyTarget;

    fn target(&self) -> Self::Target {
        PushNotifyTarget::from(self)
    }

    fn display_parts(&self) -> CommandDisplayParts {
        let target: PushNotifyTarget = self.into();
        let operation = match self {
            PushNotify::Dismiss(_) => PushNotifyOperation::Dismiss,
            PushNotify::Show(_, _) => PushNotifyOperation::Show,
        };

        CommandDisplayParts {
            command_type: "PushNotify",
            target: target.to_string(),
            state: operation.to_string(),
        }
    }

    fn command_state(&self, _snapshot: &StateSnapshot) -> Result<CommandState> {
        Ok(CommandState::UnobservableDeduplicated)
    }
}

impl CommandTargetBehavior for PushNotifyTarget {
    type Command = PushNotify;

    fn to_default_command(&self) -> Option<Self::Command> {
        PushNotify::Dismiss(*self).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_command_targets_the_notification_for_its_recipient() {
        let command = PushNotify::Show(PushRecipient::Dennis, PushNotification::WindowOpened);

        assert_eq!(command.target(), PushNotifyTarget::WindowOpened(PushRecipient::Dennis));
        assert_eq!(command.display_parts().state, "Show");
    }

    #[test]
    fn target_creates_a_dismiss_command_by_default() {
        let target = PushNotifyTarget::WindowOpened(PushRecipient::Sabine);

        let command = target
            .to_default_command()
            .expect("push notification target has a default command");

        assert_eq!(command, PushNotify::Dismiss(target));
    }

    #[test]
    fn push_notification_is_unobservable_and_deduplicated() {
        let command = PushNotify::Show(PushRecipient::Dennis, PushNotification::WindowOpened);

        assert_eq!(command.display_parts().command_type, "PushNotify");
        assert_eq!(command.display_parts().target, "WindowOpened @ Dennis");
        assert_eq!(command.display_parts().state, "Show");
        assert!(matches!(
            command.command_state(&StateSnapshot::default()),
            Ok(CommandState::UnobservableDeduplicated)
        ));
    }
}
