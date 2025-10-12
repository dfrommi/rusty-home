use crate::core::time::{DateTime, FIXED_NOW};

use crate::{
    core::planner::{Action, ActionEvaluationResult},
    home::action::HomeAction,
};

use super::{infrastructure, runtime};

pub struct ActionState {
    pub is_fulfilled: bool,
}

pub fn get_state_at(iso: &str, action: impl Into<HomeAction>) -> ActionState {
    let fake_now = DateTime::from_iso(iso).unwrap();
    let action: HomeAction = action.into();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = &infrastructure().api();

        let result = action.evaluate(api).await.unwrap();

        let is_fulfilled = !matches!(result, ActionEvaluationResult::Skip);

        ActionState { is_fulfilled }
    }))
}

// #[test]
// fn user_trigger_not_started() {
//     let action = UserTriggerAction::new(UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower));
//
//     let result = get_state_at("2025-01-05T21:05:00.584641+01:00", action);
//
//     assert!(result.is_fulfilled);
// }

mod ext_id {
    use crate::adapter::homekit::HomekitCommandTarget;
    use crate::home::Thermostat;
    use crate::home::action::{
        AutoTurnOff, CoolDownWhenOccupied, Dehumidify, FollowDefaultSetting, FollowHeatingSchedule, InformWindowOpen,
        ProvideAmbientTemperature, ReduceNoiseAtNight, SupportVentilationWithFan, UserTriggerAction,
    };
    use crate::home::command::{
        CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle,
    };
    use crate::home::common::HeatingZone;
    use crate::home::state::HeatingMode;
    use crate::home::trigger::{RemoteTarget, UserTriggerTarget};

    #[test]
    fn dehumidify_ext_ids() {
        for action in Dehumidify::variants().iter().cloned() {
            let expected_variant = match action {
                Dehumidify::Dehumidifier => "dehumidifier",
            };
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "dehumidify");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn reduce_noise_at_night_ext_ids() {
        for action in ReduceNoiseAtNight::variants().iter().cloned() {
            let expected_variant = match action {
                ReduceNoiseAtNight::Dehumidifier => "dehumidifier",
            };
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "reduce_noise_at_night");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn auto_turn_off_ext_ids() {
        for action in AutoTurnOff::variants().iter().cloned() {
            let expected_variant = match action {
                AutoTurnOff::IrHeater => "ir_heater",
            };
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "auto_turn_off");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn provide_ambient_temperature_ext_ids() {
        for action in ProvideAmbientTemperature::variants() {
            let expected_variant = match &action {
                ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomBig) => "thermostat::living_room_big",
                ProvideAmbientTemperature::Thermostat(Thermostat::LivingRoomSmall) => "thermostat::living_room_small",
                ProvideAmbientTemperature::Thermostat(Thermostat::Bedroom) => "thermostat::bedroom",
                ProvideAmbientTemperature::Thermostat(Thermostat::Kitchen) => "thermostat::kitchen",
                ProvideAmbientTemperature::Thermostat(Thermostat::RoomOfRequirements) => {
                    "thermostat::room_of_requirements"
                }
                ProvideAmbientTemperature::Thermostat(Thermostat::Bathroom) => "thermostat::bathroom",
            };
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "provide_ambient_temperature");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn support_ventilation_with_fan_ext_ids() {
        for fan in Fan::variants() {
            let expected_variant = match fan {
                Fan::LivingRoomCeilingFan => "living_room_ceiling_fan",
                Fan::BedroomCeilingFan => "bedroom_ceiling_fan",
            };
            let action = SupportVentilationWithFan::new(fan.clone());
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "support_ventilation_with_fan");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn cool_down_when_occupied_ext_ids() {
        for fan in Fan::variants() {
            let expected_variant = match fan {
                Fan::LivingRoomCeilingFan => "living_room_ceiling_fan",
                Fan::BedroomCeilingFan => "bedroom_ceiling_fan",
            };
            let action = CoolDownWhenOccupied::from_fan_for_test(fan.clone());
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "cool_down_when_occupied");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn inform_window_open_ext_ids() {
        for action in InformWindowOpen::variants() {
            let expected_variant = match action {
                InformWindowOpen::PushNotification(NotificationRecipient::Dennis) => "push_notification::dennis",
                InformWindowOpen::PushNotification(NotificationRecipient::Sabine) => "push_notification::sabine",
                InformWindowOpen::NotificationLightLivingRoom => "notification_light_living_room",
            };
            let ext_id = action.ext_id();

            assert_eq!(ext_id.type_name(), "inform_window_open");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn follow_heating_schedule_ext_ids() {
        for zone in HeatingZone::variants() {
            for mode in HeatingMode::variants() {
                let expected_variant = match (&zone, &mode) {
                    (HeatingZone::LivingRoom, HeatingMode::EnergySaving) => "living_room::energy_saving",
                    (HeatingZone::LivingRoom, HeatingMode::Comfort) => "living_room::comfort",
                    (HeatingZone::LivingRoom, HeatingMode::Sleep) => "living_room::sleep",
                    (HeatingZone::LivingRoom, HeatingMode::Ventilation) => "living_room::ventilation",
                    (HeatingZone::LivingRoom, HeatingMode::PostVentilation) => "living_room::post_ventilation",
                    (HeatingZone::LivingRoom, HeatingMode::Away) => "living_room::away",
                    (HeatingZone::Bedroom, HeatingMode::EnergySaving) => "bedroom::energy_saving",
                    (HeatingZone::Bedroom, HeatingMode::Comfort) => "bedroom::comfort",
                    (HeatingZone::Bedroom, HeatingMode::Sleep) => "bedroom::sleep",
                    (HeatingZone::Bedroom, HeatingMode::Ventilation) => "bedroom::ventilation",
                    (HeatingZone::Bedroom, HeatingMode::PostVentilation) => "bedroom::post_ventilation",
                    (HeatingZone::Bedroom, HeatingMode::Away) => "bedroom::away",
                    (HeatingZone::Kitchen, HeatingMode::EnergySaving) => "kitchen::energy_saving",
                    (HeatingZone::Kitchen, HeatingMode::Comfort) => "kitchen::comfort",
                    (HeatingZone::Kitchen, HeatingMode::Sleep) => "kitchen::sleep",
                    (HeatingZone::Kitchen, HeatingMode::Ventilation) => "kitchen::ventilation",
                    (HeatingZone::Kitchen, HeatingMode::PostVentilation) => "kitchen::post_ventilation",
                    (HeatingZone::Kitchen, HeatingMode::Away) => "kitchen::away",
                    (HeatingZone::RoomOfRequirements, HeatingMode::EnergySaving) => {
                        "room_of_requirements::energy_saving"
                    }
                    (HeatingZone::RoomOfRequirements, HeatingMode::Comfort) => "room_of_requirements::comfort",
                    (HeatingZone::RoomOfRequirements, HeatingMode::Sleep) => "room_of_requirements::sleep",
                    (HeatingZone::RoomOfRequirements, HeatingMode::Ventilation) => "room_of_requirements::ventilation",
                    (HeatingZone::RoomOfRequirements, HeatingMode::PostVentilation) => {
                        "room_of_requirements::post_ventilation"
                    }
                    (HeatingZone::RoomOfRequirements, HeatingMode::Away) => "room_of_requirements::away",
                    (HeatingZone::Bathroom, HeatingMode::EnergySaving) => "bathroom::energy_saving",
                    (HeatingZone::Bathroom, HeatingMode::Comfort) => "bathroom::comfort",
                    (HeatingZone::Bathroom, HeatingMode::Sleep) => "bathroom::sleep",
                    (HeatingZone::Bathroom, HeatingMode::Ventilation) => "bathroom::ventilation",
                    (HeatingZone::Bathroom, HeatingMode::PostVentilation) => "bathroom::post_ventilation",
                    (HeatingZone::Bathroom, HeatingMode::Away) => "bathroom::away",
                };
                let action = FollowHeatingSchedule::new(zone.clone(), mode.clone());
                let ext_id = action.ext_id();

                assert_eq!(ext_id.type_name(), "follow_heating_schedule");
                assert_eq!(ext_id.variant_name(), expected_variant);
            }
        }
    }

    #[test]
    fn follow_default_setting_ext_ids() {
        let mut targets = Vec::new();

        for device in PowerToggle::variants() {
            targets.push(CommandTarget::SetPower { device: device.clone() });
        }

        for thermostat in Thermostat::variants() {
            targets.push(CommandTarget::SetHeating {
                device: thermostat.clone(),
            });
            targets.push(CommandTarget::SetThermostatAmbientTemperature {
                device: thermostat.clone(),
            });
        }

        for recipient in NotificationRecipient::variants() {
            for notification in Notification::variants() {
                targets.push(CommandTarget::PushNotify {
                    recipient: recipient.clone(),
                    notification: notification.clone(),
                });
            }
        }

        for device in EnergySavingDevice::variants() {
            targets.push(CommandTarget::SetEnergySaving { device: device.clone() });
        }

        for fan in Fan::variants() {
            targets.push(CommandTarget::ControlFan { device: fan.clone() });
        }

        for target in targets {
            let expected_variant = match &target {
                CommandTarget::SetPower {
                    device: PowerToggle::Dehumidifier,
                } => "set_power::dehumidifier",
                CommandTarget::SetPower {
                    device: PowerToggle::InfraredHeater,
                } => "set_power::infrared_heater",
                CommandTarget::SetPower {
                    device: PowerToggle::LivingRoomNotificationLight,
                } => "set_power::living_room_notification_light",
                CommandTarget::SetHeating {
                    device: Thermostat::LivingRoomBig,
                } => "set_heating::living_room_big",
                CommandTarget::SetHeating {
                    device: Thermostat::LivingRoomSmall,
                } => "set_heating::living_room_small",
                CommandTarget::SetHeating {
                    device: Thermostat::Bedroom,
                } => "set_heating::bedroom",
                CommandTarget::SetHeating {
                    device: Thermostat::Kitchen,
                } => "set_heating::kitchen",
                CommandTarget::SetHeating {
                    device: Thermostat::RoomOfRequirements,
                } => "set_heating::room_of_requirements",
                CommandTarget::SetHeating {
                    device: Thermostat::Bathroom,
                } => "set_heating::bathroom",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::LivingRoomBig,
                } => "set_thermostat_ambient_temperature::living_room_big",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::LivingRoomSmall,
                } => "set_thermostat_ambient_temperature::living_room_small",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::Bedroom,
                } => "set_thermostat_ambient_temperature::bedroom",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::Kitchen,
                } => "set_thermostat_ambient_temperature::kitchen",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::RoomOfRequirements,
                } => "set_thermostat_ambient_temperature::room_of_requirements",
                CommandTarget::SetThermostatAmbientTemperature {
                    device: Thermostat::Bathroom,
                } => "set_thermostat_ambient_temperature::bathroom",
                CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Dennis,
                    notification: Notification::WindowOpened,
                } => "push_notify::dennis::window_opened",
                CommandTarget::PushNotify {
                    recipient: NotificationRecipient::Sabine,
                    notification: Notification::WindowOpened,
                } => "push_notify::sabine::window_opened",
                CommandTarget::SetEnergySaving {
                    device: EnergySavingDevice::LivingRoomTv,
                } => "set_energy_saving::living_room_tv",
                CommandTarget::ControlFan {
                    device: Fan::LivingRoomCeilingFan,
                } => "control_fan::living_room_ceiling_fan",
                CommandTarget::ControlFan {
                    device: Fan::BedroomCeilingFan,
                } => "control_fan::bedroom_ceiling_fan",
            };

            let ext_id = FollowDefaultSetting::new(target).ext_id();

            assert_eq!(ext_id.type_name(), "follow_default_setting");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }

    #[test]
    fn user_trigger_action_ext_ids() {
        for target in UserTriggerTarget::variants() {
            let expected_variant = match &target {
                UserTriggerTarget::Remote(RemoteTarget::BedroomDoor) => "remote::bedroom_door",
                UserTriggerTarget::Homekit(HomekitCommandTarget::InfraredHeaterPower) => {
                    "homekit::infrared_heater_power"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::DehumidifierPower) => "homekit::dehumidifier_power",
                UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomTvEnergySaving) => {
                    "homekit::living_room_tv_energy_saving"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomCeilingFanSpeed) => {
                    "homekit::living_room_ceiling_fan_speed"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomCeilingFanSpeed) => {
                    "homekit::bedroom_ceiling_fan_speed"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::LivingRoomHeatingState) => {
                    "homekit::living_room_heating_state"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::BedroomHeatingState) => {
                    "homekit::bedroom_heating_state"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::KitchenHeatingState) => {
                    "homekit::kitchen_heating_state"
                }
                UserTriggerTarget::Homekit(HomekitCommandTarget::RoomOfRequirementsHeatingState) => {
                    "homekit::room_of_requirements_heating_state"
                }
            };

            let ext_id = UserTriggerAction::new(target.clone()).ext_id();

            assert_eq!(ext_id.type_name(), "user_trigger_action");
            assert_eq!(ext_id.variant_name(), expected_variant);
        }
    }
}
