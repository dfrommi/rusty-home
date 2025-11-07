use crate::home::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};

use crate::home::state::{
    FanActivity, HeatingDemand, LightLevel, Powered, Presence, RelativeHumidity, SetPoint, Temperature,
};

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
        ("sensor.home_temperature", HaChannel::Temperature(Temperature::Outside)),
        (
            "sensor.wohnzimmer_temperature",
            HaChannel::Temperature(Temperature::LivingRoomTado),
        ),
        (
            "sensor.schlafzimmer_temperature",
            HaChannel::Temperature(Temperature::BedroomTado),
        ),
        (
            "sensor.arbeitszimmer_temperature",
            HaChannel::Temperature(Temperature::RoomOfRequirementsTado),
        ),
        //
        // HUMIDITY
        //
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
        (
            "binary_sensor.presence_sensor_fp2_2222_presence_sensor_1",
            HaChannel::PresenceFromFP2(Presence::LivingRoomArea),
        ),
        (
            "binary_sensor.presence_sensor_fp2_2222_presence_sensor_2",
            HaChannel::PresenceFromFP2(Presence::LivingRoomCouch),
        ),
        (
            "binary_sensor.presence_sensor_fp2_d775_presence_sensor_1",
            HaChannel::PresenceFromFP2(Presence::KitchenArea),
        ),
        (
            "binary_sensor.presence_sensor_fp2_2ed8_presence_sensor_1",
            HaChannel::PresenceFromFP2(Presence::RoomOfRequirementsArea),
        ),
        (
            "binary_sensor.presence_sensor_fp2_2ed8_presence_sensor_3",
            HaChannel::PresenceFromFP2(Presence::RoomOfRequirementsDesk),
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
        //
        // LIGHT LEVEL
        //
        (
            "sensor.presence_sensor_fp2_2222_light_sensor_light_level",
            HaChannel::LightLevel(LightLevel::LivingRoom),
        ),
        (
            "sensor.presence_sensor_fp2_d775_light_sensor_light_level",
            HaChannel::LightLevel(LightLevel::Kitchen),
        ),
        (
            "sensor.presence_sensor_fp2_2ed8_light_sensor_light_level",
            HaChannel::LightLevel(LightLevel::RoomOfRequirements),
        ),
    ]
}
