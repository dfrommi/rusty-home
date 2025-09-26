use crate::adapter::homekit::HomekitCommandTarget;
use crate::home::command::{CommandTarget, Fan, NotificationRecipient, PowerToggle, Thermostat};
use crate::home::trigger::RemoteTarget;
use crate::t;

use crate::home::action::{
    FollowDefaultSetting, HeatingZone, InformWindowOpen, IrHeaterAutoTurnOff, ReduceNoiseAtNight,
    SupportVentilationWithFan, UserTriggerAction,
};
use crate::home::state::UserControlled;

use super::action::{
    DeferHeatingUntilVentilationDone, Dehumidify, ExtendHeatingUntilSleeping, HomeAction, KeepUserOverride,
    NoHeatingDuringAutomaticTemperatureIncrease, NoHeatingDuringVentilation, RequestClosingWindow,
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
        ]
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
        HomeGoal::SmarterHeating(Room::Bedroom),
        vec![
            UserTriggerAction::new(HomekitCommandTarget::InfraredHeaterPower.into()).into(),
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
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        vec![
            SupportVentilationWithFan::new(Fan::BedroomCeilingFan).into(),
            UserTriggerAction::new(HomekitCommandTarget::BedroomCeilingFanSpeed.into()).into(),
            //RoolDownWhenOccupied::Bedroom.into(),
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
            //KeepUserOverride::new(UserControlled::RoomOfRequirementsThermostat, Thermostat::RoomOfRequirements.into()).into(),
            //NoHeatingDuringAutomaticTemperatureIncrease::new(HeatingZone::RoomOfRequirements).into(),
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
            UserTriggerAction::new(HomekitCommandTarget::DehumidifierPower.into()).into(),
            KeepUserOverride::new(UserControlled::Dehumidifier, PowerToggle::Dehumidifier.into()).into(),
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
