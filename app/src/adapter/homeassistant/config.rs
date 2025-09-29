use crate::home::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};

use crate::home::state::{FanActivity, HeatingDemand, Powered, Presence, RelativeHumidity, SetPoint, Temperature};

use super::{HaChannel, HaServiceTarget};

pub fn default_ha_command_config() -> Vec<(CommandTarget, HaServiceTarget)> {
    vec![
        (
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
            },
            HaServiceTarget::LightTurnOnOff("light.hue_go"),
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
        ("sensor.home_temperature", HaChannel::Temperature(Temperature::Outside)),
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
        ("light.hue_go", HaChannel::Powered(Powered::LivingRoomNotificationLight)),
        ("media_player.lg_webos_smart_tv", HaChannel::Powered(Powered::LivingRoomTv)),
        //
        // HEATING DEMAND
        //
        ("sensor.bad_heating", HaChannel::HeatingDemand(HeatingDemand::Bathroom)),
        //
        // SET POINT
        //
        ("climate.bad", HaChannel::SetPoint(SetPoint::Bathroom)),
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
