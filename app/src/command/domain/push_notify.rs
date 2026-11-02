use crate::{
    command::{CommandBehavior, CommandDisplayParts, CommandState, CommandTargetBehavior},
    home_state::StateSnapshot,
};
use anyhow::Result;
use r#macro::{EnumVariants, Id};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, derive_more::Display, Id, EnumVariants)]
#[serde(rename_all = "snake_case")]
pub enum PushNotification {
    WindowOpened,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PushNotify {
    pub notification: PushNotification,
    pub recipient: PushRecipient,
    pub operation: PushNotifyOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, Id)]
#[display("PushNotify[{} @ {}]", notification, recipient)]
pub struct PushNotifyTarget {
    pub notification: PushNotification,
    pub recipient: PushRecipient,
}

impl PushNotifyTarget {
    pub fn to_command(self, operation: PushNotifyOperation) -> PushNotify {
        PushNotify {
            notification: self.notification,
            recipient: self.recipient,
            operation,
        }
    }
}

impl From<PushNotify> for PushNotifyTarget {
    fn from(command: PushNotify) -> Self {
        Self::from(&command)
    }
}

impl From<&PushNotify> for PushNotifyTarget {
    fn from(command: &PushNotify) -> Self {
        Self {
            notification: command.notification,
            recipient: command.recipient,
        }
    }
}

impl CommandBehavior for PushNotify {
    type Target = PushNotifyTarget;

    fn target(&self) -> Self::Target {
        self.into()
    }

    fn display_parts(&self) -> CommandDisplayParts {
        CommandDisplayParts {
            command_type: "PushNotify",
            target: format!("{} @ {}", self.notification, self.recipient),
            state: self.operation.to_string(),
        }
    }

    fn command_state(&self, _snapshot: &StateSnapshot) -> Result<CommandState> {
        Ok(CommandState::UnobservableDeduplicated)
    }
}

impl CommandTargetBehavior for PushNotifyTarget {
    type Command = PushNotify;

    fn to_default_command(&self) -> Self::Command {
        self.to_command(PushNotifyOperation::Dismiss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_creates_a_command_with_the_requested_operation() {
        let target = PushNotifyTarget {
            notification: PushNotification::WindowOpened,
            recipient: PushRecipient::Dennis,
        };

        let command = target.to_command(PushNotifyOperation::Show);

        assert_eq!(command.notification, PushNotification::WindowOpened);
        assert_eq!(command.recipient, PushRecipient::Dennis);
        assert_eq!(command.operation, PushNotifyOperation::Show);
    }

    #[test]
    fn target_creates_a_dismiss_command_by_default() {
        let target = PushNotifyTarget {
            notification: PushNotification::WindowOpened,
            recipient: PushRecipient::Sabine,
        };

        let command = target.to_default_command();

        assert_eq!(command.operation, PushNotifyOperation::Dismiss);
    }

    #[test]
    fn push_notification_is_unobservable_and_deduplicated() {
        let command = PushNotify {
            notification: PushNotification::WindowOpened,
            recipient: PushRecipient::Dennis,
            operation: PushNotifyOperation::Show,
        };

        assert_eq!(command.display_parts().command_type, "PushNotify");
        assert_eq!(command.display_parts().target, "WindowOpened @ Dennis");
        assert_eq!(command.display_parts().state, "Show");
        assert!(matches!(
            command.command_state(&StateSnapshot::default()),
            Ok(CommandState::UnobservableDeduplicated)
        ));
    }
}
