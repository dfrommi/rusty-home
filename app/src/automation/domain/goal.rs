use crate::automation::domain::action::{
    AutoTurnOff, BlockAutomation, Dehumidify, FollowDefaultSetting, FollowTargetHeatingDemand, HomeAction,
    InformWindowOpen, SupportWithFan, UserTriggerAction,
};
use crate::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};
use crate::core::domain::{HeatingZone, Room};
use crate::home_state::FanActivity;
use crate::home_state::StateSnapshot;
use crate::trigger::{Door, OnOffDevice, RemoteTriggerTarget, UserTriggerTarget};

//Refactor to variants() and is_active() method
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum HomeGoal {
    PreventNoise,
    PreventMould,
    StayInformed,
    #[display("SmarterHeating[{}]", _0)]
    SmarterHeating(HeatingZone),
    #[display("BetterRoomClimate[{}]", _0)]
    BetterRoomClimate(Room),
    TvControl,
    ReactToUserRequests,
    ResetToDefaltSettings,
}

//TODO select goals based on current state
pub fn get_active_goals(_snapshot: StateSnapshot) -> Vec<HomeGoal> {
    //Prioritized high to low
    vec![
        HomeGoal::PreventNoise,
        HomeGoal::SmarterHeating(HeatingZone::LivingRoom),
        HomeGoal::SmarterHeating(HeatingZone::Bedroom),
        HomeGoal::SmarterHeating(HeatingZone::Kitchen),
        HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements),
        HomeGoal::SmarterHeating(HeatingZone::Bathroom),
        HomeGoal::BetterRoomClimate(Room::LivingRoom),
        HomeGoal::BetterRoomClimate(Room::Bedroom),
        HomeGoal::StayInformed,
        HomeGoal::PreventMould,
        HomeGoal::TvControl,
        HomeGoal::ReactToUserRequests,
        HomeGoal::ResetToDefaltSettings,
    ]
}

impl HomeGoal {
    pub fn rules(&self) -> Vec<HomeAction> {
        match self {
            HomeGoal::PreventNoise => vec![
                BlockAutomation::BathroomDehumidifier.into(),
                BlockAutomation::BedroomDehumidifier.into(),
                BlockAutomation::BedroomCeilingFan.into(),
                UserTriggerAction::new(RemoteTriggerTarget::BedroomDoorRemote.into()).into(),
            ],
            HomeGoal::SmarterHeating(HeatingZone::LivingRoom) => {
                vec![
                    FollowTargetHeatingDemand::new(HeatingZone::LivingRoom).into(),
                    //SupportWithFan::LivingRoomHeating.into()
                ]
            }
            HomeGoal::SmarterHeating(HeatingZone::Bedroom) => vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::InfraredHeater)).into(),
                AutoTurnOff::IrHeater.into(),
                FollowTargetHeatingDemand::new(HeatingZone::Bedroom).into(),
                SupportWithFan::BedroomVentilation.into(),
                SupportWithFan::BedroomHeating.into(),
            ],
            HomeGoal::SmarterHeating(HeatingZone::Kitchen) => {
                vec![FollowTargetHeatingDemand::new(HeatingZone::Kitchen).into()]
            }
            HomeGoal::SmarterHeating(HeatingZone::RoomOfRequirements) => {
                vec![FollowTargetHeatingDemand::new(HeatingZone::RoomOfRequirements).into()]
            }
            HomeGoal::SmarterHeating(HeatingZone::Bathroom) => {
                vec![FollowTargetHeatingDemand::new(HeatingZone::Bathroom).into()]
            }
            HomeGoal::BetterRoomClimate(Room::LivingRoom) => vec![
                SupportWithFan::LivingRoomVentilation.into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::LivingRoomCeilingFan)).into(),
            ],
            HomeGoal::BetterRoomClimate(Room::Bedroom) => vec![
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::BedroomCeilingFan)).into(),
                UserTriggerAction::new(UserTriggerTarget::FanSpeed(FanActivity::BedroomDehumidifier)).into(),
                SupportWithFan::BedroomVentilation.into(),
                Dehumidify::Bedroom.into(),
                SupportWithFan::BedroomDehumidification.into(),
            ],
            HomeGoal::BetterRoomClimate(_) => vec![],
            HomeGoal::StayInformed => vec![
                InformWindowOpen::NotificationLightLivingRoom.into(),
                InformWindowOpen::PushNotification(NotificationRecipient::Dennis).into(),
                InformWindowOpen::PushNotification(NotificationRecipient::Sabine).into(),
            ],
            HomeGoal::PreventMould => vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::Dehumidifier)).into(),
                Dehumidify::Bathroom.into(),
            ],
            HomeGoal::TvControl => vec![
                UserTriggerAction::new(UserTriggerTarget::DevicePower(OnOffDevice::LivingRoomTvEnergySaving)).into(),
                FollowDefaultSetting::new(CommandTarget::SetEnergySaving {
                    device: EnergySavingDevice::LivingRoomTv,
                })
                .into(),
            ],
            HomeGoal::ReactToUserRequests => {
                vec![UserTriggerAction::new(UserTriggerTarget::OpenDoor(Door::Building)).into()]
            }
            HomeGoal::ResetToDefaltSettings => vec![
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::Dehumidifier,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::InfraredHeater,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::SetPower {
                    device: PowerToggle::LivingRoomNotificationLight,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Dennis,
                    notification: Notification::WindowOpened,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Sabine,
                    notification: Notification::WindowOpened,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::LivingRoomCeilingFan,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::BedroomCeilingFan,
                })
                .into(),
                FollowDefaultSetting::new(CommandTarget::ControlFan {
                    device: Fan::BedroomDehumidifier,
                })
                .into(),
            ],
        }
    }
}
