use crate::core::domain::RoomWithWindow;
use crate::home_state::{EnergySaving, FanActivity, HomeStateValue, Opened, RelativeHumidity, Temperature};
use crate::trigger::{Door, UserTrigger, UserTriggerTarget};
use crate::{
    command::PowerToggle,
    core::domain::{HeatingZone, Room},
    frontends::homekit::{
        HomekitEvent, HomekitTargetConfig,
        accessory::{
            climate_sensor::ClimateSensor, door_lock::DoorLock, energy_saving_switch::EnergySavingSwitch, fan::Fan,
            power_switch::PowerSwitch, thermostat::Thermostat, window_sensor::WindowSensor,
        },
    },
};

mod climate_sensor;
mod door_lock;
mod energy_saving_switch;
mod fan;
mod power_switch;
mod thermostat;
mod window_sensor;

#[derive(Debug)]
pub struct HomekitCommand {
    pub trigger: UserTrigger,
    pub policy: HomekitCommandPolicy,
}

impl HomekitCommand {
    pub fn debounced(trigger: UserTrigger) -> Self {
        let target = trigger.target();
        Self {
            trigger,
            policy: HomekitCommandPolicy::Debounced { target },
        }
    }

    #[allow(dead_code)]
    pub fn immediate(trigger: UserTrigger) -> Self {
        Self {
            trigger,
            policy: HomekitCommandPolicy::Immediate { cancel_pending: None },
        }
    }

    #[allow(dead_code)]
    pub fn immediate_canceling(trigger: UserTrigger, target: UserTriggerTarget) -> Self {
        Self {
            trigger,
            policy: HomekitCommandPolicy::Immediate {
                cancel_pending: Some(target),
            },
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum HomekitCommandPolicy {
    Immediate { cancel_pending: Option<UserTriggerTarget> },
    Debounced { target: UserTriggerTarget },
}

trait Accessory: Send {
    fn get_all_targets(&self) -> Vec<HomekitTargetConfig>;

    fn export_state(&mut self, _state: &HomeStateValue) -> Vec<HomekitEvent> {
        Vec::new()
    }

    fn process_trigger(&mut self, _trigger: &HomekitEvent) -> Option<HomekitCommand> {
        None
    }
}

pub struct HomekitRegistry {
    accessories: Vec<Box<dyn Accessory>>,
}

impl HomekitRegistry {
    fn new(accessories: Vec<Box<dyn Accessory>>) -> Self {
        Self { accessories }
    }

    pub fn get_device_config(&self) -> Vec<HomekitTargetConfig> {
        self.accessories
            .iter()
            .flat_map(|accessory| accessory.get_all_targets())
            .collect()
    }

    pub fn export_state(&mut self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        self.accessories
            .iter_mut()
            .flat_map(|accessory| accessory.export_state(state))
            .collect()
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        self.accessories
            .iter_mut()
            .find_map(|accessory| accessory.process_trigger(trigger))
    }
}

impl Default for HomekitRegistry {
    fn default() -> Self {
        Self::new(config())
    }
}

fn config() -> Vec<Box<dyn Accessory>> {
    vec![
        Box::new(ClimateSensor::new(
            "Klimasensor Wohnzimmer",
            Temperature::Room(Room::LivingRoom),
            RelativeHumidity::Room(Room::LivingRoom),
        )),
        Box::new(ClimateSensor::new(
            "Klimasensor Schlafzimmer",
            Temperature::Room(Room::Bedroom),
            RelativeHumidity::Room(Room::Bedroom),
        )),
        Box::new(ClimateSensor::new(
            "Klimasensor Arbeitszimmer",
            Temperature::Room(Room::RoomOfRequirements),
            RelativeHumidity::Room(Room::RoomOfRequirements),
        )),
        Box::new(ClimateSensor::new(
            "Klimasensor Küche",
            Temperature::Room(Room::Kitchen),
            RelativeHumidity::Room(Room::Kitchen),
        )),
        Box::new(ClimateSensor::new(
            "Klimasensor Bad",
            Temperature::Room(Room::Bathroom),
            RelativeHumidity::Room(Room::Bathroom),
        )),
        Box::new(WindowSensor::new(
            "Fenstersensor Wohnzimmer",
            Opened::Room(RoomWithWindow::LivingRoom),
        )),
        Box::new(WindowSensor::new(
            "Fenstersensor Schlafzimmer",
            Opened::Room(RoomWithWindow::Bedroom),
        )),
        Box::new(WindowSensor::new("Fenstersensor Küche", Opened::Room(RoomWithWindow::Kitchen))),
        Box::new(WindowSensor::new(
            "Fenstersensor Arbeitszimmer",
            Opened::Room(RoomWithWindow::RoomOfRequirements),
        )),
        Box::new(Thermostat::new("Thermostat Wohnzimmer", HeatingZone::LivingRoom)),
        Box::new(Thermostat::new("Thermostat Schlafzimmer", HeatingZone::Bedroom)),
        Box::new(Thermostat::new("Thermostat Arbeitszimmer", HeatingZone::RoomOfRequirements)),
        Box::new(Thermostat::new("Thermostat Küche", HeatingZone::Kitchen)),
        Box::new(Thermostat::new("Thermostat Bad", HeatingZone::Bathroom)),
        Box::new(DoorLock::new("Haustür", Door::Building)),
        Box::new(PowerSwitch::new("Luftentfeuchter", PowerToggle::Dehumidifier)),
        Box::new(PowerSwitch::new("Infrarotheizung", PowerToggle::InfraredHeater)),
        Box::new(EnergySavingSwitch::new(
            "Wohnzimmer TV Bildqualität",
            EnergySaving::LivingRoomTv,
        )),
        Box::new(Fan::new("Entfeuchter Bad", FanActivity::BedroomDehumidifier)),
        Box::new(Fan::new("Luftreiniger Wohnzimmer", FanActivity::LivingRoomAirPurifier)),
    ]
}
