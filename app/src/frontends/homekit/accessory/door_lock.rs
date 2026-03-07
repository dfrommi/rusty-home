use crate::{
    frontends::homekit::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
    trigger::{Door, UserTrigger},
};

pub struct DoorLock {
    name: &'static str,
    door: Door,
    pending_reset: bool,
}

impl DoorLock {
    pub fn new(name: &'static str, door: Door) -> Self {
        // pending_reset starts true so the first home state event corrects any
        // persisted or default "open" state in homebridge-mqtt on startup
        Self {
            name,
            door,
            pending_reset: true,
        }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        // CurrentDoorState: 1 = closed, TargetDoorState: 1 = closed
        // ObstructionDetected is required by HAP but managed internally by homebridge-mqtt
        vec![
            self.target(HomekitCharacteristic::CurrentDoorState)
                .with_config(serde_json::json!(1)),
            self.target(HomekitCharacteristic::TargetDoorState)
                .with_config(serde_json::json!(1)),
        ]
    }

    pub fn export_state(&mut self, _state: &crate::home_state::HomeStateValue) -> Vec<HomekitEvent> {
        if self.pending_reset {
            self.pending_reset = false;
            // Reset to closed after the trigger was fired
            return vec![
                self.event(HomekitCharacteristic::CurrentDoorState, serde_json::json!(1)),
                self.event(HomekitCharacteristic::TargetDoorState, serde_json::json!(1)),
            ];
        }
        Vec::new()
    }

    pub fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<UserTrigger> {
        if trigger.target == self.target(HomekitCharacteristic::TargetDoorState) {
            let value = trigger
                .value
                .as_i64()
                .or_else(|| trigger.value.as_str().and_then(|s| s.parse().ok()));

            return match value {
                Some(0) => {
                    // 0 = open: activate buzzer
                    self.pending_reset = true;
                    Some(UserTrigger::OpenDoor {
                        door: self.door.clone(),
                    })
                }
                Some(1) => {
                    // Can't close mechanically, but push state back to closed
                    // so HomeKit doesn't stay stuck in "closing"
                    self.pending_reset = true;
                    None
                }
                _ => {
                    tracing::warn!(
                        "DoorLock {} received invalid TargetDoorState payload: {}",
                        self.name,
                        trigger.value
                    );
                    None
                }
            };
        }

        None
    }

    fn target(&self, characteristic: HomekitCharacteristic) -> HomekitTarget {
        HomekitTarget::new(self.name.to_string(), HomekitService::GarageDoorOpener, characteristic)
    }

    fn event(&self, characteristic: HomekitCharacteristic, value: serde_json::Value) -> HomekitEvent {
        HomekitEvent {
            target: self.target(characteristic),
            value,
        }
    }
}
