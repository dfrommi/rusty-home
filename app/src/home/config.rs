use crate::adapter::homekit::HomekitCommandTarget;
use crate::home::command::{CommandTarget, Fan, NotificationRecipient, PowerToggle, Thermostat};
use crate::home::common::HeatingZone;
use crate::home::trigger::RemoteTarget;
use crate::t;

use crate::home::action::{
    FollowDefaultSetting, FollowHeatingSchedule, InformWindowOpen, IrHeaterAutoTurnOff, ProvideAmbientTemperature,
    ReduceNoiseAtNight, SupportVentilationWithFan, UserTriggerAction,
};
use crate::home::state::{HeatingMode, UserControlled};

use super::action::{Dehumidify, HomeAction, KeepUserOverride, RequestClosingWindow};
use super::goal::{HomeGoal, Room};

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
                IrHeaterAutoTurnOff::new().into(),
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
            //RoolDownWhenOccupied::Bedroom.into(),
        ]
    ),
    // (
    //     HomeGoal::SmarterHeating(Room::Bathroom),
    //     vec![
    //         NoHeatingDuringVentilation::new(HeatingZone::Bathroom).into(),
    //         KeepUserOverride::new(UserControlled::BathroomThermostat).into(),
    //         NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bathroom).into(),
    //     ]
    // ),
    (
        HomeGoal::StayInformed,
        vec![
            RequestClosingWindow::new().into(),
            InformWindowOpen::new(NotificationRecipient::Dennis).into(),
            InformWindowOpen::new(NotificationRecipient::Sabine).into(),
        ],
    ),
    (
        HomeGoal::PreventMouldInBathroom,
        vec![
            UserTriggerAction::new(HomekitCommandTarget::DehumidifierPower.into()).into(),
            KeepUserOverride::new(UserControlled::Dehumidifier).into(),
            ReduceNoiseAtNight::new(t!(22:30 - 12:00)).into(),
            Dehumidify::new().into()
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
            ProvideAmbientTemperature::LivingRoomThermostatBig.into(), 
            ProvideAmbientTemperature::BedroomThermostat.into(), 
            ProvideAmbientTemperature::KitchenThermostat.into(), 
            ProvideAmbientTemperature::RoomOfRequirementsThermostat.into(), 
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
                device: Thermostat::LivingRoom,
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
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::PostVentilation).into(),
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
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Sleep).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::Comfort).into(),
        FollowHeatingSchedule::new(zone.clone(), HeatingMode::EnergySaving).into(),
    ]
}
