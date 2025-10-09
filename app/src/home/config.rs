use crate::adapter::homekit::HomekitCommandTarget;
use crate::home::command::{CommandTarget, Fan, NotificationRecipient, PowerToggle, Thermostat};
use crate::home::common::HeatingZone;
use crate::home::trigger::RemoteTarget;

use super::action::{Dehumidify, HomeAction};
use super::goal::{HomeGoal, Room};
use crate::home::action::{
    AutoTurnOff, FollowDefaultSetting, FollowHeatingSchedule, InformWindowOpen, ProvideAmbientTemperature,
    ReduceNoiseAtNight, SupportVentilationWithFan, UserTriggerAction,
};
use crate::home::state::HeatingMode;

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(Room::LivingRoom),
        smarter_heating_actions(HeatingZone::LivingRoom)
    ),
    (
        HomeGoal::SmarterHeating(Room::Bedroom),
        {
            let mut a = vec![
                UserTriggerAction::new(HomekitCommandTarget::InfraredHeaterPower.into()).into(),
                UserTriggerAction::new(RemoteTarget::BedroomDoor.into()).into(),
                AutoTurnOff::IrHeater.into(),
            ]; 
            a.extend(smarter_heating_actions(HeatingZone::Bedroom));
            a
        }
    ),
    (
        HomeGoal::SmarterHeating(Room::Kitchen),
        smarter_heating_actions(HeatingZone::Kitchen)
    ),
    (
        HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        smarter_heating_actions(HeatingZone::RoomOfRequirements)
    ),
    (
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        vec![
            SupportVentilationWithFan::new(Fan::LivingRoomCeilingFan).into(),
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomCeilingFanSpeed.into()).into(),
            //CoolDownWhenOccupied::LivingRoom.into(),
        ]
    ),
    (
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        vec![
            SupportVentilationWithFan::new(Fan::BedroomCeilingFan).into(),
            UserTriggerAction::new(HomekitCommandTarget::BedroomCeilingFanSpeed.into()).into(),
            //CoolDownWhenOccupied::Bedroom.into(),
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
                device: crate::home::command::EnergySavingDevice::LivingRoomTv,
            }).into(),
        ]
    ),
    (
        HomeGoal::CoreControl,
        vec![
            ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomBig).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomSmall).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::Bedroom).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::Kitchen).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::RoomOfRequirements).into(), 
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
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::LivingRoomBig,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::LivingRoomSmall,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::Bedroom,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::RoomOfRequirements,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::Kitchen,
            }).into(),
            // FollowDefaultSetting::new(CommandTarget::SetHeating {
            //     device: Thermostat::Bathroom,
            // }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: crate::home::command::Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: crate::home::command::Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::LivingRoomCeilingFan,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::BedroomCeilingFan,
            }).into(),
        ]
    )
    ]
}

fn smarter_heating_actions(zone: HeatingZone) -> Vec<HomeAction> {
    vec![
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Away).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Ventilation).into(),
        UserTriggerAction::new(
            match zone {
                HeatingZone::LivingRoom => HomekitCommandTarget::LivingRoomHeatingState,
                HeatingZone::Bedroom => HomekitCommandTarget::BedroomHeatingState,
                HeatingZone::Kitchen => HomekitCommandTarget::KitchenHeatingState,
                HeatingZone::RoomOfRequirements => HomekitCommandTarget::RoomOfRequirementsHeatingState,
                HeatingZone::Bathroom => panic!("Smart Heating for Bathroom currently not supported"),
            }
            .into(),
        )
        .into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::PostVentilation).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Sleep).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Comfort).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::EnergySaving).into(),
    ]
}
