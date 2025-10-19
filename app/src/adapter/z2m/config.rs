use std::sync::Arc;

use tokio::sync::Mutex;

use super::Z2mChannel;
use super::Z2mCommandTarget;
use crate::adapter::z2m::incoming::ThermostatGroup;
use crate::core::unit::KiloWattHours;
use crate::home::Thermostat;
use crate::home::command::CommandTarget;
use crate::home::state::Opened;
use crate::home::state::*;
use crate::home::trigger::RemoteTarget;

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
        (
            CommandTarget::SetHeating {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
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
        (
            CommandTarget::SetThermostatAmbientTemperature {
                device: Thermostat::RoomOfRequirements,
            },
            Z2mCommandTarget::Thermostat("room_of_requirements/radiator_thermostat"),
        ),
    ]
}

pub fn default_z2m_state_config() -> Vec<(&'static str, Z2mChannel)> {
    let thermostat_group_living_room = Arc::new(Mutex::new(ThermostatGroup::new(
        "living_room/radiator_thermostat_big".to_string(),
        Thermostat::LivingRoomBig.heating_factor(),
        "living_room/radiator_thermostat_small".to_string(),
        Thermostat::LivingRoomSmall.heating_factor(),
    )));

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
                SetPoint::LivingRoomBig,
                HeatingDemand::LivingRoomBig,
                Opened::LivingRoomRadiatorThermostatBig,
                Temperature::Thermostat(Thermostat::LivingRoomBig),
                Some(thermostat_group_living_room.clone()),
            ),
        ),
        (
            "living_room/radiator_thermostat_small",
            Z2mChannel::Thermostat(
                SetPoint::LivingRoomSmall,
                HeatingDemand::LivingRoomSmall,
                Opened::LivingRoomRadiatorThermostatSmall,
                Temperature::Thermostat(Thermostat::LivingRoomSmall),
                Some(thermostat_group_living_room.clone()),
            ),
        ),
        (
            "kitchen/radiator_thermostat",
            Z2mChannel::Thermostat(
                SetPoint::Kitchen,
                HeatingDemand::Kitchen,
                Opened::KitchenRadiatorThermostat,
                Temperature::Thermostat(Thermostat::Kitchen),
                None,
            ),
        ),
        (
            "bedroom/radiator_thermostat",
            Z2mChannel::Thermostat(
                SetPoint::Bedroom,
                HeatingDemand::Bedroom,
                Opened::BedroomRadiatorThermostat,
                Temperature::Thermostat(Thermostat::Bedroom),
                None,
            ),
        ),
        (
            "room_of_requirements/radiator_thermostat",
            Z2mChannel::Thermostat(
                SetPoint::RoomOfRequirements,
                HeatingDemand::RoomOfRequirements,
                Opened::RoomOfRequirementsThermostat,
                Temperature::Thermostat(Thermostat::RoomOfRequirements),
                None,
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
        //
        // PRESENCE
        //
        (
            "bedroom/bed_dennis_occupancy",
            Z2mChannel::PresenceFromLeakSensor(Presence::BedDennis),
        ),
        (
            "bedroom/bed_sabine_occupancy",
            Z2mChannel::PresenceFromLeakSensor(Presence::BedSabine),
        ),
        //
        // BUTTON PRESS
        //
        ("bedroom/remote", Z2mChannel::RemoteClick(RemoteTarget::BedroomDoor)),
    ]
}
