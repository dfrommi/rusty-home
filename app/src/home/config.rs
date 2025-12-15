use crate::adapter::homekit::HomekitCommandTarget;
use crate::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};
use crate::home::common::HeatingZone;
use crate::home::{LoadBalancedThermostat, Room, Thermostat};
use crate::trigger::RemoteTarget;

use super::action::{Dehumidify, HomeAction};
use super::goal::HomeGoal;
use crate::home::action::{
    AutoTurnOff, FollowDefaultSetting, FollowHeatingSchedule, InformWindowOpen, ProvideAmbientTemperature,
    ProvideLoadRoomMean, ReduceNoiseAtNight, SupportVentilationWithFan, UserTriggerAction,
};

#[rustfmt::skip]
pub fn default_config() -> Vec<(HomeGoal, Vec<HomeAction>)> {
    vec![
    (
        HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
        smarter_heating_actions(HeatingZone::LivingRoom)
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::Bedroom),
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
        HomeGoal::SmarterHeating(HeatingZone::Kitchen),
        smarter_heating_actions(HeatingZone::Kitchen)
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
        smarter_heating_actions(HeatingZone::RoomOfRequirements)
    ),
    (
        HomeGoal::SmarterHeating(HeatingZone::Bathroom),
        smarter_heating_actions(HeatingZone::Bathroom)
    ),
    (
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        vec![
            SupportVentilationWithFan::new(Fan::LivingRoomCeilingFan).into(),
            UserTriggerAction::new(HomekitCommandTarget::LivingRoomCeilingFanSpeed.into()).into(),
        ]
    ),
    (
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        vec![
            SupportVentilationWithFan::new(Fan::BedroomCeilingFan).into(),
            UserTriggerAction::new(HomekitCommandTarget::BedroomCeilingFanSpeed.into()).into(),
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
        HomeGoal::CoreControl,
        vec![
            ProvideLoadRoomMean::LivingRoom.into(),
            ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomBig).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomSmall).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::Bedroom).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::Kitchen).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::RoomOfRequirements).into(), 
            ProvideAmbientTemperature::Thermostat(Thermostat::Bathroom).into(), 
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
            // FollowDefaultSetting::new(CommandTarget::SetHeating {
            //     device: Thermostat::RoomOfRequirements,
            // }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::Kitchen,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetHeating {
                device: Thermostat::Bathroom,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomBig,
            }).into(),
            FollowDefaultSetting::new(CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomSmall,
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
        ]
    )
    ]
}

fn smarter_heating_actions(zone: HeatingZone) -> Vec<HomeAction> {
    vec![
        FollowHeatingSchedule::new(zone.clone()).into(),
        // UserTriggerAction::new(
        //     match zone {
        //         HeatingZone::LivingRoom => HomekitCommandTarget::LivingRoomHeatingState,
        //         HeatingZone::Bedroom => HomekitCommandTarget::BedroomHeatingState,
        //         HeatingZone::Kitchen => HomekitCommandTarget::KitchenHeatingState,
        //         HeatingZone::RoomOfRequirements => HomekitCommandTarget::RoomOfRequirementsHeatingState,
        //         HeatingZone::Bathroom => HomekitCommandTarget::BathroomHeatingState,
        //     }
        //     .into(),
        // )
        // .into(),
    ]
}
