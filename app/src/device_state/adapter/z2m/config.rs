use super::Z2mChannel;
use crate::automation::Thermostat;
use crate::core::unit::KiloWattHours;
use crate::device_state::*;

pub fn default_z2m_state_config() -> Vec<(&'static str, Z2mChannel)> {
    vec![
        //
        // CLIMATE SENSORS
        //
        (
            "living_room/temp_sensor_couch",
            Z2mChannel::ClimateSensor(Temperature::LivingRoom, RelativeHumidity::LivingRoom),
        ),
        (
            "living_room/temp_sensor_radiator_small",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::LivingRoomSmall),
                RelativeHumidity::Radiator(Thermostat::LivingRoomSmall),
            ),
        ),
        (
            "living_room/temp_sensor_radiator_big",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::LivingRoomBig),
                RelativeHumidity::Radiator(Thermostat::LivingRoomBig),
            ),
        ),
        (
            "bedroom/temp_sensor_bed",
            Z2mChannel::ClimateSensor(Temperature::Bedroom, RelativeHumidity::Bedroom),
        ),
        (
            "bedroom/temp_sensor_radiator",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::Bedroom),
                RelativeHumidity::Radiator(Thermostat::Bedroom),
            ),
        ),
        (
            "bedroom/outer_wall",
            Z2mChannel::ClimateSensor(Temperature::BedroomOuterWall, RelativeHumidity::BedroomOuterWall),
        ),
        (
            "room_of_requirements/temp_sensor_desk",
            Z2mChannel::ClimateSensor(Temperature::RoomOfRequirements, RelativeHumidity::RoomOfRequirements),
        ),
        (
            "room_of_requirements/temp_sensor_radiator",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::RoomOfRequirements),
                RelativeHumidity::Radiator(Thermostat::RoomOfRequirements),
            ),
        ),
        (
            "bathroom/temp_sensor",
            Z2mChannel::ClimateSensor(Temperature::BathroomShower, RelativeHumidity::BathroomShower),
        ),
        (
            "bathroom/dehumidifier",
            Z2mChannel::ClimateSensor(Temperature::Dehumidifier, RelativeHumidity::Dehumidifier),
        ),
        (
            "bathroom/temp_sensor_radiator",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::Bathroom),
                RelativeHumidity::Radiator(Thermostat::Bathroom),
            ),
        ),
        (
            "kitchen/temp_sensor",
            Z2mChannel::ClimateSensor(Temperature::Kitchen, RelativeHumidity::Kitchen),
        ),
        (
            "kitchen/temp_sensor_outer_wall",
            Z2mChannel::ClimateSensor(Temperature::KitchenOuterWall, RelativeHumidity::KitchenOuterWall),
        ),
        (
            "kitchen/temp_sensor_radiator",
            Z2mChannel::ClimateSensor(
                Temperature::Radiator(Thermostat::Kitchen),
                RelativeHumidity::Radiator(Thermostat::Kitchen),
            ),
        ),
        //
        // THERMOSTATS
        //
        (
            "living_room/radiator_thermostat_big_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::LivingRoomBig, HeatingDemand::LivingRoomBig),
        ),
        (
            "living_room/radiator_thermostat_small_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::LivingRoomSmall, HeatingDemand::LivingRoomSmall),
        ),
        (
            "kitchen/radiator_thermostat_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::Kitchen, HeatingDemand::Kitchen),
        ),
        (
            "bedroom/radiator_thermostat_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::Bedroom, HeatingDemand::Bedroom),
        ),
        (
            "room_of_requirements/radiator_thermostat_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::RoomOfRequirements, HeatingDemand::RoomOfRequirements),
        ),
        (
            "bathroom/radiator_thermostat_sonoff",
            Z2mChannel::SonoffThermostat(Thermostat::Bathroom, HeatingDemand::Bathroom),
        ),
        //
        // WINDOW CONTACTS
        //
        ("bedroom/window", Z2mChannel::ContactSensor(Opened::BedroomWindow)),
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
        ("kitchen/window", Z2mChannel::ContactSensor(Opened::KitchenWindow)),
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
                KiloWattHours(0.0),
            ),
        ),
        (
            "living_room/couch_plug",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::CouchPlug,
                TotalEnergyConsumption::CouchPlug,
                KiloWattHours(0.0),
            ),
        ),
        (
            "room_of_requirements/makerspace",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::RoomOfRequirementsDesk,
                TotalEnergyConsumption::RoomOfRequirementsDesk,
                KiloWattHours(0.0),
            ),
        ),
        (
            "room_of_requirements/desk_monitor",
            Z2mChannel::PowerPlug(
                CurrentPowerUsage::RoomOfRequirementsMonitor,
                TotalEnergyConsumption::RoomOfRequirementsMonitor,
                KiloWattHours(0.0),
            ),
        ),
    ]
}
