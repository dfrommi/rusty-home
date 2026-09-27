use crate::automation::domain::action::{
    AutoTurnOff, BlockAutomation, Dehumidify, FollowDefaultSetting, FollowTargetHeatingDemand, HomeAction,
    InformWindowOpen, PurifyAir, RemoteTurnOff, UserTriggerAction,
};
use crate::command::{
    CommandTarget, EnergySavingDevice, Fan, NotificationKind, NotificationLight, NotificationRecipient, PowerToggle,
};
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
                device: PowerToggle::LivingRoomTvAmbilight,
            },
            vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvAmbilight)).into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::LivingRoomTvAmbilight,
                })
                .into(),
            ],
        ),
        // --- Fan devices ---
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
                device: Fan::LivingRoomAirPurifier,
            },
            vec![
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::LivingRoomAirPurifier)).into(),
                PurifyAir::LivingRoom.into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::LivingRoomAirPurifier,
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
            CommandTarget::NotifyLight {
                device: NotificationLight::LivingRoom,
            },
            vec![
                InformWindowOpen::NotificationLightLivingRoom.into(),
                FollowDefaultSetting::new(CommandTarget::NotifyLight {
                    device: NotificationLight::LivingRoom,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::NotifyPhone {
                recipient: NotificationRecipient::Dennis,
                notification: NotificationKind::WindowOpened,
            },
            vec![
                InformWindowOpen::PushNotification(NotificationRecipient::Dennis).into(),
                FollowDefaultSetting::new(CommandTarget::NotifyPhone {
                    recipient: NotificationRecipient::Dennis,
                    notification: NotificationKind::WindowOpened,
                })
                .into(),
            ],
        ),
        (
            CommandTarget::NotifyPhone {
                recipient: NotificationRecipient::Sabine,
                notification: NotificationKind::WindowOpened,
            },
            vec![
                InformWindowOpen::PushNotification(NotificationRecipient::Sabine).into(),
                FollowDefaultSetting::new(CommandTarget::NotifyPhone {
                    recipient: NotificationRecipient::Sabine,
                    notification: NotificationKind::WindowOpened,
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn notification_plans_are_keyed_by_their_lockable_targets() {
        let plans = resource_plans();
        let mut targets = HashSet::new();
        for (target, _) in &plans {
            assert!(targets.insert(target.clone()), "duplicate resource plan for {target}");
        }

        let light_target = CommandTarget::NotifyLight {
            device: NotificationLight::LivingRoom,
        };
        let (_, light_rules) = plans
            .iter()
            .find(|(target, _)| target == &light_target)
            .expect("living-room notification light plan");
        assert!(matches!(
            light_rules.first(),
            Some(HomeAction::InformWindowOpen(InformWindowOpen::NotificationLightLivingRoom))
        ));
        assert!(matches!(light_rules.last(), Some(HomeAction::FollowDefaultSetting(_))));

        for recipient in [NotificationRecipient::Dennis, NotificationRecipient::Sabine] {
            let phone_target = CommandTarget::NotifyPhone {
                recipient,
                notification: NotificationKind::WindowOpened,
            };
            assert!(plans.iter().any(|(target, _)| target == &phone_target));
        }
    }

    #[test]
    fn ambilight_follows_user_requests_before_its_default() {
        let target = CommandTarget::SetPower {
            device: PowerToggle::LivingRoomTvAmbilight,
        };
        let (_, rules) = resource_plans()
            .into_iter()
            .find(|(candidate, _)| candidate == &target)
            .expect("living-room Ambilight plan");

        assert!(matches!(
            rules.as_slice(),
            [HomeAction::UserTriggerAction(_), HomeAction::FollowDefaultSetting(_)]
        ));
    }
}
