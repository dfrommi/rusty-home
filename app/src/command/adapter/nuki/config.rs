use super::NukiCommandTarget;
use crate::command::{CommandTarget, Lock};

pub fn default_nuki_command_config() -> Vec<(CommandTarget, NukiCommandTarget)> {
    vec![(
        CommandTarget::OpenDoor {
            device: Lock::BuildingEntrance,
        },
        NukiCommandTarget::Opener("1CC90CCA"),
    )]
}
