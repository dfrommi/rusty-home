use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::adapter::homeassistant::HaChannel;
use api::state::{CurrentPowerUsage, TotalEnergyConsumption};
use api::{
    command::Command, command::PowerToggle, state::Opened, state::Powered, state::RelativeHumidity,
    state::Temperature,
};

use super::HaCommandEntity;
use super::HomeAssistantService;

pub fn ha_incoming_event_channel(entity_id: &str) -> Option<HaChannel> {
    HA_ENTITIES.get(&entity_id).cloned()
}

pub fn ha_command_entity(command: &Command) -> Option<HaCommandEntity> {
    match command {
        Command::SetPower {
            item: PowerToggle::Dehumidifier,
            power_on,
        } => Some(HaCommandEntity {
            id: "switch.mytest",
            service: turn_on_off(*power_on),
        }),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

fn turn_on_off(power_on: bool) -> HomeAssistantService {
    if power_on {
        HomeAssistantService::SwitchTurnOn
    } else {
        HomeAssistantService::SwitchTurnOff
    }
}

lazy_static! {
    static ref HA_ENTITIES: HashMap<&'static str, HaChannel> = {
        let v = [
            //
            // TEMPERATURE
            //
            (
                "sensor.bathroom_temp_sensor_temperature",
                HaChannel::Temperature(Temperature::BathroomShower)
            ),
            (
                "sensor.kitchen_temp_sensor_temperature",
                HaChannel::Temperature(Temperature::KitchenOuterWall),
            ),
            (
                "sensor.bedroom_outer_wall_temperature",
                HaChannel::Temperature(Temperature::BedroomOuterWall)
            ),
            (
                "sensor.wohnzimmer_temperature",
                HaChannel::Temperature(Temperature::LivingRoomDoor)
            ),
            (
                "sensor.arbeitszimmer_temperature",
                HaChannel::Temperature(Temperature::RoomOfRequirementsDoor)
            ),
            (
                "sensor.schlafzimmer_temperature",
                HaChannel::Temperature(Temperature::BedroomDoor)
            ),
            (
                "sensor.home_temperature",
                HaChannel::Temperature(Temperature::Outside)
            ),

            //
            // HUMIDITY
            //
            (
                "sensor.bathroom_temp_sensor_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::BathroomShower)
            ),
            (
                "sensor.kitchen_temp_sensor_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::KitchenOuterWall)
            ),
            (
                "sensor.bedroom_outer_wall_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::BedroomOuterWall)
            ),
            (
                "sensor.wohnzimmer_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::LivingRoomDoor)
            ),
            (
                "sensor.arbeitszimmer_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::RoomOfRequirementsDoor)
            ),
            (
                "sensor.schlafzimmer_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::BedroomDoor)
            ),
            (
                "sensor.home_relative_humidity",
                HaChannel::RelativeHumidity(RelativeHumidity::Outside)
            ),


            //
            // WINDOW CONTACTS
            //
            (
                "binary_sensor.bedroom_window_contact",
                HaChannel::Opened(Opened::BedroomWindow)
            ),
            (
                "binary_sensor.kitchen_window_contact" ,
                HaChannel::Opened(Opened::KitchenWindow)
            ),
            (
                "binary_sensor.living_room_balcony_door_contact" ,
                HaChannel::Opened(Opened::LivingRoomBalconyDoor)
            ),
            (
                "binary_sensor.living_room_window_side_contact" ,
                HaChannel::Opened(Opened::LivingRoomWindowSide)
            ),
            (
                "binary_sensor.living_room_window_left_contact" ,
                HaChannel::Opened(Opened::LivingRoomWindowLeft)
            ),
            (
                "binary_sensor.living_room_window_right_contact" ,
                HaChannel::Opened(Opened::LivingRoomWindowRight)
            ),
            (
                "binary_sensor.room_of_requirements_window_side_contact",
                HaChannel::Opened(Opened::RoomOfRequirementsWindowSide)
            ),
            (
                "binary_sensor.room_of_requirements_window_left_contact",
                HaChannel::Opened(Opened::RoomOfRequirementsWindowLeft)
            ),
            (
                "binary_sensor.room_of_requirements_window_right_contact",
                HaChannel::Opened(Opened::RoomOfRequirementsWindowRight)
            ),

            //
            //POWERED STATE
            //
            (
                "switch.dehumidifier",
                HaChannel::Powered(Powered::Dehumidifier)
            ),

            //
            //POWER CONSUMPTION
            //
            (
                "sensor.fridge_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Fridge)
            ),
            (
                "sensor.dehumidifier_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Dehumidifier)
            ),
            (
                "sensor.appletv_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::AppleTv)
            ),
            (
                "sensor.tv_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Tv)
            ),
            (
                "sensor.airpurifier_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::AirPurifier)
            ),
            (
                "sensor.couchlight_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::CouchLight)
            ),
            (
                "sensor.dishwasher_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Dishwasher)
            ),
            (
                "sensor.kettle_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Kettle)
            ),
            (
                "sensor.washer_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::WashingMachine)
            ),
            (
                "sensor.nuc_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::Nuc)
            ),
            (
                "sensor.dslmodem_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::DslModem)
            ),
            (
                "sensor.unifi_usg_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::InternetGateway)
            ),
            (
                "sensor.unifi_switch_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::NetworkSwitch)
            ),
            (
                "sensor.kitchen_multiplug_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::KitchenMultiPlug)
            ),
            (
                "sensor.living_room_couch_plug_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::CouchPlug)
            ),
            (
                "sensor.room_of_requirements_makerspace_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::RoomOfRequirementsDesk)
            ),


            //
            //ENERGY USAGE
            //
            (
                "sensor.fridge_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Fridge)
            ),
            (
                "sensor.dehumidifier_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Dehumidifier)
            ),
            (
                "sensor.appletv_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::AppleTv)
            ),
            (
                "sensor.tv_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Tv)
            ),
            (
                "sensor.airpurifier_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::AirPurifier)
            ),
            (
                "sensor.couchlight_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::CouchLight)
            ),
            (
                "sensor.dishwasher_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Dishwasher)
            ),
            (
                "sensor.kettle_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Kettle)
            ),
            (
                "sensor.washer_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::WashingMachine)
            ),
            (
                "sensor.nuc_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::Nuc)
            ),
            (
                "sensor.dslmodem_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::DslModem)
            ),
            (
                "sensor.unifi_usg_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::InternetGateway)
            ),
            (
                "sensor.unifi_switch_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::NetworkSwitch)
            ),
            (
                "sensor.kitchen_multiplug_energy",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::KitchenMultiPlug)
            ),
            (
                "sensor.living_room_couch_plug_energy",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::CouchPlug)
            ),
            (
                "sensor.room_of_requirements_makerspace_energy",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::RoomOfRequirementsDesk)
            ),
        ];

        let mut m = HashMap::new();

        for (id, channel) in v {
            m.insert(id, channel);
        }

        m
    };
}
