use crate::{
    adapter::{
        homebridge::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
        homekit::HomekitCommand,
    },
    home::state::{HomeStateValue, OpenedArea},
};

pub struct WindowSensor {
    name: &'static str,
    opened_area: OpenedArea,
}

impl WindowSensor {
    pub fn new(name: &'static str, opened_area: OpenedArea) -> Self {
        Self { name, opened_area }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        vec![
            HomekitTarget::new(
                self.name.to_string(),
                HomekitService::ContactSensor,
                HomekitCharacteristic::ContactSensorState,
            )
            .into_config(),
        ]
    }

    pub fn export_state(&self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        match state {
            HomeStateValue::OpenedArea(area, is_open) if area == &self.opened_area => {
                // HomeKit reports 0 when the window is closed (contact detected) and 1 when it is open.
                let sensor_state = if *is_open { 1 } else { 0 };
                vec![HomekitEvent {
                    target: HomekitTarget::new(
                        self.name.to_string(),
                        HomekitService::ContactSensor,
                        HomekitCharacteristic::ContactSensorState,
                    ),
                    value: serde_json::json!(sensor_state),
                }]
            }
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&self, _trigger: &HomekitEvent) -> Option<HomekitCommand> {
        None
    }
}
