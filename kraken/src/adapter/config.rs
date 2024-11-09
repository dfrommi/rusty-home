use std::collections::HashMap;

use api::command::{HeatingTargetState, SetHeating, SetPower, Thermostat};
use lazy_static::lazy_static;

use crate::adapter::CommandBackendService;
use api::state::{CurrentPowerUsage, HeatingDemand, Presence, SetPoint, TotalEnergyConsumption};
use api::{
    command::Command, command::PowerToggle, state::Opened, state::Powered, state::RelativeHumidity,
    state::Temperature,
};

use super::homeassistant::{HaChannel, HaClimateHvacMode};
use super::HaService;

pub fn ha_incoming_event_channel(entity_id: &str) -> Vec<HaChannel> {
    HA_ENTITIES.get(&entity_id).unwrap_or(&vec![]).clone()
}

pub fn to_backend_command(command: &Command) -> CommandBackendService {
    match command {
        Command::SetPower(SetPower {
            device: PowerToggle::Dehumidifier,
            power_on,
        }) => CommandBackendService::HomeAssistant(HaService::SwitchTurnOnOff {
            id: "switch.dehumidifier".to_owned(),
            power_on: *power_on,
        }),
        Command::SetPower(SetPower {
            device: PowerToggle::LivingRoomNotificationLight,
            power_on,
        }) => CommandBackendService::HomeAssistant(HaService::LightTurnOnOff {
            id: "light.hue_go".to_owned(),
            power_on: *power_on,
        }),
        Command::SetHeating(SetHeating {
            device: item,
            target_state,
        }) => {
            let thermostat = match item {
                Thermostat::LivingRoom => "climate.wohnzimmer",
                Thermostat::Bedroom => "climate.schlafzimmer",
                Thermostat::Kitchen => "climate.kuche",
                Thermostat::RoomOfRequirements => "climate.arbeitszimmer",
                Thermostat::Bathroom => "climate.bad",
            }
            .to_string();

            CommandBackendService::HomeAssistant(match target_state {
                HeatingTargetState::Auto => HaService::ClimateSetHvacMode {
                    id: thermostat,
                    mode: HaClimateHvacMode::Auto,
                },
                HeatingTargetState::Off => HaService::ClimateSetHvacMode {
                    id: thermostat,
                    mode: HaClimateHvacMode::Off,
                },
                HeatingTargetState::Heat { temperature, until } => HaService::TadoSetClimateTimer {
                    id: thermostat,
                    temperature: *temperature,
                    until: until.clone(),
                },
            })
        }
    }
}

lazy_static! {
    static ref HA_ENTITIES: HashMap<&'static str, Vec<HaChannel>> = {
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
            (
                "light.hue_go",
                HaChannel::Powered(Powered::LivingRoomNotificationLight)
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
                "sensor.irheater_energy_power",
                HaChannel::CurrentPowerUsage(CurrentPowerUsage::InfraredHeater)
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
                "sensor.irheater_energy_total",
                HaChannel::TotalEnergyConsumption(TotalEnergyConsumption::InfraredHeater)
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

            //
            // HEATING DEMAND
            //
            (
                "sensor.wohnzimmer_heating",
                HaChannel::HeatingDemand(HeatingDemand::LivingRoom)
            ),
            (
                "sensor.schlafzimmer_heating",
                HaChannel::HeatingDemand(HeatingDemand::Bedroom)
            ),
            (
                "sensor.arbeitsimmer_heating",
                HaChannel::HeatingDemand(HeatingDemand::RoomOfRequirements)
            ),
            (
                "sensor.kuche_heating",
                HaChannel::HeatingDemand(HeatingDemand::Kitchen)
            ),
            (
                "sensor.bad_heating",
                HaChannel::HeatingDemand(HeatingDemand::Bathroom)
            ),

            //
            // SET POINT
            //
            (
                "climate.wohnzimmer",
                HaChannel::SetPoint(SetPoint::LivingRoom)
            ),
            (
                "climate.schlafzimmer",
                HaChannel::SetPoint(SetPoint::Bedroom)
            ),
            (
                "climate.arbeitszimmer",
                HaChannel::SetPoint(SetPoint::RoomOfRequirements)
            ),
            (
                "climate.kuche",
                HaChannel::SetPoint(SetPoint::Kitchen)
            ),
            (
                "climate.bad",
                HaChannel::SetPoint(SetPoint::Bathroom)
            ),

            //
            // USER CONTROLLED
            //
            (
                "climate.arbeitszimmer",
                HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::RoomOfRequirementsThermostat)
            ),
            (
                "climate.bad",
                HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::BathroomThermostat)
            ),
            (
                "climate.kuche",
                HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::KitchenThermostat)
            ),
            (
                "climate.schlafzimmer",
                HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::BedroomThermostat)
            ),
            (
                "climate.wohnzimmer",
                HaChannel::ClimateAutoMode(api::state::ExternalAutoControl::LivingRoomThermostat)
            ),

            //
            // PRESENCE
            //
            (
                "binary_sensor.bedroom_bed_dennis_occupancy_water_leak",
                HaChannel::PresenceFromLeakSensor(Presence::BedDennis)
            ),
            (
                "binary_sensor.bedroom_bed_sabine_occupancy_water_leak",
                HaChannel::PresenceFromLeakSensor(Presence::BedSabine)
            ),
            (
                "binary_sensor.esphome_couch_couch_left",
                HaChannel::PresenceFromEsp(Presence::CouchLeft)
            ),
            (
                "binary_sensor.esphome_couch_couch_center",
                HaChannel::PresenceFromEsp(Presence::CouchCenter)
            ),
            (
                "binary_sensor.esphome_couch_couch_right",
                HaChannel::PresenceFromEsp(Presence::CouchRight)
            ),
            (
                "device_tracker.jarvis",
                HaChannel::PresenceFromDeviceTracker(Presence::AtHomeDennis)
            ),
            (
                "device_tracker.simi_2",
                HaChannel::PresenceFromDeviceTracker(Presence::AtHomeSabine)
            ),
        ];

        let mut m: HashMap<&str, Vec<HaChannel>> = HashMap::new();

        for (id, channel) in v {
            m.entry(id).or_default().push(channel);
        }

        m
    };
}
