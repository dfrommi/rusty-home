use api::command::{
    CommandTarget, EnergySavingDevice, Notification, NotificationRecipient, Thermostat,
};

use api::state::{CurrentPowerUsage, HeatingDemand, Presence, SetPoint, TotalEnergyConsumption};
use api::trigger::RemoteTarget;
use api::{
    command::PowerToggle, state::Opened, state::Powered, state::RelativeHumidity,
    state::Temperature,
};

use crate::homeassistant::{HaChannel, HaServiceTarget};
use crate::tasmota::TasmotaChannel;
use crate::z2m::Z2mChannel;

pub fn default_ha_command_config() -> Vec<(CommandTarget, HaServiceTarget)> {
    vec![
        (
            PowerToggle::Dehumidifier.into(),
            HaServiceTarget::SwitchTurnOnOff("switch.dehumidifier"),
        ),
        (
            PowerToggle::InfraredHeater.into(),
            HaServiceTarget::SwitchTurnOnOff("switch.irheater"),
        ),
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
            "switch.dehumidifier",
            HaChannel::Powered(Powered::Dehumidifier),
        ),
        (
            "switch.irheater",
            HaChannel::Powered(Powered::InfraredHeater),
        ),
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
            "binary_sensor.bedroom_bed_dennis_occupancy_water_leak",
            HaChannel::PresenceFromLeakSensor(Presence::BedDennis),
        ),
        (
            "binary_sensor.bedroom_bed_sabine_occupancy_water_leak",
            HaChannel::PresenceFromLeakSensor(Presence::BedSabine),
        ),
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
        // BUTTON PRESS
        //
        (
            "sensor.bedroom_remote_click",
            HaChannel::ButtonPress(RemoteTarget::BedroomDoor),
        ),
    ]
}

pub fn default_z2m_state_config() -> Vec<(&'static str, Z2mChannel)> {
    vec![
        //
        // CLIMATE SENSORS
        //
        (
            "bathroom/temp_sensor",
            Z2mChannel::ClimateSensor(
                Temperature::BathroomShower,
                RelativeHumidity::BathroomShower,
            ),
        ),
        (
            "kitchen/temp_sensor",
            Z2mChannel::ClimateSensor(
                Temperature::KitchenOuterWall,
                RelativeHumidity::KitchenOuterWall,
            ),
        ),
        (
            "bedroom/outer_wall",
            Z2mChannel::ClimateSensor(
                Temperature::BedroomOuterWall,
                RelativeHumidity::BedroomOuterWall,
            ),
        ),
        (
            "bathroom/dehumidifier",
            Z2mChannel::ClimateSensor(Temperature::Dehumidifier, RelativeHumidity::Dehumidifier),
        ),
        //
        // WINDOW CONTACTS
        //
        (
            "bedroom/window",
            Z2mChannel::ContactSensor(Opened::BedroomWindow),
        ),
        (
            "living_room/balcony_door",
            Z2mChannel::ContactSensor(Opened::LivingRoomBalconyDoor),
        ),
        (
            "living_room/window_left",
            Z2mChannel::ContactSensor(Opened::LivingRoomWindowLeft),
        ),
        (
            "living_room/window_right",
            Z2mChannel::ContactSensor(Opened::LivingRoomWindowRight),
        ),
        (
            "living_room/window_side",
            Z2mChannel::ContactSensor(Opened::LivingRoomWindowSide),
        ),
        (
            "kitchen/window",
            Z2mChannel::ContactSensor(Opened::KitchenWindow),
        ),
        (
            "room_of_requirements/window_left",
            Z2mChannel::ContactSensor(Opened::RoomOfRequirementsWindowLeft),
        ),
        (
            "room_of_requirements/window_right",
            Z2mChannel::ContactSensor(Opened::RoomOfRequirementsWindowRight),
        ),
        (
            "room_of_requirements/window_side",
            Z2mChannel::ContactSensor(Opened::RoomOfRequirementsWindowSide),
        ),
        //
        // POWER PLUGS
        //
        (
            "kitchen/multiplug",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::KitchenMultiPlug,
                TotalEnergyConsumption::KitchenMultiPlug,
            ),
        ),
        (
            "living_room/couch_plug",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::CouchPlug,
                TotalEnergyConsumption::CouchPlug,
            ),
        ),
        (
            "room_of_requirements/makerspace",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::RoomOfRequirementsDesk,
                TotalEnergyConsumption::RoomOfRequirementsDesk,
            ),
        ),
    ]
}

pub fn default_tasmota_state_config() -> Vec<(&'static str, TasmotaChannel)> {
    vec![
        //
        // POWER PLUGS
        //
        (
            "appletv",
            TasmotaChannel::PowerPlug(CurrentPowerUsage::AppleTv, TotalEnergyConsumption::AppleTv),
        ),
        (
            "tv",
            TasmotaChannel::PowerPlug(CurrentPowerUsage::Tv, TotalEnergyConsumption::Tv),
        ),
        (
            "fridge",
            TasmotaChannel::PowerPlug(CurrentPowerUsage::Fridge, TotalEnergyConsumption::Fridge),
        ),
        (
            "dehumidifier",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::Dehumidifier,
                TotalEnergyConsumption::Dehumidifier,
            ),
        ),
        (
            "airpurifier",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::AirPurifier,
                TotalEnergyConsumption::AirPurifier,
            ),
        ),
        (
            "kettle",
            TasmotaChannel::PowerPlug(CurrentPowerUsage::Kettle, TotalEnergyConsumption::Kettle),
        ),
        (
            "washer",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::WashingMachine,
                TotalEnergyConsumption::WashingMachine,
            ),
        ),
        (
            "couchlight",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::CouchLight,
                TotalEnergyConsumption::CouchLight,
            ),
        ),
        (
            "dishwasher",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::Dishwasher,
                TotalEnergyConsumption::Dishwasher,
            ),
        ),
        (
            "nuc",
            TasmotaChannel::PowerPlug(CurrentPowerUsage::Nuc, TotalEnergyConsumption::Nuc),
        ),
        (
            "dslmodem",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::DslModem,
                TotalEnergyConsumption::DslModem,
            ),
        ),
        (
            "unifi-usg",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::InternetGateway,
                TotalEnergyConsumption::InternetGateway,
            ),
        ),
        (
            "unifi-switch",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::NetworkSwitch,
                TotalEnergyConsumption::NetworkSwitch,
            ),
        ),
        (
            "irheater",
            TasmotaChannel::PowerPlug(
                CurrentPowerUsage::InfraredHeater,
                TotalEnergyConsumption::InfraredHeater,
            ),
        ),
    ]
}
