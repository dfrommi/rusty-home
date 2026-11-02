use crate::automation::domain::action::{
    AutoTurnOff, BlockAutomation, Dehumidify, FollowDefaultSetting, FollowTargetHeatingDemand, HomeAction,
    InformWindowOpen, PauseHeatingForVentilation, PurifyAir, RemoteTurnOff, UserTriggerAction,
};
use crate::command::{
    CommandTarget, ControlFanTarget, EnergySavingDevice, Fan, NotificationLight, OpenDoorTarget, PowerToggle,
    PushNotification, PushNotifyTarget, PushRecipient, SetEnergySavingTarget, SetHeatingTarget,
    SetLightNotificationTarget, SetPowerTarget,
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
            SetPowerTarget {
                device: PowerToggle::BedroomAirPurifier,
            }
            .into(),
            vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::BedroomAirPurifier)).into(),
                PurifyAir::BedroomSleepPreparation.into(),
                PurifyAir::BedroomAfterVentilation.into(),
            ],
        ),
        (
            SetPowerTarget {
                device: PowerToggle::Dehumidifier,
            }
            .into(),
            vec![
                BlockAutomation::BathroomDehumidifier.into(),
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier)).into(),
                Dehumidify::Bathroom.into(),
            ],
        ),
        (
            SetPowerTarget {
                device: PowerToggle::InfraredHeater,
            }
            .into(),
            vec![
                RemoteTurnOff::InfraredHeater.into(),
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::InfraredHeater)).into(),
                AutoTurnOff::IrHeater.into(),
            ],
        ),
        (
            SetPowerTarget {
                device: PowerToggle::LivingRoomTvAmbilight,
            }
            .into(),
            vec![UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvAmbilight)).into()],
        ),
        // --- Fan devices ---
        (
            ControlFanTarget {
                device: Fan::BedroomDehumidifier,
            }
            .into(),
            vec![
                BlockAutomation::BedroomDehumidifier.into(),
                RemoteTurnOff::BedroomDehumidifier.into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::BedroomDehumidifier)).into(),
                Dehumidify::Bedroom.into(),
            ],
        ),
        (
            ControlFanTarget {
                device: Fan::LivingRoomAirPurifier,
            }
            .into(),
            vec![
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::LivingRoomAirPurifier)).into(),
                PurifyAir::LivingRoom.into(),
            ],
        ),
        // --- Heating devices (one per radiator) ---
        (
            SetHeatingTarget {
                device: Radiator::LivingRoomBig,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::LivingRoomBig).into(),
                FollowTargetHeatingDemand::new(Radiator::LivingRoomBig).into(),
            ],
        ),
        (
            SetHeatingTarget {
                device: Radiator::LivingRoomSmall,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::LivingRoomSmall).into(),
                FollowTargetHeatingDemand::new(Radiator::LivingRoomSmall).into(),
            ],
        ),
        (
            SetHeatingTarget {
                device: Radiator::Bedroom,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::Bedroom).into(),
                FollowTargetHeatingDemand::new(Radiator::Bedroom).into(),
            ],
        ),
        (
            SetHeatingTarget {
                device: Radiator::Kitchen,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::Kitchen).into(),
                FollowTargetHeatingDemand::new(Radiator::Kitchen).into(),
            ],
        ),
        (
            SetHeatingTarget {
                device: Radiator::RoomOfRequirements,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::RoomOfRequirements).into(),
                FollowTargetHeatingDemand::new(Radiator::RoomOfRequirements).into(),
            ],
        ),
        (
            SetHeatingTarget {
                device: Radiator::Bathroom,
            }
            .into(),
            vec![
                PauseHeatingForVentilation::new(Radiator::Bathroom).into(),
                FollowTargetHeatingDemand::new(Radiator::Bathroom).into(),
            ],
        ),
        // --- Energy saving ---
        (
            SetEnergySavingTarget {
                device: EnergySavingDevice::LivingRoomTv,
            }
            .into(),
            vec![UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvEnergySaving)).into()],
        ),
        // --- Notifications ---
        (
            SetLightNotificationTarget {
                device: NotificationLight::LivingRoom,
            }
            .into(),
            vec![InformWindowOpen::NotificationLightLivingRoom.into()],
        ),
        (
            PushNotifyTarget {
                recipient: PushRecipient::Dennis,
                notification: PushNotification::WindowOpened,
            }
            .into(),
            vec![InformWindowOpen::PushNotification(PushRecipient::Dennis).into()],
        ),
        (
            PushNotifyTarget {
                recipient: PushRecipient::Sabine,
                notification: PushNotification::WindowOpened,
            }
            .into(),
            vec![InformWindowOpen::PushNotification(PushRecipient::Sabine).into()],
        ),
        // --- Door ---
        (
            OpenDoorTarget {
                device: crate::command::Lock::BuildingEntrance,
            }
            .into(),
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
            let target: CommandTarget = SetHeatingTarget { device: *radiator }.into();
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

        let light_target: CommandTarget = SetLightNotificationTarget {
            device: NotificationLight::LivingRoom,
        }
        .into();
        let (_, light_rules) = plans
            .iter()
            .find(|(target, _)| target == &light_target)
            .expect("living-room notification light plan");
        assert!(matches!(
            light_rules.first(),
            Some(HomeAction::InformWindowOpen(InformWindowOpen::NotificationLightLivingRoom))
        ));
        assert!(matches!(light_rules.last(), Some(HomeAction::FollowDefaultSetting(_))));

        for recipient in [PushRecipient::Dennis, PushRecipient::Sabine] {
            let phone_target: CommandTarget = PushNotifyTarget {
                recipient,
                notification: PushNotification::WindowOpened,
            }
            .into();
            assert!(plans.iter().any(|(target, _)| target == &phone_target));
        }
    }

    #[test]
    fn ambilight_follows_user_requests_before_its_default() {
        let target: CommandTarget = SetPowerTarget {
            device: PowerToggle::LivingRoomTvAmbilight,
        }
        .into();
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
