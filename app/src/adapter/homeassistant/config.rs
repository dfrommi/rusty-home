use crate::device_state::{FanActivity, LightLevel, PowerAvailable, Presence, RelativeHumidity, Temperature};

use super::HaChannel;

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
        (
            "sensor.wohnzimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::LivingRoomTado),
        ),
        (
            "sensor.schlafzimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::BedroomTado),
        ),
        (
            "sensor.arbeitszimmer_humidity",
            HaChannel::RelativeHumidity(RelativeHumidity::RoomOfRequirementsTado),
        ),
        //
        //POWERED STATE
        //
        ("light.hue_go", HaChannel::Powered(PowerAvailable::LivingRoomNotificationLight)),
        (
            "media_player.lg_webos_smart_tv",
            HaChannel::Powered(PowerAvailable::LivingRoomTv),
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
            "binary_sensor.presence_sensor_fp2_2b4e_presence_sensor_2",
            HaChannel::PresenceFromFP2(Presence::BedroomBed),
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
            "sensor.presence_sensor_fp2_2b4e_light_sensor_light_level",
            HaChannel::LightLevel(LightLevel::Bedroom),
        ),
    ]
}
