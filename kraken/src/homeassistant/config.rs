use api::command::{
    CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, Thermostat,
};

use api::state::{FanActivity, HeatingDemand, Presence, SetPoint};
use api::{command::PowerToggle, state::Powered, state::RelativeHumidity, state::Temperature};

use crate::homeassistant::{HaChannel, HaServiceTarget};

pub fn default_ha_command_config() -> Vec<(CommandTarget, HaServiceTarget)> {
    vec![
        (
            PowerToggle::LivingRoomNotificationLight.into(),
            HaServiceTarget::LightTurnOnOff("light.hue_go"),
        ),
        (
            Thermostat::LivingRoom.into(),
            HaServiceTarget::ClimateControl("climate.wohnzimmer"),
        ),
        (
            Thermostat::Bedroom.into(),
            HaServiceTarget::ClimateControl("climate.schlafzimmer"),
        ),
        (
            Thermostat::RoomOfRequirements.into(),
            HaServiceTarget::ClimateControl("climate.arbeitszimmer"),
        ),
        (
            Thermostat::Kitchen.into(),
            HaServiceTarget::ClimateControl("climate.kuche"),
        ),
        (
            Thermostat::Bathroom.into(),
            HaServiceTarget::ClimateControl("climate.bad"),
        ),
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: Notification::WindowOpened,
            },
            HaServiceTarget::PushNotification("mobile_app_jarvis"),
        ),
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: Notification::WindowOpened,
            },
            HaServiceTarget::PushNotification("mobile_app_simi_2"),
        ),
        (
            CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            },
            HaServiceTarget::LgWebosSmartTv("media_player.lg_webos_smart_tv"),
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::LivingRoomCeilingFan,
            },
            HaServiceTarget::WindcalmFanSpeed("fan.ceiling_fan_living_room"),
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::BedroomCeilingFan,
            },
            HaServiceTarget::WindcalmFanSpeed("fan.ceiling_fan_bedroom"),
        ),
    ]
}

pub fn default_ha_state_config() -> Vec<(&'static str, HaChannel)> {
    vec![
        //
        // TEMPERATURE
        //
        (
            "sensor.wohnzimmer_temperature",
            HaChannel::Temperature(Temperature::LivingRoomDoor),
        ),
        (
            "sensor.arbeitszimmer_temperature",
            HaChannel::Temperature(Temperature::RoomOfRequirementsDoor),
        ),
        (
            "sensor.schlafzimmer_temperature",
            HaChannel::Temperature(Temperature::BedroomDoor),
        ),
        (
            "sensor.home_temperature",
            HaChannel::Temperature(Temperature::Outside),
        ),
        //
        // HUMIDITY
        //
        (
            "sensor.wohnzimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::LivingRoomDoor),
        ),
        (
            "sensor.arbeitszimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::RoomOfRequirementsDoor),
        ),
        (
            "sensor.schlafzimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::BedroomDoor),
        ),
        (
            "sensor.home_relative_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::Outside),
        ),
        //
        //POWERED STATE
        //
        (
            "light.hue_go",
            HaChannel::Powered(Powered::LivingRoomNotificationLight),
        ),
        (
            "media_player.lg_webos_smart_tv",
            HaChannel::Powered(Powered::LivingRoomTv),
        ),
        //
        // HEATING DEMAND
        //
        (
            "sensor.wohnzimmer_heating",
            HaChannel::HeatingDemand(HeatingDemand::LivingRoom),
        ),
        (
            "sensor.schlafzimmer_heating",
            HaChannel::HeatingDemand(HeatingDemand::Bedroom),
        ),
        (
            "sensor.arbeitszimmer_heating",
            HaChannel::HeatingDemand(HeatingDemand::RoomOfRequirements),
        ),
        (
            "sensor.kuche_heating",
            HaChannel::HeatingDemand(HeatingDemand::Kitchen),
        ),
        (
            "sensor.bad_heating",
            HaChannel::HeatingDemand(HeatingDemand::Bathroom),
        ),
        //
        // SET POINT
        //
        (
            "climate.wohnzimmer",
            HaChannel::SetPoint(SetPoint::LivingRoom),
        ),
        (
            "climate.schlafzimmer",
            HaChannel::SetPoint(SetPoint::Bedroom),
        ),
        (
            "climate.arbeitszimmer",
            HaChannel::SetPoint(SetPoint::RoomOfRequirements),
        ),
        ("climate.kuche", HaChannel::SetPoint(SetPoint::Kitchen)),
        ("climate.bad", HaChannel::SetPoint(SetPoint::Bathroom)),
        //
        // USER CONTROLLED
        //
        (
            "climate.arbeitszimmer",
            HaChannel::ClimateAutoMode(
                api::state::ExternalAutoControl::RoomOfRequirementsThermostat,
            ),
        ),
        (
            "climate.bad",
            HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::BathroomThermostat),
        ),
        (
            "climate.kuche",
            HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::KitchenThermostat),
        ),
        (
            "climate.schlafzimmer",
            HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::BedroomThermostat),
        ),
        (
            "climate.wohnzimmer",
            HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::LivingRoomThermostat),
        ),
        //
        // PRESENCE
        //
        (
            "binary_sensor.esphome_couch_couch_left",
            HaChannel::PresenceFromEsp(Presence::CouchLeft),
        ),
        (
            "binary_sensor.esphome_couch_couch_center",
            HaChannel::PresenceFromEsp(Presence::CouchCenter),
        ),
        (
            "binary_sensor.esphome_couch_couch_right",
            HaChannel::PresenceFromEsp(Presence::CouchRight),
        ),
        (
            "device_tracker.jarvis",
            HaChannel::PresenceFromDeviceTracker(Presence::AtHomeDennis),
        ),
        (
            "device_tracker.simi_2",
            HaChannel::PresenceFromDeviceTracker(Presence::AtHomeSabine),
        ),
        //
        // FAN SPEED
        //
        (
            "fan.ceiling_fan_living_room",
            HaChannel::WindcalmFanSpeed(FanActivity::LivingRoomCeilingFan),
        ),
        (
            "fan.ceiling_fan_bedroom",
            HaChannel::WindcalmFanSpeed(FanActivity::BedroomCeilingFan),
        ),
    ]
}
