use crate::automation::{HeatingZone, Room};
use crate::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};
use crate::frontends::homekit::HomekitCommandTarget;

use super::action::{
    AutoTurnOff, FollowDefaultSetting, FollowTargetHeatingDemand, InformWindowOpen, ReduceNoiseAtNight,
    SupportVentilationWithFan, UserTriggerAction,
};
use super::action::{Dehumidify, HomeAction};
use super::goal::HomeGoal;

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
        vec![
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomBigHeatingDemand.into()).into(),
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomSmallHeatingDemand.into()).into(),
            FollowTargetHeatingDemand::new(HeatingZone::LivingRoom).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::Bedroom),
        {
            vec![
                UserTriggerAction::new(HomekitCommandTarget::InfraredHeaterPower.into()).into(),
                AutoTurnOff::IrHeater.into(),
                UserTriggerAction::new(HomekitCommandTarget::BedroomHeatingDemand.into()).into(),
                FollowTargetHeatingDemand::new(HeatingZone::Bedroom).into(),
            ] 
        }
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::Kitchen),
        vec![
            UserTriggerAction::new(HomekitCommandTarget::KitchenHeatingDemand.into()).into(),
            FollowTargetHeatingDemand::new(HeatingZone::Kitchen).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
        vec![
            UserTriggerAction::new(HomekitCommandTarget::RoomOfRequirementsHeatingDemand.into()).into(),
            FollowTargetHeatingDemand::new(HeatingZone::RoomOfRequirements).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::Bathroom),
        vec![
            UserTriggerAction::new(HomekitCommandTarget::BathroomHeatingDemand.into()).into(),
            FollowTargetHeatingDemand::new(HeatingZone::Bathroom).into(),
        ]
    ),
    (
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        vec![
            SupportVentilationWithFan::LivingRoomCeilingFan.into(),
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomCeilingFanSpeed.into()).into(),
        ]
    ),
    (
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        vec![
            SupportVentilationWithFan::BedroomCeilingFan.into(),
            UserTriggerAction::new(HomekitCommandTarget::BedroomCeilingFanSpeed.into()).into(),
            UserTriggerAction::new(HomekitCommandTarget::BedroomDehumidifierFanSpeed.into()).into(),
        ]
    ),
    (
        HomeGoal::StayInformed,
        vec![
            InformWindowOpen::NotificationLightLivingRoom.into(),
            InformWindowOpen::PushNotification(NotificationRecipient::Dennis).into(),
            InformWindowOpen::PushNotification(NotificationRecipient::Sabine).into(),
        ],
    ),
    (
        HomeGoal::PreventMouldInBathroom,
        vec![
            UserTriggerAction::new(HomekitCommandTarget::DehumidifierPower.into()).into(),
            ReduceNoiseAtNight::Dehumidifier.into(),
            Dehumidify::Dehumidifier.into()
        ],
    ),
    (
        HomeGoal::TvControl,
        vec![
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomTvEnergySaving.into()).into(),
            FollowDefaultSetting::new(CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            }).into(),
        ]
    ),
    (
        HomeGoal::ResetToDefaltSettings,
        vec![
            FollowDefaultSetting::new(CommandTarget::SetPower {
                device: PowerToggle::Dehumidifier,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetPower {
                device: PowerToggle::InfraredHeater,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::LivingRoomCeilingFan,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::BedroomCeilingFan,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::BedroomDehumidifier,
            }).into(),
        ]
    )
    ]
}
