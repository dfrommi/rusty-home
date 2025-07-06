use super::Z2mChannel;
use crate::home::state::OpenedRaw as Opened;
use crate::home::state::*;
use crate::home::trigger::RemoteTarget;

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
        (
            "bedroom/remote",
            Z2mChannel::RemoteClick(RemoteTarget::BedroomDoor),
        ),
    ]
}
