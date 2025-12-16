use crate::home_state::{FanActivity, HomeStateValue, OpenedArea, RelativeHumidity, Temperature};
use crate::{
    adapter::homekit::{
        HomekitCommand, HomekitEvent, HomekitTargetConfig,
        accessory::{
            climate_sensor::ClimateSensor, fan::Fan, power_switch::PowerSwitch, thermostat::Thermostat,
            window_sensor::WindowSensor,
        },
    },
    automation::HeatingZone,
    command::PowerToggle,
};

mod climate_sensor;
mod fan;
mod power_switch;
mod thermostat;
mod window_sensor;

enum HomekitAccessory {
    ClimateSensor(ClimateSensor),
    Fan(Fan),
    PowerSwitch(PowerSwitch),
    Thermostat(Thermostat),
    WindowSensor(WindowSensor),
}

pub struct HomekitRegistry {
    accessories: Vec<HomekitAccessory>,
}

impl HomekitRegistry {
    fn new(accessories: Vec<HomekitAccessory>) -> Self {
        Self { accessories }
    }

    pub fn get_device_config(&self) -> Vec<HomekitTargetConfig> {
        self.accessories
            .iter()
            .flat_map(|accessory| match accessory {
                HomekitAccessory::ClimateSensor(sensor) => sensor.get_all_targets(),
                HomekitAccessory::Fan(fan) => fan.get_all_targets(),
                HomekitAccessory::Thermostat(sensor) => sensor.get_all_targets(),
                HomekitAccessory::WindowSensor(sensor) => sensor.get_all_targets(),
                HomekitAccessory::PowerSwitch(power_switch) => power_switch.get_all_targets(),
            })
            .collect()
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        self.accessories
            .iter_mut()
            .flat_map(|accessory| match accessory {
                HomekitAccessory::ClimateSensor(sensor) => sensor.export_state(state),
                HomekitAccessory::Fan(fan) => fan.export_state(state),
                HomekitAccessory::Thermostat(sensor) => sensor.export_state(state),
                HomekitAccessory::WindowSensor(sensor) => sensor.export_state(state),
                HomekitAccessory::PowerSwitch(power_switch) => power_switch.export_state(state),
            })
            .collect()
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        for accessory in &mut self.accessories {
            let command = match accessory {
                HomekitAccessory::ClimateSensor(sensor) => sensor.process_trigger(trigger),
                HomekitAccessory::Fan(fan) => fan.process_trigger(trigger),
                HomekitAccessory::Thermostat(sensor) => sensor.process_trigger(trigger),
                HomekitAccessory::WindowSensor(sensor) => sensor.process_trigger(trigger),
                HomekitAccessory::PowerSwitch(power_switch) => power_switch.process_trigger(trigger),
            };

            if command.is_some() {
                return command;
            }
        }

        None
    }
}

impl Default for HomekitRegistry {
    fn default() -> Self {
        Self::new(config())
    }
}

fn config() -> Vec<HomekitAccessory> {
    vec![
        HomekitAccessory::ClimateSensor(ClimateSensor::new(
            "Klimasensor Wohnzimmer",
            Temperature::LivingRoom,
            RelativeHumidity::LivingRoom,
        )),
        HomekitAccessory::ClimateSensor(ClimateSensor::new(
            "Klimasensor Schlafzimmer",
            Temperature::Bedroom,
            RelativeHumidity::Bedroom,
        )),
        HomekitAccessory::ClimateSensor(ClimateSensor::new(
            "Klimasensor Arbeitszimmer",
            Temperature::RoomOfRequirements,
            RelativeHumidity::RoomOfRequirements,
        )),
        HomekitAccessory::ClimateSensor(ClimateSensor::new(
            "Klimasensor Küche",
            Temperature::Kitchen,
            RelativeHumidity::Kitchen,
        )),
        HomekitAccessory::ClimateSensor(ClimateSensor::new(
            "Klimasensor Bad",
            Temperature::Bathroom,
            RelativeHumidity::Bathroom,
        )),
        HomekitAccessory::WindowSensor(WindowSensor::new(
            "Fenstersensor Wohnzimmer",
            OpenedArea::LivingRoomWindowOrDoor,
        )),
        HomekitAccessory::WindowSensor(WindowSensor::new("Fenstersensor Schlafzimmer", OpenedArea::BedroomWindow)),
        HomekitAccessory::WindowSensor(WindowSensor::new("Fenstersensor Küche", OpenedArea::KitchenWindow)),
        HomekitAccessory::WindowSensor(WindowSensor::new(
            "Fenstersensor Arbeitszimmer",
            OpenedArea::RoomOfRequirementsWindow,
        )),
        HomekitAccessory::Thermostat(Thermostat::new("Thermostat Wohnzimmer", HeatingZone::LivingRoom)),
        HomekitAccessory::Thermostat(Thermostat::new("Thermostat Schlafzimmer", HeatingZone::Bedroom)),
        HomekitAccessory::Thermostat(Thermostat::new("Thermostat Arbeitszimmer", HeatingZone::RoomOfRequirements)),
        HomekitAccessory::Thermostat(Thermostat::new("Thermostat Küche", HeatingZone::Kitchen)),
        HomekitAccessory::Thermostat(Thermostat::new("Thermostat Bad", HeatingZone::Bathroom)),
        HomekitAccessory::PowerSwitch(PowerSwitch::new("Luftentfeuchter", PowerToggle::Dehumidifier)),
        HomekitAccessory::PowerSwitch(PowerSwitch::new("Infrarotheizung", PowerToggle::InfraredHeater)),
        HomekitAccessory::Fan(Fan::new("Deckenventilator Wohnzimmer", FanActivity::LivingRoomCeilingFan)),
        HomekitAccessory::Fan(Fan::new("Deckenventilator Schlafzimmer", FanActivity::BedroomCeilingFan)),
    ]
}
