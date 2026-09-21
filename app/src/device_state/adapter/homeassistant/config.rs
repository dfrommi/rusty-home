use crate::device_state::{
    AllergenIndex, FanActivity, LightLevel, ParticulateMatter, PowerAvailable, Presence, RelativeHumidity, Temperature,
};

use super::HaChannel;

pub fn default_ha_state_config() -> Vec<(&'static str, HaChannel)> {
    vec![
        //
        // AIR QUALITY
        //
        (
            "sensor.wohnzimmer_indoor_allergen_index",
            HaChannel::AllergenIndex(AllergenIndex::LivingRoom),
        ),
        (
            "sensor.wohnzimmer_pm2_5",
            HaChannel::ParticulateMatter(ParticulateMatter::LivingRoomPM25),
        ),
        //
        // TEMPERATURE
        //
        ("sensor.home_temperature", HaChannel::Temperature(Temperature::Outside)),
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
        ("light.hue_go", HaChannel::Powered(PowerAvailable::LivingRoomNotificationLight)),
        //
        // PRESENCE
        //
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
        //
        // FAN SPEED
        //
        (
            "fan.dehumidifier_34e8_fan",
            HaChannel::ComfeeDehumidifierFanSpeed(FanActivity::BedroomDehumidifier),
        ),
        (
            "humidifier.dehumidifier_34e8",
            HaChannel::ComfeeDehumidifierFanPowerState(FanActivity::BedroomDehumidifier),
        ),
        (
            "fan.wohnzimmer",
            HaChannel::PhilipsAirPurifierFan(FanActivity::LivingRoomAirPurifier),
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
