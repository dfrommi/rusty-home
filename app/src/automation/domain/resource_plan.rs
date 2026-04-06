use crate::automation::domain::action::{
    AutoTurnOff, BlockAutomation, Dehumidify, FollowDefaultSetting, FollowTargetHeatingDemand, HomeAction,
    InformWindowOpen, RemoteTurnOff, SupportWithFan, UserTriggerAction,
};
use crate::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};
use crate::core::domain::Radiator;
use crate::home_state::FanActivity;
use crate::trigger::{Door, OnOffDevice, UserTriggerTarget};

/// Single source of truth: what controls each device and in what order.
/// Rules are listed in priority order per resource — first non-Skip wins.
pub fn resource_plans() -> Vec<(CommandTarget, Vec<HomeAction>)> {
    vec![
        // --- Power devices ---
        (
            CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
            vec![
                BlockAutomation::BathroomDehumidifier.into(),
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier)).into(),
                Dehumidify::Bathroom.into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::Dehumidifier,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::SetPower {
                device: PowerToggle::InfraredHeater,
            },
            vec![
                RemoteTurnOff::InfraredHeater.into(),
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::InfraredHeater)).into(),
                AutoTurnOff::IrHeater.into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::InfraredHeater,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
            },
            vec![
                InformWindowOpen::NotificationLightLivingRoom.into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::LivingRoomNotificationLight,
                })
                .into(),
            ],
        ),
        // --- Fan devices ---
        (
            CommandTarget::ControlFan {
                device: Fan::BedroomCeilingFan,
            },
            vec![
                BlockAutomation::BedroomCeilingFan.into(),
                RemoteTurnOff::BedroomCeilingFan.into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::BedroomCeilingFan)).into(),
                SupportWithFan::BedroomVentilation.into(),
                SupportWithFan::BedroomDehumidification.into(),
                SupportWithFan::BedroomHeating.into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::BedroomCeilingFan,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::BedroomDehumidifier,
            },
            vec![
                BlockAutomation::BedroomDehumidifier.into(),
                RemoteTurnOff::BedroomDehumidifier.into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::BedroomDehumidifier)).into(),
                Dehumidify::Bedroom.into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::BedroomDehumidifier,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::LivingRoomCeilingFan,
            },
            vec![
                SupportWithFan::LivingRoomVentilation.into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::LivingRoomCeilingFan)).into(),
                // SupportWithFan::LivingRoomHeating currently disabled
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::LivingRoomCeilingFan,
                })
                .into(),
            ],
        ),
        // --- Heating devices (one per radiator) ---
        (
            CommandTarget::SetHeating {
                device: Radiator::LivingRoomBig,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::LivingRoomBig).into()],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::LivingRoomSmall,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::LivingRoomSmall).into()],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Bedroom,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::Bedroom).into()],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Kitchen,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::Kitchen).into()],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::RoomOfRequirements,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::RoomOfRequirements).into()],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Bathroom,
            },
            vec![FollowTargetHeatingDemand::new(Radiator::Bathroom).into()],
        ),
        // --- Energy saving ---
        (
            CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            },
            vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvEnergySaving)).into(),
                FollowDefaultSetting::new(CommandTarget::SetEnergySaving {
                    device: EnergySavingDevice::LivingRoomTv,
                })
                .into(),
            ],
        ),
        // --- Notifications ---
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: Notification::WindowOpened,
            },
            vec![
                InformWindowOpen::PushNotification(NotificationRecipient::Dennis).into(),
                FollowDefaultSetting::new(CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Dennis,
                    notification: Notification::WindowOpened,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: Notification::WindowOpened,
            },
            vec![
                InformWindowOpen::PushNotification(NotificationRecipient::Sabine).into(),
                FollowDefaultSetting::new(CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Sabine,
                    notification: Notification::WindowOpened,
                })
                .into(),
            ],
        ),
        // --- Door ---
        (
            CommandTarget::OpenDoor {
                device: crate::command::Lock::BuildingEntrance,
            },
            vec![UserTriggerAction::new(UserTriggerTarget::OpenDoor(Door::Building)).into()],
        ),
    ]
}
