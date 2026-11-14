use super::{Accessory, HomekitCommand};
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
        // Correct any persisted or default "open" state in homebridge-mqtt on startup.
        Self {
            name,
            door,
            pending_reset: true,
        }
    }

    fn export_pending_reset(&mut self) -> Vec<HomekitEvent> {
        if self.pending_reset {
            self.pending_reset = false;
            return vec![
                self.event(HomekitCharacteristic::CurrentDoorState, serde_json::json!(1)),
                self.event(HomekitCharacteristic::TargetDoorState, serde_json::json!(1)),
            ];
        }

        Vec::new()
    }
}

impl Accessory for DoorLock {
    fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        // CurrentDoorState: 1 = closed, TargetDoorState: 1 = closed
        // ObstructionDetected is required by HAP but managed internally by homebridge-mqtt
        vec![
            self.target(HomekitCharacteristic::CurrentDoorState)
                .with_config(serde_json::json!(1)),
            self.target(HomekitCharacteristic::TargetDoorState)
                .with_config(serde_json::json!(1)),
        ]
    }

    fn export_state(&mut self, _state: &crate::home_state::HomeStateValue) -> Vec<HomekitEvent> {
        self.export_pending_reset()
    }

    fn export_pending_events(&mut self) -> Vec<HomekitEvent> {
        self.export_pending_reset()
    }

    fn process_trigger(&mut self, trigger: &HomekitEvent) -> Option<HomekitCommand> {
        if trigger.target == self.target(HomekitCharacteristic::TargetDoorState) {
            let value = trigger
                .value
                .as_i64()
                .or_else(|| trigger.value.as_str().and_then(|s| s.parse().ok()));

            return match value {
                Some(0) => {
                    // 0 = open: activate buzzer
                    self.pending_reset = true;
                    Some(HomekitCommand::immediate(UserTrigger::OpenDoor {
                        door: self.door.clone(),
                    }))
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
}

impl DoorLock {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buzzer_request_can_reset_homekit_state_without_a_home_state_change() {
        let mut door = DoorLock::new("Haustür", Door::Building);
        let request = HomekitEvent {
            target: door.target(HomekitCharacteristic::TargetDoorState),
            value: serde_json::json!(0),
        };

        let command = door
            .process_trigger(&request)
            .expect("open request triggers the buzzer");
        assert!(matches!(command.trigger, UserTrigger::OpenDoor { .. }));

        let reset_events = door.export_pending_events();

        assert_eq!(reset_events.len(), 2);
        assert!(reset_events.iter().any(|event| {
            event.target.characteristic == HomekitCharacteristic::CurrentDoorState
                && event.value == serde_json::json!(1)
        }));
        assert!(reset_events.iter().any(|event| {
            event.target.characteristic == HomekitCharacteristic::TargetDoorState && event.value == serde_json::json!(1)
        }));
        assert!(door.export_pending_events().is_empty());
    }
}
