use api::command::{CommandTarget, Fan, NotificationRecipient, PowerToggle, Thermostat};
use api::state::FanSpeed;
use api::trigger::{HomekitTarget, RemoteTarget};
use support::t;

use crate::home::action::{
    FollowDefaultSetting, HeatingZone, InformWindowOpen, IrHeaterAutoTurnOff, ReduceNoiseAtNight,
    SupportVentilationWithFan, UserTriggerAction,
};
use crate::home::state::UserControlled;

use super::action::{
    DeferHeatingUntilVentilationDone, Dehumidify, ExtendHeatingUntilSleeping, HomeAction,
    KeepUserOverride, NoHeatingDuringAutomaticTemperatureIncrease, NoHeatingDuringVentilation,
    RequestClosingWindow,
};
use super::goal::{HomeGoal, Room};

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(Room::LivingRoom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::LivingRoom).into(),
            KeepUserOverride::new(UserControlled::LivingRoomThermostat, Thermostat::LivingRoom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::LivingRoom).into(),
            ExtendHeatingUntilSleeping::LivingRoom.into(),
            DeferHeatingUntilVentilationDone::LivingRoom.into(),
            SupportVentilationWithFan::new(FanSpeed::LivingRoomCeilingFan).into(),
            UserTriggerAction::new(HomekitTarget::LivingRoomCeilingFanSpeed.into()).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bedroom),
        vec![
            UserTriggerAction::new(HomekitTarget::InfraredHeaterPower.into()).into(),
            UserTriggerAction::new(RemoteTarget::BedroomDoor.into()).into(),
            IrHeaterAutoTurnOff::new().into(),
            NoHeatingDuringVentilation::new(HeatingZone::Bedroom).into(),
            KeepUserOverride::new(UserControlled::BedroomThermostat, Thermostat::Bedroom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bedroom).into(),
            ExtendHeatingUntilSleeping::Bedroom.into(),
            DeferHeatingUntilVentilationDone::Bedroom.into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Kitchen),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Kitchen).into(),
            KeepUserOverride::new(UserControlled::KitchenThermostat, Thermostat::Kitchen.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Kitchen).into(),
            DeferHeatingUntilVentilationDone::Kitchen.into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::RoomOfRequirements),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::RoomOfRequirements).into(),
            KeepUserOverride::new(UserControlled::RoomOfRequirementsThermostat, Thermostat::RoomOfRequirements.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::RoomOfRequirements).into(),
        ]
    ),
    (
        HomeGoal::SmarterHeating(Room::Bathroom),
        vec![
            NoHeatingDuringVentilation::new(HeatingZone::Bathroom).into(),
            KeepUserOverride::new(UserControlled::BathroomThermostat, Thermostat::Bathroom.into()).into(),
            NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::Bathroom).into(),
        ]
    ),
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
            UserTriggerAction::new(HomekitTarget::DehumidifierPower.into()).into(),
            KeepUserOverride::new(UserControlled::Dehumidifier, PowerToggle::Dehumidifier.into()).into(),
            ReduceNoiseAtNight::new(t!(22:30 - 12:00)).into(),
            Dehumidify::new().into()
        ],
    ),
    (
        HomeGoal::TvControl,
        vec![
            UserTriggerAction::new(HomekitTarget::LivingRoomTvEnergySaving.into()).into(),
            FollowDefaultSetting::new(CommandTarget::SetEnergySaving {
                device: api::command::EnergySavingDevice::LivingRoomTv,
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
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::Bathroom,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: api::command::Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: api::command::Notification::WindowOpened,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::ControlFan {
                device: Fan::LivingRoomFan,
            }).into(),
        ]
    )
    ]
}
