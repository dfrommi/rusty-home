use super::Z2mChannel;
use super::Z2mCommandTarget;
use crate::automation::{LoadBalancedThermostat, Thermostat};
use crate::command::CommandTarget;
use crate::core::unit::KiloWattHours;
use crate::device_state::*;

pub fn default_z2m_command_config() -> Vec<(CommandTarget, Z2mCommandTarget)> {
    vec![
        (
            CommandTarget::SetHeating {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::Thermostat("bedroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::Thermostat("kitchen/radiator_thermostat"),
        ),
        // (
        //     CommandTarget::SetHeating {
        //         device: Thermostat::RoomOfRequirements,
        //     },
        //     Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
        // ),
        (
            CommandTarget::SetThermostatValveOpeningPosition {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat_sonoff"),
        ),
        (
            CommandTarget::SetHeating {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::Thermostat("bathroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomBig,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_big"),
        ),
        (
            CommandTarget::SetThermostatLoadMean {
                device: LoadBalancedThermostat::LivingRoomSmall,
            },
            Z2mCommandTarget::Thermostat("living_room/radiator_thermostat_small"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Bedroom,
            },
            Z2mCommandTarget::Thermostat("bedroom/radiator_thermostat"),
        ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Kitchen,
            },
            Z2mCommandTarget::Thermostat("kitchen/radiator_thermostat"),
        ),
        // (
        //     CommandTarget::SetThermostatAmbientTemperature {
        //         device: Thermostat::RoomOfRequirements,
        //     },
        //     Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
        // ),
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::Bathroom,
            },
            Z2mCommandTarget::Thermostat("bathroom/radiator_thermostat"),
        ),
    ]
}

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
            "bedroom/temp_sensor_bed",
            Z2mChannel::ClimateSensor(Temperature::Bedroom, RelativeHumidity::Bedroom),
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
            "bathroom/temp_sensor",
            Z2mChannel::ClimateSensor(Temperature::BathroomShower, RelativeHumidity::BathroomShower),
        ),
        (
            "bathroom/dehumidifier",
            Z2mChannel::ClimateSensor(Temperature::Dehumidifier, RelativeHumidity::Dehumidifier),
        ),
        (
            "kitchen/temp_sensor",
            Z2mChannel::ClimateSensor(Temperature::Kitchen, RelativeHumidity::Kitchen),
        ),
        (
            "kitchen/temp_sensor_outer_wall",
            Z2mChannel::ClimateSensor(Temperature::KitchenOuterWall, RelativeHumidity::KitchenOuterWall),
        ),
        //
        // THERMOSTATS
        //
        (
            "living_room/radiator_thermostat_big",
            Z2mChannel::Thermostat(
                Thermostat::LivingRoomBig,
                SetPoint::LivingRoomBig,
                HeatingDemand::LivingRoomBig,
                Opened::LivingRoomRadiatorThermostatBig,
            ),
        ),
        (
            "living_room/radiator_thermostat_small",
            Z2mChannel::Thermostat(
                Thermostat::LivingRoomSmall,
                SetPoint::LivingRoomSmall,
                HeatingDemand::LivingRoomSmall,
                Opened::LivingRoomRadiatorThermostatSmall,
            ),
        ),
        (
            "kitchen/radiator_thermostat",
            Z2mChannel::Thermostat(
                Thermostat::Kitchen,
                SetPoint::Kitchen,
                HeatingDemand::Kitchen,
                Opened::KitchenRadiatorThermostat,
            ),
        ),
        (
            "bedroom/radiator_thermostat",
            Z2mChannel::Thermostat(
                Thermostat::Bedroom,
                SetPoint::Bedroom,
                HeatingDemand::Bedroom,
                Opened::BedroomRadiatorThermostat,
            ),
        ),
        (
            "room_of_requirements/radiator_thermostat_sonoff",
            Z2mChannel::Thermostat(
                Thermostat::RoomOfRequirements,
                SetPoint::RoomOfRequirements,
                HeatingDemand::RoomOfRequirements,
                Opened::RoomOfRequirementsThermostat,
            ),
        ),
        (
            "bathroom/radiator_thermostat",
            Z2mChannel::Thermostat(
                Thermostat::Bathroom,
                SetPoint::Bathroom,
                HeatingDemand::Bathroom,
                Opened::BathroomThermostat,
            ),
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
