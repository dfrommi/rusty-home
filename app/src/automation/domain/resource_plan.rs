use crate::automation::domain::action::{
    AutoTurnOff, BlockAutomation, Dehumidify, FollowDefaultSetting, FollowTargetHeatingDemand, HomeAction,
    InformWindowOpen, PauseHeatingForVentilation, PurifyAir, RemoteTurnOff, UserTriggerAction,
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
    let mut plan: Vec<(CommandTarget, Vec<HomeAction>)> = vec![
        // --- Power devices ---
        (
            CommandTarget::SetPower {
                device: PowerToggle::BedroomAirPurifier,
            },
            vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::BedroomAirPurifier)).into(),
                PurifyAir::BedroomSleepPreparation.into(),
                PurifyAir::BedroomAfterVentilation.into(),
            ],
        ),
        (
            CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            },
            vec![
                BlockAutomation::BathroomDehumidifier.into(),
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier)).into(),
                Dehumidify::Bathroom.into(),
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
            ],
        ),
        (
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomTvAmbilight,
            },
            vec![UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvAmbilight)).into()],
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
            ],
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::LivingRoomAirPurifier,
            },
            vec![
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::LivingRoomAirPurifier)).into(),
                PurifyAir::LivingRoom.into(),
            ],
        ),
        // --- Heating devices (one per radiator) ---
        (
            CommandTarget::SetHeating {
                device: Radiator::LivingRoomBig,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::LivingRoomBig).into(),
                FollowTargetHeatingDemand::new(Radiator::LivingRoomBig).into(),
            ],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::LivingRoomSmall,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::LivingRoomSmall).into(),
                FollowTargetHeatingDemand::new(Radiator::LivingRoomSmall).into(),
            ],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Bedroom,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::Bedroom).into(),
                FollowTargetHeatingDemand::new(Radiator::Bedroom).into(),
            ],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Kitchen,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::Kitchen).into(),
                FollowTargetHeatingDemand::new(Radiator::Kitchen).into(),
            ],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::RoomOfRequirements,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::RoomOfRequirements).into(),
                FollowTargetHeatingDemand::new(Radiator::RoomOfRequirements).into(),
            ],
        ),
        (
            CommandTarget::SetHeating {
                device: Radiator::Bathroom,
            },
            vec![
                PauseHeatingForVentilation::new(Radiator::Bathroom).into(),
                FollowTargetHeatingDemand::new(Radiator::Bathroom).into(),
            ],
        ),
        // --- Energy saving ---
        (
            CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            },
            vec![UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvEnergySaving)).into()],
        ),
        // --- Notifications ---
        (
            CommandTarget::NotifyLight {
                device: NotificationLight::LivingRoom,
            },
            vec![InformWindowOpen::NotificationLightLivingRoom.into()],
        ),
        (
            CommandTarget::NotifyPhone {
                recipient: NotificationRecipient::Dennis,
                notification: NotificationKind::WindowOpened,
            },
            vec![InformWindowOpen::PushNotification(NotificationRecipient::Dennis).into()],
        ),
        (
            CommandTarget::NotifyPhone {
                recipient: NotificationRecipient::Sabine,
                notification: NotificationKind::WindowOpened,
            },
            vec![InformWindowOpen::PushNotification(NotificationRecipient::Sabine).into()],
        ),
        // --- Door ---
        (
            CommandTarget::OpenDoor {
                device: crate::command::Lock::BuildingEntrance,
            },
            vec![UserTriggerAction::new(UserTriggerTarget::OpenDoor(Door::Building)).into()],
        ),
    ];

    for (_, rules) in &mut plan {
        if !matches!(rules.last(), Some(HomeAction::FollowDefaultSetting(_))) {
            rules.push(FollowDefaultSetting::GlobalDefault.into());
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::core::domain::{HeatingZone, RoomWithWindow};

    #[test]
    fn every_radiator_pauses_for_ventilation_before_following_its_target() {
        for radiator in Radiator::variants() {
            let target = CommandTarget::SetHeating { device: *radiator };
            let (_, rules) = resource_plans()
                .into_iter()
                .find(|(candidate, _)| candidate == &target)
                .expect("radiator heating plan");
            assert!(matches!(
                rules.as_slice(),
                [
                    HomeAction::PauseHeatingForVentilation(_),
                    HomeAction::FollowTargetHeatingDemand(_),
                    HomeAction::FollowDefaultSetting(_),
                ]
            ));
        }
        assert_eq!(HeatingZone::Bathroom.room_with_window(), RoomWithWindow::Bedroom);
    }

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
